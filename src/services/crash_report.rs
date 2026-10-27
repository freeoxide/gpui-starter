use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use gpui_kit::App;
#[cfg(not(target_family = "wasm"))]
use gpui_kit::BorrowAppContext as _;
use gpui_kit::Global;
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
        // Runs inside the panic hook: id and timestamp come from std only so
        // no external crate can be mid-borrow when this executes.
        let nanos = unix_now_nanos();
        Self {
            id: report_id(nanos),
            panic_message: scrub(&panic_message, MAX_PANIC_MESSAGE_CHARS),
            backtrace: scrub(&backtrace, MAX_BACKTRACE_CHARS),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            os: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
            timestamp: rfc3339(nanos),
            render_path,
            recent_errors: recent_errors
                .into_iter()
                .take(MAX_RECENT_ERRORS)
                .map(|err| scrub(&err, MAX_RECENT_ERROR_CHARS))
                .collect(),
        }
    }
}

/// Clock collisions cannot produce duplicate ids; the counter guarantees it.
static REPORT_SEQUENCE: AtomicU64 = AtomicU64::new(0);

fn unix_now_nanos() -> u128 {
    // Pre-epoch clocks (misconfigured guests) clamp to the epoch: the hook
    // path never panics.
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

fn report_id(nanos: u128) -> String {
    let seq = REPORT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("crash-{nanos:x}-{}-{seq}", std::process::id())
}

/// RFC3339 in fixed-width form so lexicographic order matches chronology
/// without chrono.
fn rfc3339(nanos: u128) -> String {
    let secs = (nanos / 1_000_000_000) as i64;
    let subsec = (nanos % 1_000_000_000) as u32;
    let (year, month, day) = civil_from_days(secs.div_euclid(86_400));
    let time_of_day = secs.rem_euclid(86_400);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}.{subsec:09}+00:00",
        time_of_day / 3600,
        (time_of_day % 3600) / 60,
        time_of_day % 60,
    )
}

/// Days since the Unix epoch to proleptic Gregorian Y/M/D (Howard Hinnant's
/// civil_from_days).
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = (z - era * 146_097) as u64;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_prime + 2) / 5 + 1) as u32;
    let month = (if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    }) as u32;
    (
        year_of_era as i64 + era * 400 + i64::from(month <= 2),
        month,
        day,
    )
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

/// Synchronous and stdlib-only: the panic hook owns this path, so no async
/// runtime and no logging/serialization framework may run down it.
pub fn write_crash_report(report: &CrashReport, data_dir: &Path) -> std::io::Result<()> {
    // Panics on other threads serialize here; a poisoned lock means an
    // earlier write panicked and this one should still proceed.
    static WRITE_LOCK: Mutex<()> = Mutex::new(());
    let _guard = WRITE_LOCK
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);

    let reports_dir = data_dir.join("crash_reports");
    std::fs::create_dir_all(&reports_dir)?;

    let file_path = reports_dir.join(format!("{}.json", report.id));
    std::fs::write(&file_path, report_json(report))?;
    Ok(())
}

/// Same shape serde_json::to_string_pretty produced historically; the
/// reader-side round-trip tests pin the schema.
fn report_json(report: &CrashReport) -> String {
    let mut out = String::with_capacity(512);
    out.push('{');
    json_str_field(&mut out, "id", &report.id);
    json_str_field(&mut out, "panic_message", &report.panic_message);
    json_str_field(&mut out, "backtrace", &report.backtrace);
    json_str_field(&mut out, "app_version", &report.app_version);
    json_str_field(&mut out, "os", &report.os);
    json_str_field(&mut out, "arch", &report.arch);
    json_str_field(&mut out, "timestamp", &report.timestamp);
    out.push_str(",\n  \"render_path\": ");
    out.push_str(if report.render_path { "true" } else { "false" });
    out.push_str(",\n  \"recent_errors\": [");
    if !report.recent_errors.is_empty() {
        for (index, error) in report.recent_errors.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            out.push_str("\n    ");
            push_json_string(&mut out, error);
        }
        out.push_str("\n  ");
    }
    out.push_str("]\n}");
    out
}

fn json_str_field(out: &mut String, key: &str, value: &str) {
    if out.ends_with('{') {
        out.push('\n');
    } else {
        out.push_str(",\n");
    }
    out.push_str("  \"");
    out.push_str(key);
    out.push_str("\": ");
    push_json_string(out, value);
}

/// Escapes exactly the characters serde_json escapes; everything else
/// (including non-ASCII) passes through as UTF-8.
fn push_json_string(out: &mut String, text: &str) {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if (ch as u32) < 0x20 => {
                let code = ch as u32;
                out.push_str("\\u00");
                out.push(HEX[((code >> 4) & 0xf) as usize] as char);
                out.push(HEX[(code & 0xf) as usize] as char);
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
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
