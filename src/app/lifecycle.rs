#[cfg(not(target_family = "wasm"))]
use std::fs;
#[cfg(not(target_family = "wasm"))]
use std::io::Write as _;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

use gpui::{App, Global};
use serde::{Deserialize, Serialize};

use crate::time::AppTimestamp;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LifecycleStage {
    Starting,
    Running,
    ShuttingDown,
    Crashed,
}

#[derive(Clone, Debug)]
pub struct LifecycleState {
    pub stage: LifecycleStage,
    pub updated_at: AppTimestamp,
    pub startup_step: Option<String>,
    pub shutdown_step: Option<String>,
    pub last_startup_error: Option<String>,
    pub last_shutdown_error: Option<String>,
    pub last_error: Option<String>,
}

impl Global for LifecycleState {}

impl Default for LifecycleState {
    fn default() -> Self {
        Self::starting()
    }
}

impl LifecycleState {
    pub fn starting() -> Self {
        Self {
            stage: LifecycleStage::Starting,
            updated_at: AppTimestamp::now(),
            startup_step: None,
            shutdown_step: None,
            last_startup_error: None,
            last_shutdown_error: None,
            last_error: None,
        }
    }
}

pub fn set_stage(stage: LifecycleStage, cx: &mut App) {
    let state = cx.default_global::<LifecycleState>();
    state.stage = stage;
    state.updated_at = AppTimestamp::now();
}

pub fn set_startup_step(step: impl Into<String>, cx: &mut App) {
    let state = cx.default_global::<LifecycleState>();
    state.startup_step = Some(step.into());
    state.updated_at = AppTimestamp::now();
}

pub fn set_shutdown_step(step: impl Into<String>, cx: &mut App) {
    let state = cx.default_global::<LifecycleState>();
    state.shutdown_step = Some(step.into());
    state.updated_at = AppTimestamp::now();
}

pub fn set_startup_error(error: impl Into<String>, cx: &mut App) {
    let error = error.into();
    let state = cx.default_global::<LifecycleState>();
    state.last_startup_error = Some(error.clone());
    state.last_error = Some(error);
    state.updated_at = AppTimestamp::now();
}

pub fn set_shutdown_error(error: impl Into<String>, cx: &mut App) {
    let error = error.into();
    let state = cx.default_global::<LifecycleState>();
    state.last_shutdown_error = Some(error.clone());
    state.last_error = Some(error);
    state.updated_at = AppTimestamp::now();
}

static LAST_PANIC_SUMMARY: OnceLock<Mutex<Option<String>>> = OnceLock::new();

/// Application data directory, set once during startup so the panic hook can
/// write crash report files without needing GPUI context.
static APP_DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Ring buffer of recent errors attached to the next crash report.
static RECENT_ERRORS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();

/// Point the panic hook at the app data dir; call once, after app_state init.
pub fn set_app_data_dir(path: PathBuf) {
    let _ = APP_DATA_DIR.set(path);
}

/// Record a recent error message so it can be attached to the next crash
/// report. Keeps at most 20 entries (oldest are dropped).
pub fn track_recent_error(msg: String) {
    let slot = RECENT_ERRORS.get_or_init(|| Mutex::new(Vec::new()));
    if let Ok(mut guard) = slot.lock() {
        guard.push(msg);
        if guard.len() > 20 {
            guard.remove(0);
        }
    }
}

std::thread_local! {
    static IN_RENDER_PATH: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

struct RenderPathGuard;

impl Drop for RenderPathGuard {
    fn drop(&mut self) {
        IN_RENDER_PATH.with(|c| c.set(false));
    }
}

/// Only render-originating panics trigger the error boundary.
pub fn enter_render_path() -> impl Drop {
    IN_RENDER_PATH.with(|c| c.set(true));
    RenderPathGuard
}

pub fn in_render_path() -> bool {
    IN_RENDER_PATH.with(|c| c.get())
}

/// Set by the panic hook; the next render swaps in the error boundary view.
static RENDER_PANIC_OCCURRED: AtomicBool = AtomicBool::new(false);

pub fn last_panic_summary() -> Option<String> {
    LAST_PANIC_SUMMARY
        .get()
        .and_then(|slot| slot.lock().ok().and_then(|value| value.clone()))
}

pub fn take_render_panic() -> bool {
    RENDER_PANIC_OCCURRED.swap(false, Ordering::SeqCst)
}

pub fn install_panic_hook() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let summary = info.to_string();
        let slot = LAST_PANIC_SUMMARY.get_or_init(|| Mutex::new(None));
        if let Ok(mut value) = slot.lock() {
            *value = Some(summary.clone());
        }

        let is_render = in_render_path();

        // Only render-originating panics swap in the error boundary; other
        // panics (background tasks, init) must not.
        if is_render {
            RENDER_PANIC_OCCURRED.store(true, Ordering::SeqCst);
        }

        if let Some(data_dir) = APP_DATA_DIR.get() {
            let backtrace = std::backtrace::Backtrace::capture();
            let bt_string = match backtrace.status() {
                std::backtrace::BacktraceStatus::Captured => backtrace.to_string(),
                _ => String::new(),
            };

            let recent_errors = RECENT_ERRORS
                .get()
                .and_then(|slot| slot.lock().ok())
                .map(|guard| guard.clone())
                .unwrap_or_default();

            let report = crate::services::crash_report::CrashReport::new(
                summary.clone(),
                bt_string,
                is_render,
                recent_errors,
            );

            if let Err(err) = crate::services::crash_report::write_crash_report(&report, data_dir) {
                // We are inside the panic handler -- best-effort logging only.
                eprintln!("[gpui_starter::lifecycle] failed to write crash report: {err}");
            }
        }

        tracing::error!(
            target: "gpui_starter::lifecycle",
            panic = %summary,
            "application panic captured"
        );
        previous(info);
    }));
}

// File-based crash detection is native-only: path APIs panic at runtime on
// wasm and a browser tab has no next-launch semantics anyway.

// Marker lives under the app data dir, never $TMPDIR (symlink planting); an
// unset dir disables the feature.
#[cfg(not(target_family = "wasm"))]
fn crash_marker_path() -> Option<PathBuf> {
    APP_DATA_DIR.get().map(|dir| dir.join("crash-marker"))
}

#[cfg(not(target_family = "wasm"))]
pub fn write_crash_marker() {
    let Some(path) = crash_marker_path() else {
        return;
    };
    let pid = std::process::id();
    let timestamp = chrono::Utc::now().to_rfc3339();
    // Unlink first, then create_new 0600: a marker left by an older build or
    // a symlink planted on the path is replaced, never written through.
    let _ = fs::remove_file(&path);
    let mut options = fs::File::options();
    options.write(true).create_new(true);
    #[cfg(target_family = "unix")]
    {
        use std::os::unix::fs::OpenOptionsExt as _;
        options.mode(0o600);
    }
    if let Err(err) = options
        .open(&path)
        .and_then(|mut file| write!(file, "pid={pid}\nstarted_at={timestamp}\n"))
    {
        tracing::warn!(
            target: "gpui_starter::lifecycle",
            path = %path.display(),
            error = %err,
            "failed to write crash marker"
        );
    }
}

#[cfg(not(target_family = "wasm"))]
pub fn check_previous_crash() -> Option<String> {
    let path = crash_marker_path()?;
    if path.exists() {
        match fs::read_to_string(&path) {
            Ok(contents) => Some(contents),
            Err(err) => {
                tracing::warn!(
                    target: "gpui_starter::lifecycle",
                    path = %path.display(),
                    error = %err,
                    "crash marker exists but could not be read"
                );
                Some("<unreadable>".to_string())
            }
        }
    } else {
        None
    }
}

#[cfg(not(target_family = "wasm"))]
pub fn remove_crash_marker() {
    let Some(path) = crash_marker_path() else {
        return;
    };
    if path.exists()
        && let Err(err) = fs::remove_file(&path)
    {
        tracing::warn!(
            target: "gpui_starter::lifecycle",
            path = %path.display(),
            error = %err,
            "failed to remove crash marker"
        );
    }
}

/// Wasm: no filesystem — a crash marker can never be written. No-op.
#[cfg(target_family = "wasm")]
pub fn write_crash_marker() {}

/// Wasm: no crash marker can ever exist; report no previous crash.
#[cfg(target_family = "wasm")]
pub fn check_previous_crash() -> Option<String> {
    None
}

/// Wasm: no crash marker to remove. No-op.
#[cfg(target_family = "wasm")]
pub fn remove_crash_marker() {}

#[cfg(test)]
#[path = "lifecycle.test.rs"]
mod lifecycle_test;
