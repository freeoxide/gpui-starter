use gpui_kit::TestAppContext;
use tempfile::tempdir;

use super::*;
use crate::page::Page;

#[test]
fn save_and_load_config_uses_json_state_file() {
    let dir = tempdir().unwrap();
    let state_file = dir.path().join("state.json");
    let config = AppConfig {
        active_route: AppRoute::page(Page::Settings),
        sidebar_collapsed: true,
        ..AppConfig::default()
    };

    let bytes = serde_json::to_vec(&config).unwrap();
    assert!(save_config(&state_file, &bytes, None).unwrap());
    #[cfg(target_family = "unix")]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = std::fs::metadata(&state_file).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "state file must stay user-only");
    }
    let (loaded, err) = load_config(&state_file);

    assert_eq!(err, None);
    assert_eq!(loaded.active_route, AppRoute::page(Page::Settings));
    assert!(loaded.sidebar_collapsed);

    // The write path's hardening: each commit re-tightens 0600 and the
    // rename replaces a planted symlink instead of writing through it.
    #[cfg(target_family = "unix")]
    {
        use std::os::unix::fs::PermissionsExt as _;

        std::fs::set_permissions(&state_file, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(save_config(&state_file, &bytes, None).unwrap());
        let mode = std::fs::metadata(&state_file).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "every commit must re-tighten to 0600");

        let planted = dir.path().join("planted.json");
        std::fs::write(&planted, b"planted").unwrap();
        std::fs::remove_file(&state_file).unwrap();
        std::os::unix::fs::symlink(&planted, &state_file).unwrap();
        assert!(save_config(&state_file, &bytes, None).unwrap());
        assert!(
            !std::fs::symlink_metadata(&state_file)
                .unwrap()
                .file_type()
                .is_symlink(),
            "the rename commit must replace a planted symlink, not write through it"
        );
        assert_eq!(
            std::fs::read_to_string(&planted).unwrap(),
            "planted",
            "the symlink target must survive untouched"
        );
    }
}

#[test]
fn bluetooth_optin_round_trips_through_the_state_file() {
    let dir = tempdir().unwrap();
    let state_file = dir.path().join("state.json");
    let config = AppConfig {
        bluetooth_monitoring_enabled: true,
        ..AppConfig::default()
    };

    assert!(save_config(&state_file, &serde_json::to_vec(&config).unwrap(), None).unwrap());
    let (loaded, err) = load_config(&state_file);

    assert_eq!(err, None);
    assert!(loaded.bluetooth_monitoring_enabled);
}

#[test]
fn config_without_the_bluetooth_optin_key_defaults_off() {
    // A state file written before the gate exists has no
    // bluetooth_monitoring_enabled key; it must parse to "off".
    let mut value = serde_json::to_value(AppConfig::default()).unwrap();
    let object = value.as_object_mut().unwrap();
    assert!(object.remove("bluetooth_monitoring_enabled").is_some());
    let parsed: AppConfig = serde_json::from_value(value).unwrap();
    assert!(!parsed.bluetooth_monitoring_enabled);
}

#[test]
fn update_check_startup_optin_round_trips_through_the_state_file() {
    let dir = tempdir().unwrap();
    let state_file = dir.path().join("state.json");
    let config = AppConfig {
        update_check_on_startup: true,
        ..AppConfig::default()
    };

    assert!(save_config(&state_file, &serde_json::to_vec(&config).unwrap(), None).unwrap());
    let (loaded, err) = load_config(&state_file);

    assert_eq!(err, None);
    assert!(loaded.update_check_on_startup);
}

#[test]
fn config_without_the_update_check_key_defaults_off() {
    // A state file written before the gate exists has no
    // update_check_on_startup key; it must parse to "off".
    let mut value = serde_json::to_value(AppConfig::default()).unwrap();
    let object = value.as_object_mut().unwrap();
    assert!(object.remove("update_check_on_startup").is_some());
    let parsed: AppConfig = serde_json::from_value(value).unwrap();
    assert!(!parsed.update_check_on_startup);
}

#[test]
fn config_without_the_global_shortcut_key_defaults_on() {
    // A state file written before the field exists must parse to "on"; this
    // serde default is why no migration step forces the value.
    let mut value = serde_json::to_value(AppConfig::default()).unwrap();
    let object = value.as_object_mut().unwrap();
    assert!(object.remove("global_shortcut_enabled").is_some());
    let parsed: AppConfig = serde_json::from_value(value).unwrap();
    assert!(parsed.global_shortcut_enabled);
}

#[test]
fn the_state_file_is_pretty_printed_for_hand_editing() {
    let dir = tempdir().unwrap();
    let state_file = dir.path().join("state.json");
    save_config(
        &state_file,
        &serialize_config(&AppConfig::default()).unwrap(),
        None,
    )
    .unwrap();

    let json = std::fs::read_to_string(&state_file).unwrap();

    assert!(
        json.starts_with("{\n  \"version\": 1,"),
        "state.json must be pretty-printed, found: {json:?}"
    );
    assert!(json.ends_with("}\n"), "state.json must end with a newline");
}

#[test]
fn first_run_writes_the_default_state_file() {
    let dir = tempdir().unwrap();
    let state_file = dir.path().join("state.json");

    let (loaded, err) = load_config(&state_file);

    assert_eq!(err, None);
    assert_eq!(loaded, AppConfig::default());
    assert!(
        state_file.exists(),
        "first run must materialize the default file"
    );
    assert_eq!(
        load_config(&state_file).0,
        AppConfig::default(),
        "the materialized defaults must round-trip"
    );
}

#[test]
fn corrupt_config_is_quarantined() {
    let dir = tempdir().unwrap();
    let state_file = dir.path().join("state.json");
    std::fs::write(&state_file, "{not-json").unwrap();

    let (loaded, err) = load_config(&state_file);

    assert_eq!(loaded, AppConfig::default());
    assert!(err.is_some());
    assert!(!state_file.exists());
    assert!(state_file.with_extension("json.bad").exists());
}

#[test]
fn oversized_config_is_quarantined_without_parsing() {
    let dir = tempdir().unwrap();
    let state_file = dir.path().join("state.json");
    // Valid JSON past the cap: only the size check can reject it, so this
    // pins the cap itself rather than the parse-failure path.
    let mut json = serde_json::to_value(AppConfig::default()).unwrap();
    json["theme"] = serde_json::Value::String("a".repeat(MAX_STATE_FILE_BYTES as usize + 1));
    std::fs::write(&state_file, serde_json::to_vec(&json).unwrap()).unwrap();

    let (loaded, err) = load_config(&state_file);

    assert_eq!(loaded, AppConfig::default());
    let err = err.expect("over-cap file must be reported as a load error");
    assert!(
        err.contains(&format!("exceeds the {MAX_STATE_FILE_BYTES}-byte cap")),
        "the size cap, not a parse failure, must reject the file: {err}"
    );
    assert!(!state_file.exists());
    assert!(state_file.with_extension("json.bad").exists());
}

#[test]
fn newer_version_config_is_quarantined_not_rewritten() {
    // Loading a newer schema here would re-save it v1-shaped on the next
    // flush, destroying the newer fields; the original must be kept aside.
    let dir = tempdir().unwrap();
    let state_file = dir.path().join("state.json");
    let mut json = serde_json::to_value(AppConfig::default()).unwrap();
    json["version"] = serde_json::json!(APP_STATE_VERSION + 1);
    json["future_field"] = serde_json::json!(true);
    std::fs::write(&state_file, serde_json::to_vec(&json).unwrap()).unwrap();

    let (loaded, err) = load_config(&state_file);

    assert_eq!(loaded, AppConfig::default());
    assert!(
        err.is_some(),
        "a newer-version file must report a load error"
    );
    assert!(!state_file.exists());
    let quarantined = state_file.with_extension("json.bad");
    let preserved: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&quarantined).expect("quarantined copy exists"),
    )
    .unwrap();
    assert_eq!(
        preserved["version"],
        serde_json::json!(APP_STATE_VERSION + 1)
    );
    assert_eq!(preserved["future_field"], serde_json::json!(true));
}

#[test]
fn normalized_drops_implausible_window_bounds() {
    let cases = [
        PersistedWindowBounds {
            x: 0.0,
            y: 0.0,
            width: f32::NAN,
            height: 600.0,
            display_id: None,
            state: PersistedWindowState::Windowed,
        },
        PersistedWindowBounds {
            x: 0.0,
            y: 0.0,
            width: f32::INFINITY,
            height: 600.0,
            display_id: None,
            state: PersistedWindowState::Windowed,
        },
        PersistedWindowBounds {
            x: 0.0,
            y: 0.0,
            width: -800.0,
            height: 600.0,
            display_id: None,
            state: PersistedWindowState::Windowed,
        },
        PersistedWindowBounds {
            x: 0.0,
            y: 0.0,
            width: MAX_PLAUSIBLE_DIM * 10.0,
            height: 600.0,
            display_id: None,
            state: PersistedWindowState::Windowed,
        },
    ];

    for bounds in &cases {
        let mut config = AppConfig {
            window_bounds: Some(bounds.clone()),
            ..AppConfig::default()
        };
        config.normalized();
        assert_eq!(
            config.window_bounds, None,
            "implausible bounds {bounds:?} must fall back to default placement"
        );
    }
}

#[test]
fn normalized_keeps_plausible_and_multimonitor_bounds() {
    // A negative origin is legal (left-of-primary monitor), and a window
    // wider than the 8192 lint ceiling is still a real window worth keeping.
    let cases = [
        PersistedWindowBounds {
            x: -1920.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
            display_id: None,
            state: PersistedWindowState::Windowed,
        },
        PersistedWindowBounds {
            x: 0.0,
            y: 0.0,
            width: 9000.0,
            height: 600.0,
            display_id: None,
            state: PersistedWindowState::Windowed,
        },
    ];
    for bounds in &cases {
        let mut config = AppConfig {
            window_bounds: Some(bounds.clone()),
            ..AppConfig::default()
        };
        config.normalized();
        assert!(
            config.window_bounds.is_some(),
            "plausible bounds {bounds:?} must survive normalization"
        );
    }
}

#[test]
fn load_config_repairs_values_the_validation_tier_only_lints() {
    // Repair tier: an unknown locale and negative bounds come back fixed, not
    // just linted. NaN cannot stand in here: serde_json writes it as null.
    let dir = tempdir().unwrap();
    let state_file = dir.path().join("state.json");
    let config = AppConfig {
        locale: Some("klingon".to_string()),
        window_bounds: Some(PersistedWindowBounds {
            x: 0.0,
            y: 0.0,
            width: -800.0,
            height: 600.0,
            display_id: None,
            state: PersistedWindowState::Windowed,
        }),
        ..AppConfig::default()
    };
    std::fs::write(&state_file, serde_json::to_vec(&config).unwrap()).unwrap();

    let (loaded, err) = load_config(&state_file);

    assert_eq!(err, None);
    assert_eq!(loaded.locale.as_deref(), Some(LOCALE_EN));
    assert_eq!(loaded.window_bounds, None);
}

/// Hand-built against a throwaway dir because [`initialize`] would point
/// the store at the real OS config dir; the child module reaches the
/// private fields directly.
fn dirty_state(state_file: &std::path::Path) -> AppState {
    let dir = state_file.parent().unwrap().to_path_buf();
    AppState {
        paths: AppPaths {
            config_dir: dir.clone(),
            data_dir: dir.clone(),
            cache_dir: dir.clone(),
            log_dir: dir.clone(),
            runtime_dir: dir,
            state_file: state_file.to_path_buf(),
        },
        config: AppConfig::default(),
        last_load_error: None,
        last_save_error: None,
        dirty: true,
        last_flushed_bytes: Vec::new(),
        flush_generation: Arc::new(AtomicU64::new(0)),
        in_flight_save: None,
    }
}

#[test]
fn prepare_flush_serializes_a_dirty_state_to_its_file() {
    let dir = tempdir().unwrap();
    let state_file = dir.path().join("state.json");
    let mut state = dirty_state(&state_file);

    let (path, bytes) = prepare_flush(&mut state).expect("dirty state must produce a flush");

    assert_eq!(path, state_file);
    assert_eq!(bytes, serialize_config(&state.config).unwrap());
}

#[test]
fn prepare_flush_skips_unchanged_bytes_and_clears_dirty() {
    let dir = tempdir().unwrap();
    let state_file = dir.path().join("state.json");
    let mut state = dirty_state(&state_file);
    state.last_flushed_bytes = serialize_config(&state.config).unwrap();

    assert!(
        prepare_flush(&mut state).is_none(),
        "a state identical to the last flush must not write again"
    );
    assert!(!state.dirty, "skipping the write re-cleaned the state");
}

#[test]
fn commit_flush_success_commits_the_written_bytes() {
    let dir = tempdir().unwrap();
    let state_file = dir.path().join("state.json");
    let mut state = dirty_state(&state_file);
    state.last_save_error = Some("previous failure".to_string());
    let bytes = serialize_config(&state.config).unwrap();

    let needs_rearm = commit_flush(&mut state, bytes.clone(), Ok(true));

    assert!(!needs_rearm, "a clean commit must not re-arm the debounce");
    assert!(!state.dirty);
    assert_eq!(state.last_flushed_bytes, bytes);
    assert_eq!(state.last_save_error, None, "a good write clears the error");
}

#[test]
fn commit_flush_remarks_dirty_when_config_changed_midflight() {
    let dir = tempdir().unwrap();
    let state_file = dir.path().join("state.json");
    let mut state = dirty_state(&state_file);
    let stale_bytes = serialize_config(&state.config).unwrap();
    // The write raced a mutation: the flushed bytes are no longer current.
    state.config.sidebar_collapsed = !state.config.sidebar_collapsed;

    let needs_rearm = commit_flush(&mut state, stale_bytes, Ok(true));

    assert!(needs_rearm, "a stale write must signal the re-arm");
    assert!(
        state.dirty,
        "the concurrent mutation must not be swallowed by the commit"
    );
    assert!(
        state.last_flushed_bytes.is_empty(),
        "stale bytes must not be recorded as flushed"
    );
}

#[test]
fn commit_flush_failure_keeps_the_state_dirty_for_retry() {
    let dir = tempdir().unwrap();
    let state_file = dir.path().join("state.json");
    let mut state = dirty_state(&state_file);
    let bytes = serialize_config(&state.config).unwrap();
    let error = AppError::StateWrite {
        path: state_file.to_path_buf(),
        details: "disk full".to_string(),
    };

    let needs_rearm = commit_flush(&mut state, bytes, Err(error));

    assert!(!needs_rearm, "a failed write must not loop immediately");
    assert!(
        state.dirty,
        "a failed write stays pending for the next flush"
    );
    assert!(state.last_save_error.is_some());
}

#[test]
fn a_superseded_write_abandons_the_rename() {
    let dir = tempdir().unwrap();
    let state_file = dir.path().join("state.json");
    save_config(&state_file, b"{\"version\":1}", None).unwrap();

    let latest_generation = AtomicU64::new(2);
    let superseded = save_config(
        &state_file,
        b"{\"version\":2}",
        Some((&latest_generation, 1)),
    )
    .unwrap();

    assert!(!superseded, "a superseded write must not commit its rename");
    assert_eq!(
        std::fs::read_to_string(&state_file).unwrap(),
        "{\"version\":1}\n",
        "the previous write must survive the abandoned commit"
    );
}

#[test]
fn commit_flush_rearms_when_a_write_was_superseded() {
    let dir = tempdir().unwrap();
    let state_file = dir.path().join("state.json");
    let mut state = dirty_state(&state_file);
    let bytes = serialize_config(&state.config).unwrap();

    let needs_rearm = commit_flush(&mut state, bytes, Ok(false));

    assert!(needs_rearm, "an abandoned write must re-arm while dirty");
    assert!(state.dirty, "the mutation must stay pending");
    assert!(
        state.last_flushed_bytes.is_empty(),
        "abandoned bytes must not be recorded as flushed"
    );
}

/// A clean store: nothing pending, the default config already flushed, so
/// only a real mutation produces bytes.
fn clean_state(state_file: &std::path::Path) -> AppState {
    let mut state = dirty_state(state_file);
    state.dirty = false;
    state.last_flushed_bytes = serialize_config(&state.config).unwrap();
    state
}

fn read_persisted(state_file: &std::path::Path) -> AppConfig {
    let json = std::fs::read_to_string(state_file)
        .unwrap_or_else(|e| panic!("read {}: {e}", state_file.display()));
    serde_json::from_str(&json).expect("persisted state must parse back")
}

// The debounce task runs on the test scheduler, so these use the async
// harness and hold the App borrow only for each cx.update call.
#[gpui_kit::test]
async fn update_config_persists_after_the_debounce_window(cx: &TestAppContext) {
    let _guard = debounce_test_lock();
    let dir = tempdir().unwrap();
    let state_file = dir.path().join("state.json");
    cx.update(|cx| cx.set_global(clean_state(&state_file)));

    cx.update(|cx| update_config(cx, |config| config.sidebar_collapsed = true));

    assert!(
        !state_file.exists(),
        "the write must wait for the debounce window to close"
    );
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(DEBOUNCE_MS));
    cx.executor().run_until_parked();

    let loaded = read_persisted(&state_file);
    assert!(loaded.sidebar_collapsed);
    assert!(!cx.read(|cx| cx.global::<AppState>().dirty));
}

#[gpui_kit::test]
async fn a_burst_of_updates_lands_as_the_final_value(cx: &TestAppContext) {
    let _guard = debounce_test_lock();
    let dir = tempdir().unwrap();
    let state_file = dir.path().join("state.json");
    cx.update(|cx| cx.set_global(clean_state(&state_file)));

    cx.update(|cx| update_config(cx, |config| config.sidebar_collapsed = true));
    cx.update(|cx| update_config(cx, |config| config.theme = "Gruvbox Dark".to_string()));

    assert!(
        !state_file.exists(),
        "both mutations share one debounce window"
    );
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(DEBOUNCE_MS));
    cx.executor().run_until_parked();

    let loaded = read_persisted(&state_file);
    assert!(loaded.sidebar_collapsed);
    assert_eq!(loaded.theme, "Gruvbox Dark");
}

#[gpui_kit::test]
async fn an_unchanged_update_skips_the_write_entirely(cx: &TestAppContext) {
    let _guard = debounce_test_lock();
    let dir = tempdir().unwrap();
    let state_file = dir.path().join("state.json");
    cx.update(|cx| cx.set_global(clean_state(&state_file)));

    cx.update(|cx| update_config(cx, |_config| {}));
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(DEBOUNCE_MS));
    cx.executor().run_until_parked();

    assert!(
        !state_file.exists(),
        "a state identical to the last flush must never hit the disk"
    );
    assert!(!cx.read(|cx| cx.global::<AppState>().dirty));
}

#[gpui_kit::test]
async fn update_config_repairs_an_unknown_locale_in_place(cx: &TestAppContext) {
    let _guard = debounce_test_lock();
    let dir = tempdir().unwrap();
    let state_file = dir.path().join("state.json");
    let mut state = clean_state(&state_file);
    state.config.locale = Some("klingon".to_string());
    state.last_flushed_bytes = serialize_config(&state.config).unwrap();
    cx.update(|cx| cx.set_global(state));

    cx.update(|cx| update_config(cx, |config| config.sidebar_collapsed = true));

    assert!(
        cx.read(|cx| cx.global::<AppState>().config.locale.as_deref() == Some(LOCALE_EN)),
        "the mutation path must repair an unknown locale"
    );
    // Let the armed timer fire so SAVE_SCHEDULED is released for later tests.
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(DEBOUNCE_MS));
    cx.executor().run_until_parked();

    assert_eq!(
        read_persisted(&state_file).locale.as_deref(),
        Some(LOCALE_EN)
    );
}

#[gpui_kit::test]
async fn force_save_flushes_immediately_and_cancels_the_debounce(cx: &TestAppContext) {
    let _guard = debounce_test_lock();
    let dir = tempdir().unwrap();
    let state_file = dir.path().join("state.json");
    cx.update(|cx| cx.set_global(clean_state(&state_file)));

    cx.update(|cx| update_config(cx, |config| config.theme = "Gruvbox Dark".to_string()));
    cx.update(force_save);

    let loaded = read_persisted(&state_file);
    assert_eq!(loaded.theme, "Gruvbox Dark");
    let task_cancelled = cx.read(|cx| {
        cx.try_global::<AppState>()
            .unwrap()
            .in_flight_save
            .is_none()
    });
    assert!(
        task_cancelled,
        "the debounce task must be cancelled, not left to fire after shutdown"
    );
    assert!(!SAVE_SCHEDULED.load(Ordering::Relaxed));
}

#[gpui_kit::test]
async fn a_failed_write_is_not_retried_until_the_next_mutation(cx: &TestAppContext) {
    let _guard = debounce_test_lock();
    let dir = tempdir().unwrap();
    // A regular file where the state file's parent dir would be makes every
    // write fail at ensure_parent_dir.
    let blocker = dir.path().join("blocker");
    std::fs::write(&blocker, b"not a dir").unwrap();
    let state_file = blocker.join("state.json");
    cx.update(|cx| cx.set_global(clean_state(&state_file)));

    cx.update(|cx| update_config(cx, |config| config.sidebar_collapsed = true));
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(DEBOUNCE_MS));
    cx.executor().run_until_parked();
    assert!(
        cx.read(|cx| cx.global::<AppState>().last_save_error.is_some()),
        "the blocked write must record an error"
    );

    // Unblock and wait far past another window: no retry without a mutation.
    std::fs::remove_file(&blocker).unwrap();
    std::fs::create_dir_all(&blocker).unwrap();
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(DEBOUNCE_MS * 10));
    cx.executor().run_until_parked();
    assert!(
        !state_file.exists(),
        "a failed write must not retry on its own"
    );

    cx.update(|cx| update_config(cx, |config| config.theme = "Gruvbox Dark".to_string()));
    cx.executor()
        .advance_clock(std::time::Duration::from_millis(DEBOUNCE_MS));
    cx.executor().run_until_parked();

    let loaded = read_persisted(&state_file);
    assert!(loaded.sidebar_collapsed, "the failed payload must retry");
    assert_eq!(loaded.theme, "Gruvbox Dark");
}

#[gpui_kit::test]
async fn update_config_before_initialize_warns_and_does_nothing(cx: &TestAppContext) {
    cx.update(|cx| update_config(cx, |config| config.sidebar_collapsed = true));
    assert!(
        cx.read(|cx| cx.try_global::<AppState>().is_none()),
        "a missing store must not be created by an update"
    );
    cx.update(force_save);
    assert!(cx.read(|cx| cx.try_global::<AppState>().is_none()));
}
