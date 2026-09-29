use std::collections::HashSet;
#[cfg(not(target_family = "wasm"))]
use std::io::{Read as _, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(not(target_family = "wasm"))]
use atomic_write_file::AtomicWriteFile;
use gpui_kit::{App, BorrowAppContext, Global, Task};
// Renamed upstream at gpui-component 5a5e2ab; variant names (and so the
// persisted serde representation) are unchanged.
use gpui_kit::component::scroll::ScrollbarMode;
use serde::{Deserialize, Serialize};

use crate::{
    app::{LOCALE_EN, LOCALE_ZH_CN},
    errors::AppError,
    notifications::inbox::NotificationInboxItem,
    paths::AppPaths,
    routes::AppRoute,
};
// Atomic-write persistence helpers are native-only (see `save_config`).
#[cfg(not(target_family = "wasm"))]
use crate::paths::ensure_parent_dir;

/// Wait after the last config mutation before flushing, so bursts of
/// `update_config` calls coalesce into a single write.
const DEBOUNCE_MS: u64 = 300;

/// Upper bound for the state file. A larger file is corrupt by definition, and
/// parsing it would allocate unbounded memory from untrusted input at startup.
#[cfg(not(target_family = "wasm"))]
const MAX_STATE_FILE_BYTES: u64 = 1024 * 1024;

pub const APP_STATE_VERSION: u32 = 1;

/// Bounds beyond this cannot be a real window on any display; they indicate a
/// corrupt file. Generous enough that exotic multi-monitor spans still persist.
const MAX_PLAUSIBLE_DIM: f32 = 100_000.0;

/// Set while a debounce timer is armed, so concurrent mutations reuse the
/// pending timer instead of spawning another one.
static SAVE_SCHEDULED: AtomicBool = AtomicBool::new(false);

/// App-wide config store (GPUI global). Write only via [`update_config`]
/// and [`force_save`], which own the debounced persistence; in-crate reads
/// go through the helper fns in this module where one exists.
pub struct AppState {
    // Crate-visible only while diagnostics rows read them directly; external
    // consumers go through the reader helpers below.
    pub(crate) paths: AppPaths,
    pub(crate) config: AppConfig,
    pub(crate) last_load_error: Option<String>,
    pub(crate) last_save_error: Option<String>,
    dirty: bool,
    // Serialized bytes of the last successful flush; lets identical states skip the write.
    last_flushed_bytes: Vec<u8>,
    // Bound debounce task; dropped on shutdown so it cannot commit stale bytes
    // beneath the synchronous flush (dropping a `Task` cancels it).
    in_flight_save: Option<Task<()>>,
}

impl Global for AppState {}

/// Persisted application configuration. The pub fields are the on-disk serde
/// schema pinned by tests/snapshot_tests.rs: add new fields with a
/// `#[serde(default)]`, never rename or repurpose an existing one.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AppConfig {
    pub version: u32,
    pub theme: String,
    pub scrollbar_show: Option<ScrollbarMode>,
    pub locale: String,
    pub active_route: AppRoute,
    pub sidebar_collapsed: bool,
    pub native_notifications_enabled: bool,
    #[serde(default = "default_true")]
    pub global_shortcut_enabled: bool,
    pub first_run_completed: bool,
    pub notification_inbox: Vec<NotificationInboxItem>,
    pub window_bounds: Option<PersistedWindowBounds>,
    #[serde(default)]
    pub granted_permissions: HashSet<String>,
    #[serde(default)]
    pub denied_permissions: HashSet<String>,
    #[serde(default = "default_stable")]
    pub update_channel: String,
    #[serde(default)]
    pub last_update_check: Option<String>,
    #[serde(default = "default_show_frame_time")]
    pub show_frame_time: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PersistedWindowBounds {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl PersistedWindowBounds {
    /// Finite, positive dimensions within the corruption ceiling; negative
    /// x/y stay legal (multi-monitor).
    fn is_plausible(&self) -> bool {
        self.x.is_finite()
            && self.y.is_finite()
            && self.width.is_finite()
            && self.height.is_finite()
            && self.width > 0.0
            && self.height > 0.0
            && self.width <= MAX_PLAUSIBLE_DIM
            && self.height <= MAX_PLAUSIBLE_DIM
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: APP_STATE_VERSION,
            theme: "Default Light".to_string(),
            scrollbar_show: None,
            locale: LOCALE_EN.to_string(),
            active_route: AppRoute::default(),
            sidebar_collapsed: false,
            native_notifications_enabled: true,
            global_shortcut_enabled: true,
            first_run_completed: false,
            notification_inbox: Vec::new(),
            window_bounds: None,
            granted_permissions: HashSet::new(),
            denied_permissions: HashSet::new(),
            update_channel: default_stable(),
            last_update_check: None,
            show_frame_time: default_show_frame_time(),
        }
    }
}

impl AppConfig {
    /// Canonical locale/version, and drop window bounds a corrupt file could
    /// have planted so window creation always sees usable values.
    pub fn normalized(mut self) -> Self {
        if self.version == 0 {
            self.version = APP_STATE_VERSION;
        }
        if self.locale != LOCALE_EN && self.locale != LOCALE_ZH_CN {
            self.locale = LOCALE_EN.to_string();
        }
        if let Some(bounds) = &self.window_bounds
            && !bounds.is_plausible()
        {
            self.window_bounds = None;
        }
        self
    }
}

fn default_true() -> bool {
    true
}

fn default_stable() -> String {
    "stable".to_string()
}

fn default_show_frame_time() -> bool {
    cfg!(debug_assertions)
}

pub fn initialize(cx: &mut App) {
    let paths = match AppPaths::new() {
        Ok(paths) => paths,
        // wasm has no OS standard directories; degrade to the in-memory
        // fallback (default config, no disk persistence) instead of bailing.
        #[cfg(target_family = "wasm")]
        Err(_) => AppPaths::fallback(),
        #[cfg(not(target_family = "wasm"))]
        Err(err) => {
            tracing::error!(target: "gpui_starter::app_state", error = %err, "failed to initialize app paths");
            return;
        }
    };

    // wasm has no readable state file (fallback path is empty) — start from
    // defaults instead of attempting a filesystem load.
    #[cfg(target_family = "wasm")]
    let (config, last_load_error) = (AppConfig::default(), None);
    #[cfg(not(target_family = "wasm"))]
    let (config, last_load_error) = load_config(&paths.state_file);
    tracing::info!(
        target: "gpui_starter::app_state",
        state_file = %paths.state_file.display(),
        config_dir = %paths.config_dir.display(),
        data_dir = %paths.data_dir.display(),
        log_dir = %paths.log_dir.display(),
        last_load_error = ?last_load_error,
        "loaded app state"
    );

    // Pre-compute the initial flushed bytes so that a no-op update_config
    // immediately after startup will skip the write.
    let initial_bytes = serde_json::to_vec(&config).unwrap_or_default();

    cx.set_global(AppState {
        paths,
        config,
        last_load_error,
        last_save_error: None,
        dirty: false,
        last_flushed_bytes: initial_bytes,
        in_flight_save: None,
    });
}

pub fn config(cx: &App) -> AppConfig {
    cx.try_global::<AppState>()
        .map(|s| s.config.clone())
        .unwrap_or_default()
}

/// Borrow the active [`AppConfig`] without cloning the whole struct; prefer
/// this (or [`with_config`]) over [`config`] in render paths.
pub fn config_handle(cx: &App) -> Option<&AppConfig> {
    cx.try_global::<AppState>().map(|s| &s.config)
}

/// Run a closure with borrowed access to the active [`AppConfig`], falling
/// back to the default when [`initialize`] has not run.
pub fn with_config<R>(cx: &App, f: impl FnOnce(&AppConfig) -> R) -> R {
    match cx.try_global::<AppState>() {
        Some(state) => f(&state.config),
        None => f(&AppConfig::default()),
    }
}

/// Convenience getter cloning just the update channel instead of the config.
pub fn update_channel(cx: &App) -> String {
    with_config(cx, |c| c.update_channel.clone())
}

pub fn paths(cx: &App) -> AppPaths {
    cx.try_global::<AppState>()
        .map(|s| s.paths.clone())
        .unwrap_or_else(|| {
            tracing::error!(target: "gpui_starter::app_state", "AppState not initialized, using fallback paths");
            // wasm: no OS directories exist — never panic on the missing
            // ProjectDirs, degrade to the in-memory fallback instead.
            #[cfg(target_family = "wasm")]
            {
                AppPaths::fallback()
            }
            #[cfg(not(target_family = "wasm"))]
            {
                AppPaths::new().expect("failed to initialize fallback app paths")
            }
        })
}

/// Mutate the config and schedule a debounced, coalesced save; the closure
/// runs synchronously. Use [`force_save`] for immediate persistence.
pub fn update_config(cx: &mut App, update: impl FnOnce(&mut AppConfig)) {
    if cx.try_global::<AppState>().is_none() {
        tracing::warn!(target: "gpui_starter::app_state", "attempted to update app state before initialization");
        return;
    }

    cx.update_global::<AppState, _>(|state, _cx| {
        update(&mut state.config);
        state.config = state.config.clone().normalized();
        state.dirty = true;
    });

    arm_debounce(cx);
}

/// Flush pending config changes to disk immediately; a no-op when clean.
/// Called on the shutdown path, where the debounced write cannot fire.
pub fn force_save(cx: &mut App) {
    if cx.try_global::<AppState>().is_none() {
        return;
    }

    SAVE_SCHEDULED.store(false, Ordering::Relaxed);

    // Shutdown: write synchronously (the process must not exit before the
    // write lands); the debounced hot path keeps fsync off the UI thread.
    cx.update_global::<AppState, _>(|state, _cx| {
        // Drop the debounce task first so it cannot race or reorder beneath
        // the synchronous write below.
        state.in_flight_save = None;

        if !state.dirty {
            return;
        }
        if let Some((path, bytes)) = prepare_flush(state) {
            let result = save_config(&path, &bytes);
            // Nothing to re-arm on the shutdown path.
            let _ = commit_flush(state, bytes, result);
        }
    });
}

/// Arm the debounced flush timer unless one is already pending; the
/// `SAVE_SCHEDULED` compare_exchange keeps at most one timer in flight.
fn arm_debounce(cx: &mut App) {
    if SAVE_SCHEDULED
        .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed)
        .is_err()
    {
        return;
    }

    let bg = cx.background_executor().clone();
    let task = cx.spawn(async move |cx| {
        bg.timer(std::time::Duration::from_millis(DEBOUNCE_MS))
            .await;

        SAVE_SCHEDULED.store(false, Ordering::Relaxed);

        // No I/O here.
        let request = cx.update(|cx| {
            cx.update_global::<AppState, _>(|state, _cx| {
                if !state.dirty {
                    return None;
                }
                prepare_flush(state)
            })
        });

        let Some((path, bytes)) = request else {
            return;
        };

        let write_bytes = bytes.clone();
        let result = bg
            .spawn(async move { save_config(&path, &write_bytes) })
            .await;

        cx.update(|cx| {
            let needs_rearm =
                cx.update_global::<AppState, _>(|state, _cx| commit_flush(state, bytes, result));
            if needs_rearm {
                arm_debounce(cx);
            }
        });
    });

    cx.update_global::<AppState, _>(|state, _cx| {
        state.in_flight_save = Some(task);
    });
}

/// Serialize on the UI thread (no I/O); `None` when clean or on failure.
fn prepare_flush(state: &mut AppState) -> Option<(PathBuf, Vec<u8>)> {
    let new_bytes = match serde_json::to_vec(&state.config) {
        Ok(bytes) => bytes,
        Err(err) => {
            let error = err.to_string();
            tracing::error!(
                target: "gpui_starter::app_state",
                error = %error,
                "failed to serialize app state"
            );
            state.last_save_error = Some(error);
            return None;
        }
    };

    if new_bytes == state.last_flushed_bytes {
        state.dirty = false;
        return None;
    }

    Some((state.paths.state_file.clone(), new_bytes))
}

/// Apply a [`save_config`] outcome; `true` means a stale write (mutation
/// mid-flight or reordered commits) re-marked dirty — re-arm the debounce.
fn commit_flush(
    state: &mut AppState,
    written_bytes: Vec<u8>,
    result: Result<(), AppError>,
) -> bool {
    match result {
        Ok(()) => {
            state.last_save_error = None;
            let current = serde_json::to_vec(&state.config).unwrap_or_default();
            if current == written_bytes {
                state.dirty = false;
                state.last_flushed_bytes = written_bytes;
                tracing::debug!(
                    target: "gpui_starter::app_state",
                    state_file = %state.paths.state_file.display(),
                    "persisted app state"
                );
                false
            } else {
                // Stale write: re-flush the current bytes, and don't let a
                // concurrent clean commit keep `dirty = false`.
                state.dirty = true;
                tracing::debug!(
                    target: "gpui_starter::app_state",
                    "config changed during flush; will re-flush on next debounce"
                );
                true
            }
        }
        Err(err) => {
            let error = err.to_string();
            tracing::error!(
                target: "gpui_starter::app_state",
                error = %error,
                "failed to persist app state"
            );
            state.last_save_error = Some(error);
            false
        }
    }
}

#[cfg(not(target_family = "wasm"))]
fn load_config(path: &Path) -> (AppConfig, Option<String>) {
    // Cap untrusted input before parsing: serde on an oversized file would
    // allocate unbounded memory at startup.
    let file = match std::fs::File::open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return (AppConfig::default(), None);
        }
        Err(err) => return state_read_error(path, err),
    };

    match file.metadata() {
        Ok(meta) if meta.len() > MAX_STATE_FILE_BYTES => {
            quarantine_bad_config(path);
            return oversized_state_error(path);
        }
        Ok(_) => {}
        Err(err) => return state_read_error(path, err),
    }

    let mut json = String::new();
    if let Err(err) = file
        .take(MAX_STATE_FILE_BYTES + 1)
        .read_to_string(&mut json)
    {
        return state_read_error(path, err);
    }
    // The bounded read is the real cap; this catches a file that grew past
    // it between the metadata check and the read.
    if json.len() as u64 > MAX_STATE_FILE_BYTES {
        quarantine_bad_config(path);
        return oversized_state_error(path);
    }

    match serde_json::from_str::<AppConfig>(&json) {
        Ok(config) => {
            let config = crate::state::migrations::migrate(config).normalized();
            // Log-only tier: lints surface unusable-but-loadable values.
            crate::state::config_validation::validate_config(&config);
            (config, None)
        }
        Err(err) => {
            quarantine_bad_config(path);
            (
                AppConfig::default(),
                Some(
                    AppError::StateParse {
                        path: path.to_path_buf(),
                        details: err.to_string(),
                    }
                    .to_string(),
                ),
            )
        }
    }
}

#[cfg(not(target_family = "wasm"))]
fn state_read_error(path: &Path, err: std::io::Error) -> (AppConfig, Option<String>) {
    (
        AppConfig::default(),
        Some(
            AppError::StateRead {
                path: path.to_path_buf(),
                details: err.to_string(),
            }
            .to_string(),
        ),
    )
}

#[cfg(not(target_family = "wasm"))]
fn oversized_state_error(path: &Path) -> (AppConfig, Option<String>) {
    (
        AppConfig::default(),
        Some(
            AppError::StateParse {
                path: path.to_path_buf(),
                details: format!("file exceeds the {MAX_STATE_FILE_BYTES}-byte cap"),
            }
            .to_string(),
        ),
    )
}

/// Write pre-serialized bytes atomically.
fn save_config(path: &Path, json_bytes: &[u8]) -> Result<(), AppError> {
    // wasm has no writable config directory (fallback path is empty) — treat
    // persistence as a no-op so the in-memory config stays authoritative.
    #[cfg(target_family = "wasm")]
    {
        let _ = (path, json_bytes);
        return Ok(());
    }
    #[cfg(not(target_family = "wasm"))]
    {
        ensure_parent_dir(path)?;
        // `mut` is only consumed by the unix mode/preserve_mode calls below.
        #[cfg_attr(not(target_family = "unix"), allow(unused_mut))]
        let mut options = AtomicWriteFile::options();
        // The rename commit replaces a planted symlink instead of writing
        // through it; every commit lands 0600 (preserve_mode off).
        #[cfg(target_family = "unix")]
        {
            use atomic_write_file::unix::OpenOptionsExt as _;
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600).preserve_mode(false);
        }
        let mut file = options.open(path).map_err(|err| AppError::StateWrite {
            path: path.to_path_buf(),
            details: err.to_string(),
        })?;
        file.write_all(json_bytes)
            .map_err(|err| AppError::StateWrite {
                path: path.to_path_buf(),
                details: err.to_string(),
            })?;
        file.write_all(b"\n").map_err(|err| AppError::StateWrite {
            path: path.to_path_buf(),
            details: err.to_string(),
        })?;
        file.commit().map_err(|err| AppError::StateWrite {
            path: path.to_path_buf(),
            details: err.to_string(),
        })?;
        Ok(())
    }
}

#[cfg(not(target_family = "wasm"))]
fn quarantine_bad_config(path: &Path) {
    if !path.exists() {
        return;
    }
    let quarantine_path = path.with_extension("json.bad");
    if let Err(err) = std::fs::rename(path, &quarantine_path) {
        tracing::warn!(
            target: "gpui_starter::app_state",
            source = %path.display(),
            target_path = %quarantine_path.display(),
            error = %err,
            "failed to quarantine corrupt app state"
        );
    }
}

#[cfg(test)]
#[path = "config_store.test.rs"]
mod config_store_test;
