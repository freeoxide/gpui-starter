use std::path::Path;

use gpui::App;
#[cfg(not(target_family = "wasm"))]
use gpui::BorrowAppContext as _;
use gpui::Global;
use serde::{Deserialize, Serialize};

// Caps keep reports (and the upload payload) bounded; scrubbing keeps the
// user's home path out of anything that leaves the machine.
const MAX_PANIC_MESSAGE_CHARS: usize = 2048;
const MAX_BACKTRACE_CHARS: usize = 8192;
const MAX_RECENT_ERRORS: usize = 20;
const MAX_RECENT_ERROR_CHARS: usize = 512;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CrashReport {
    pub id: String,
    pub panic_message: String,
    pub backtrace: String,
    pub app_version: String,
    pub os: String,
    pub arch: String,
    pub timestamp: String,
    pub render_path: bool,
    pub recent_errors: Vec<String>,
}

impl CrashReport {
    pub fn new(
        panic_message: String,
        backtrace: String,
        render_path: bool,
        recent_errors: Vec<String>,
    ) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            panic_message: scrub(&panic_message, MAX_PANIC_MESSAGE_CHARS),
            backtrace: scrub(&backtrace, MAX_BACKTRACE_CHARS),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            render_path,
            recent_errors: recent_errors
                .into_iter()
                .take(MAX_RECENT_ERRORS)
                .map(|err| scrub(&err, MAX_RECENT_ERROR_CHARS))
                .collect(),
        }
    }
}

fn scrub(text: &str, max_chars: usize) -> String {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_default();
    let scrubbed = if home.is_empty() {
        text.to_string()
    } else {
        text.replace(&home, "~")
    };
    scrubbed.chars().take(max_chars).collect()
}

#[derive(Clone, Debug, Default)]
pub struct CrashReportSnapshot {
    pub pending_count: usize,
    pub last_crash_timestamp: Option<String>,
    pub upload_endpoint: String,
    pub last_upload_error: Option<String>,
}

impl Global for CrashReportSnapshot {}

pub fn initialize(cx: &mut App) {
    let endpoint = option_env!("GPUI_CRASH_REPORT_URL")
        .unwrap_or("")
        .to_string();

    let snapshot = CrashReportSnapshot {
        upload_endpoint: endpoint,
        ..CrashReportSnapshot::default()
    };

    cx.set_global(snapshot.clone());

    tracing::info!(
        target: "gpui_starter::crash_report",
        endpoint = %snapshot.upload_endpoint,
        "crash report service initialized"
    );
}

pub fn snapshot(cx: &App) -> CrashReportSnapshot {
    cx.try_global::<CrashReportSnapshot>()
        .cloned()
        .unwrap_or_default()
}

/// Intentionally synchronous `std::fs`: the panic hook has no async runtime.
pub fn write_crash_report(report: &CrashReport, data_dir: &Path) -> std::io::Result<()> {
    let reports_dir = data_dir.join("crash_reports");
    std::fs::create_dir_all(&reports_dir)?;

    let file_path = reports_dir.join(format!("{}.json", report.id));
    let json = serde_json::to_string_pretty(report)?;
    std::fs::write(&file_path, json)?;

    tracing::info!(
        target: "gpui_starter::crash_report",
        path = %file_path.display(),
        "crash report written to disk"
    );

    Ok(())
}

/// Non-JSON files and malformed entries are silently skipped.
pub fn detect_pending_reports(data_dir: &Path) -> Vec<CrashReport> {
    let reports_dir = data_dir.join("crash_reports");
    if !reports_dir.exists() {
        return Vec::new();
    }

    let Ok(entries) = std::fs::read_dir(&reports_dir) else {
        return Vec::new();
    };

    let mut reports = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let Ok(contents) = std::fs::read_to_string(&path) else {
            continue;
        };
        match serde_json::from_str::<CrashReport>(&contents) {
            Ok(report) => reports.push(report),
            Err(err) => {
                tracing::warn!(
                    target: "gpui_starter::crash_report",
                    path = %path.display(),
                    error = %err,
                    "skipping malformed crash report file"
                );
            }
        }
    }

    reports.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
    reports
}

/// No-op without an endpoint or backend. Wasm has no report producer (the
/// native panic hook owns that), so it degrades to a debug log.
#[cfg(not(target_family = "wasm"))]
pub fn upload_pending_reports(cx: &mut App) {
    let snap = snapshot(cx);
    if snap.upload_endpoint.is_empty() {
        tracing::debug!(
            target: "gpui_starter::crash_report",
            "no upload endpoint configured, skipping pending report upload"
        );
        return;
    }

    let Some(runtime) = cx.try_global::<crate::services::tokio_runtime::TokioRuntimeGlobal>()
    else {
        tracing::warn!(
            target: "gpui_starter::crash_report",
            "tokio runtime not available for crash report upload"
        );
        return;
    };

    let Some(storage_runtime) = cx.try_global::<crate::storage::StorageRuntime>() else {
        tracing::warn!(
            target: "gpui_starter::crash_report",
            "storage runtime not available for crash report upload"
        );
        return;
    };

    let endpoint = snap.upload_endpoint.clone();
    let http_client = runtime.0.http_client.clone();
    let backend = storage_runtime.backend.clone();

    cx.spawn(async move |cx| {
        let reports = {
            let backend = backend.clone();
            cx.background_executor()
                .spawn(async move { backend.load_pending_crash_reports(50).await })
                .await
        };

        let reports = match reports {
            Ok(r) => r,
            Err(err) => {
                tracing::warn!(
                    target: "gpui_starter::crash_report",
                    error = %err,
                    "failed to load pending crash reports"
                );
                return;
            }
        };

        if reports.is_empty() {
            return;
        }

        tracing::info!(
            target: "gpui_starter::crash_report",
            count = reports.len(),
            "uploading pending crash reports"
        );

        for report in &reports {
            let body = match serde_json::to_string(report) {
                Ok(b) => b,
                Err(err) => {
                    tracing::warn!(
                        target: "gpui_starter::crash_report",
                        id = %report.id,
                        error = %err,
                        "failed to serialize crash report"
                    );
                    continue;
                }
            };

            let result = http_client
                .post(&endpoint)
                .header("Content-Type", "application/json")
                .body(body)
                .send()
                .await;

            match result {
                Ok(resp) if resp.status().is_success() => {
                    let uploaded_at = chrono::Utc::now().to_rfc3339();
                    let backend = backend.clone();
                    let id = report.id.clone();
                    let mark_result = cx
                        .background_executor()
                        .spawn(async move {
                            backend.mark_crash_report_uploaded(&id, &uploaded_at).await
                        })
                        .await;
                    if let Err(err) = mark_result {
                        tracing::warn!(
                            target: "gpui_starter::crash_report",
                            id = %report.id,
                            error = %err,
                            "failed to mark crash report as uploaded"
                        );
                    }
                }
                Ok(resp) => {
                    tracing::warn!(
                        target: "gpui_starter::crash_report",
                        id = %report.id,
                        status = %resp.status(),
                        "crash report upload returned non-success status"
                    );
                }
                Err(err) => {
                    tracing::warn!(
                        target: "gpui_starter::crash_report",
                        id = %report.id,
                        error = %err,
                        "crash report upload failed"
                    );
                }
            }
        }

        // One 1-row query yields both the pending count and newest timestamp.
        let backend = backend.clone();
        let latest = cx
            .background_executor()
            .spawn(async move { backend.load_pending_crash_reports(1).await })
            .await;
        let (count, timestamp) = match latest {
            Ok(r) => (r.len(), r.first().map(|r| r.timestamp.clone())),
            Err(_) => (0, None),
        };
        let _ = cx.update(|cx| {
            cx.update_global::<CrashReportSnapshot, _>(|snap, _cx| {
                snap.pending_count = count;
                snap.last_crash_timestamp = timestamp;
            });
        });
    })
    .detach();
}

#[cfg(target_family = "wasm")]
pub fn upload_pending_reports(_cx: &mut App) {
    tracing::debug!(
        target: "gpui_starter::crash_report",
        "crash report upload unavailable on wasm (no crash report producer)"
    );
}

pub fn shutdown(cx: &mut App) {
    tracing::debug!(
        target: "gpui_starter::crash_report",
        "crash report service shutdown requested"
    );
    upload_pending_reports(cx);
}

#[cfg(test)]
#[path = "crash_report.test.rs"]
mod crash_report_test;
