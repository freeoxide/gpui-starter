use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[cfg(not(target_family = "wasm"))]
use arboard::Clipboard;
use gpui_kit::{App, BorrowAppContext as _, Global};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

#[derive(Debug, thiserror::Error)]
pub enum DesktopActionError {
    #[cfg(not(target_family = "wasm"))]
    #[error("clipboard operation failed: {0}")]
    Clipboard(#[from] arboard::Error),
    #[error("failed to open path '{path}'")]
    OpenPathFailed {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to open url '{url}'")]
    OpenUrlFailed {
        url: String,
        #[source]
        source: std::io::Error,
    },
    #[error("file dialog failed: {0}")]
    DialogFailed(String),
    #[error("watcher error: {0}")]
    Watcher(#[from] notify::Error),
    #[error("desktop actions unavailable")]
    Unavailable,
    #[error("watcher lock poisoned")]
    LockPoisoned,
}

#[derive(Clone, Debug)]
pub struct DesktopActionsSnapshot {
    pub clipboard_available: bool,
    pub picker_available: bool,
    pub opener_available: bool,
    pub active_watchers: usize,
    pub last_error: Option<String>,
}

impl Default for DesktopActionsSnapshot {
    fn default() -> Self {
        Self {
            clipboard_available: false,
            picker_available: true,
            opener_available: true,
            active_watchers: 0,
            last_error: None,
        }
    }
}

#[derive(Clone)]
pub struct DesktopActionsState {
    snapshot: DesktopActionsSnapshot,
    inner: Arc<Mutex<DesktopActionsInner>>,
}

struct DesktopActionsInner {
    next_watcher_id: u64,
    watchers: BTreeMap<u64, RecommendedWatcher>,
}

impl Global for DesktopActionsState {}

pub fn initialize(cx: &mut App) {
    // Wasm swaps in `navigator.clipboard` (see platform::web::clipboard) and
    // compiles out rfd/`open`; the notify watchers simply never fire there.
    #[cfg(not(target_family = "wasm"))]
    let clipboard_ok = Clipboard::new().is_ok();
    #[cfg(target_family = "wasm")]
    let clipboard_ok = crate::platform::web::clipboard::is_available();
    let mut snapshot = DesktopActionsSnapshot {
        clipboard_available: clipboard_ok,
        opener_available: !cfg!(target_family = "wasm"),
        picker_available: !cfg!(target_family = "wasm"),
        ..DesktopActionsSnapshot::default()
    };
    if !snapshot.clipboard_available {
        snapshot.last_error = Some("clipboard backend unavailable".to_string());
    }
    let status = match snapshot.last_error.as_deref() {
        Some(err) => crate::capabilities::CapabilityStatus::degraded(err),
        None => crate::capabilities::CapabilityStatus::supported_enabled(),
    };
    crate::capabilities::set("desktop_actions", status, cx);
    cx.set_global(DesktopActionsState {
        snapshot,
        inner: Arc::new(Mutex::new(DesktopActionsInner {
            next_watcher_id: 1,
            watchers: BTreeMap::new(),
        })),
    });
}

pub fn snapshot(cx: &App) -> DesktopActionsSnapshot {
    cx.try_global::<DesktopActionsState>()
        .map(|s| s.snapshot.clone())
        .unwrap_or_default()
}

pub fn copy_text(text: &str, cx: &mut App) -> Result<(), DesktopActionError> {
    // Wasm: the Promise result of `navigator.clipboard.writeText` can't be
    // observed from this sync signature — the write is fire-and-forget.
    #[cfg(target_family = "wasm")]
    {
        crate::platform::web::clipboard::write_text_fire_and_forget(text);
        update_result("clipboard_copy", None, cx);
        return Ok(());
    }
    #[cfg(not(target_family = "wasm"))]
    {
        let result = Clipboard::new()
            .and_then(|mut clipboard| clipboard.set_text(text.to_string()))
            .map_err(DesktopActionError::from);
        update_result(
            "clipboard_copy",
            result.as_ref().err().map(|e| e.to_string()),
            cx,
        );
        result
    }
}

pub fn copy_diagnostics(cx: &mut App) -> Result<(), DesktopActionError> {
    let diagnostics = build_diagnostics_text(cx);
    copy_text(&diagnostics, cx)
}

pub fn open_logs_folder(cx: &mut App) -> Result<(), DesktopActionError> {
    let path = crate::app_state::paths(cx).log_dir;
    open_path(path, cx)
}

pub fn open_config_folder(cx: &mut App) -> Result<(), DesktopActionError> {
    let path = crate::app_state::paths(cx).config_dir;
    open_path(path, cx)
}

pub fn open_url(url: &str, cx: &mut App) -> Result<(), DesktopActionError> {
    // Only web URLs are ever expected here; rejecting other schemes keeps
    // `xdg-open` from being aimed at files, or custom/fictional handlers.
    let scheme_ok = {
        let (scheme, rest) = url.split_once(':').unwrap_or(("", ""));
        !rest.is_empty() && matches!(scheme, "http" | "https")
    };
    #[cfg(target_family = "wasm")]
    {
        // No `open` crate on wasm (it hits compile_error); a future web entry
        // could use window.open here instead.
        let result = if scheme_ok {
            Err(DesktopActionError::Unavailable)
        } else {
            Err(DesktopActionError::OpenUrlFailed {
                url: url.to_string(),
                source: std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "only http/https URLs are supported",
                ),
            })
        };
        update_result("open_url", result.as_ref().err().map(|e| e.to_string()), cx);
        return result;
    }
    #[cfg(not(target_family = "wasm"))]
    {
        let result = if scheme_ok {
            open::that_detached(url).map_err(|source| DesktopActionError::OpenUrlFailed {
                url: url.to_string(),
                source,
            })
        } else {
            Err(DesktopActionError::OpenUrlFailed {
                url: url.to_string(),
                source: std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "only http/https URLs are supported",
                ),
            })
        };
        update_result("open_url", result.as_ref().err().map(|e| e.to_string()), cx);
        result
    }
}

#[cfg(not(target_family = "wasm"))]
pub fn pick_file(cx: &mut App) -> Option<PathBuf> {
    pick_with_dialog("pick_file", rfd::FileDialog::new().pick_file(), cx)
}

#[cfg(target_family = "wasm")]
pub fn pick_file(cx: &mut App) -> Option<PathBuf> {
    pick_with_dialog("pick_file", None, cx)
}

#[cfg(not(target_family = "wasm"))]
pub fn pick_folder(cx: &mut App) -> Option<PathBuf> {
    pick_with_dialog("pick_folder", rfd::FileDialog::new().pick_folder(), cx)
}

#[cfg(target_family = "wasm")]
pub fn pick_folder(cx: &mut App) -> Option<PathBuf> {
    pick_with_dialog("pick_folder", None, cx)
}

#[cfg(not(target_family = "wasm"))]
pub fn save_file(cx: &mut App) -> Option<PathBuf> {
    pick_with_dialog("save_file", rfd::FileDialog::new().save_file(), cx)
}

#[cfg(target_family = "wasm")]
pub fn save_file(cx: &mut App) -> Option<PathBuf> {
    pick_with_dialog("save_file", None, cx)
}

/// `rfd::FileDialog` does not exist on wasm, so the dialog helpers report
/// "unavailable" and return `None` there.
fn pick_with_dialog(action: &'static str, _file: Option<PathBuf>, cx: &mut App) -> Option<PathBuf> {
    #[cfg(target_family = "wasm")]
    {
        tracing::debug!(
            target: "gpui_starter::desktop_actions",
            action,
            "file dialog unavailable on wasm"
        );
        update_result(
            action,
            Some("file dialog unavailable on wasm".to_string()),
            cx,
        );
        return None;
    }
    #[cfg(not(target_family = "wasm"))]
    {
        tracing::info!(target: "gpui_starter::desktop_actions", file = ?_file, "file picker result");
        update_result(action, None, cx);
        _file
    }
}

fn watch_path(path: PathBuf, cx: &mut App) -> Result<u64, DesktopActionError> {
    let state = cx
        .try_global::<DesktopActionsState>()
        .ok_or(DesktopActionError::Unavailable)?;
    let mut inner = state
        .inner
        .lock()
        .map_err(|_| DesktopActionError::LockPoisoned)?;
    let watcher_id = inner.next_watcher_id;
    inner.next_watcher_id += 1;

    let mut watcher =
        notify::recommended_watcher(move |result: notify::Result<notify::Event>| match result {
            Ok(event) => tracing::debug!(
                target: "gpui_starter::desktop_actions",
                watcher_id,
                kind = ?event.kind,
                paths = ?event.paths,
                "watch event"
            ),
            Err(err) => tracing::warn!(
                target: "gpui_starter::desktop_actions",
                watcher_id,
                error = %err,
                "watch event error"
            ),
        })
        .map_err(DesktopActionError::from)?;
    watcher
        .watch(&path, RecursiveMode::NonRecursive)
        .map_err(DesktopActionError::from)?;
    inner.watchers.insert(watcher_id, watcher);
    let active_watchers = inner.watchers.len();
    drop(inner);
    cx.update_global::<DesktopActionsState, _>(|state, _cx| {
        state.snapshot.active_watchers = active_watchers;
        state.snapshot.last_error = None;
    });
    tracing::info!(target: "gpui_starter::desktop_actions", watcher_id, path = %path.display(), "watcher registered");
    Ok(watcher_id)
}

pub fn shutdown(cx: &mut App) {
    let Some(state) = cx.try_global::<DesktopActionsState>() else {
        return;
    };
    if let Ok(mut inner) = state.inner.lock() {
        inner.watchers.clear();
    }
    cx.update_global::<DesktopActionsState, _>(|state, _cx| {
        state.snapshot.active_watchers = 0;
    });
}

pub fn watch_log_dir(cx: &mut App) -> Result<u64, DesktopActionError> {
    let path = crate::app_state::paths(cx).log_dir;
    watch_path(path, cx)
}

pub fn watch_config_dir(cx: &mut App) -> Result<u64, DesktopActionError> {
    let path = crate::app_state::paths(cx).config_dir;
    watch_path(path, cx)
}

fn unwatch_path(id: u64, cx: &mut App) -> bool {
    let Some(state) = cx.try_global::<DesktopActionsState>() else {
        return false;
    };
    let (removed, active_watchers) = if let Ok(mut inner) = state.inner.lock() {
        let removed = inner.watchers.remove(&id).is_some();
        (removed, inner.watchers.len())
    } else {
        return false;
    };
    cx.update_global::<DesktopActionsState, _>(|state, _cx| {
        state.snapshot.active_watchers = active_watchers;
    });
    removed
}

pub fn unwatch_all(cx: &mut App) -> usize {
    let state = match cx.try_global::<DesktopActionsState>().cloned() {
        Some(state) => state,
        None => return 0,
    };
    let watcher_ids = state
        .inner
        .lock()
        .ok()
        .map(|inner| inner.watchers.keys().copied().collect::<Vec<_>>())
        .unwrap_or_default();
    let count = watcher_ids.len();
    for watcher_id in watcher_ids {
        let _ = unwatch_path(watcher_id, cx);
    }
    count
}

fn build_diagnostics_text(cx: &App) -> String {
    let app_state = crate::app_state::config(cx);
    let stage = cx
        .try_global::<crate::lifecycle::LifecycleState>()
        .map(|s| s.stage)
        .unwrap_or(crate::lifecycle::LifecycleStage::Starting);
    let connectivity = crate::connectivity::snapshot(cx);
    format!(
        "app={} version={} route={} lifecycle={:?} connectivity={:?}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        app_state.active_route.to_url(),
        stage,
        connectivity.state
    )
}

pub fn open_path(path: PathBuf, cx: &mut App) -> Result<(), DesktopActionError> {
    #[cfg(target_family = "wasm")]
    {
        // No `open` crate on wasm; degrade to an explicit error instead.
        let result = Err(DesktopActionError::Unavailable);
        update_result(
            "open_path",
            result.as_ref().err().map(|e| e.to_string()),
            cx,
        );
        let _ = path;
        return result;
    }
    #[cfg(not(target_family = "wasm"))]
    {
        let result = open::that_detached(path.as_path()).map_err(|source| {
            DesktopActionError::OpenPathFailed {
                path: path.display().to_string(),
                source,
            }
        });
        update_result(
            "open_path",
            result.as_ref().err().map(|e| e.to_string()),
            cx,
        );
        result
    }
}

fn update_result(action: &str, error: Option<String>, cx: &mut App) {
    let Some(state) = cx.try_global::<DesktopActionsState>() else {
        return;
    };
    let active_watchers = state
        .inner
        .lock()
        .ok()
        .map(|inner| inner.watchers.len())
        .unwrap_or(0);
    cx.update_global::<DesktopActionsState, _>(|state, _cx| {
        state.snapshot.last_error = error.clone();
        state.snapshot.active_watchers = active_watchers;
    });
    if let Some(error) = error {
        tracing::warn!(
            target: "gpui_starter::desktop_actions",
            action,
            error = %error,
            "desktop action failed"
        );
        crate::error_surface::report(
            format!("Desktop action `{action}` failed: {error}"),
            crate::errors::AppErrorSeverity::Warning,
            crate::error_surface::ErrorCategory::System,
            vec![
                crate::error_surface::ErrorAction::Retry,
                crate::error_surface::ErrorAction::Dismiss,
            ],
            cx,
        );
    } else {
        tracing::debug!(
            target: "gpui_starter::desktop_actions",
            action,
            "desktop action succeeded"
        );
    }
}
