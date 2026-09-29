use tempfile::tempdir;

use super::*;
use crate::sidebar::Page;

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
    save_config(&state_file, &bytes).unwrap();
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
    std::fs::write(&state_file, vec![b' '; (MAX_STATE_FILE_BYTES + 1) as usize]).unwrap();

    let (loaded, err) = load_config(&state_file);

    assert_eq!(loaded, AppConfig::default());
    assert!(
        err.is_some(),
        "over-cap file must be reported as a load error"
    );
    assert!(!state_file.exists());
    assert!(state_file.with_extension("json.bad").exists());
}

#[test]
fn normalized_drops_implausible_window_bounds() {
    let cases = [
        PersistedWindowBounds {
            x: 0.0,
            y: 0.0,
            width: f32::NAN,
            height: 600.0,
        },
        PersistedWindowBounds {
            x: 0.0,
            y: 0.0,
            width: f32::INFINITY,
            height: 600.0,
        },
        PersistedWindowBounds {
            x: 0.0,
            y: 0.0,
            width: -800.0,
            height: 600.0,
        },
        PersistedWindowBounds {
            x: 0.0,
            y: 0.0,
            width: MAX_PLAUSIBLE_DIM * 10.0,
            height: 600.0,
        },
    ];

    for bounds in &cases {
        let config = AppConfig {
            window_bounds: Some(bounds.clone()),
            ..AppConfig::default()
        };
        assert_eq!(
            config.normalized().window_bounds,
            None,
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
        },
        PersistedWindowBounds {
            x: 0.0,
            y: 0.0,
            width: 9000.0,
            height: 600.0,
        },
    ];
    for bounds in &cases {
        let config = AppConfig {
            window_bounds: Some(bounds.clone()),
            ..AppConfig::default()
        };
        assert!(
            config.normalized().window_bounds.is_some(),
            "plausible bounds {bounds:?} must survive normalization"
        );
    }
}

// ---------------------------------------------------------------------------
// Write-path decision functions (prepare_flush / commit_flush)
// ---------------------------------------------------------------------------

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
    assert_eq!(bytes, serde_json::to_vec(&state.config).unwrap());
}

#[test]
fn prepare_flush_skips_unchanged_bytes_and_clears_dirty() {
    let dir = tempdir().unwrap();
    let state_file = dir.path().join("state.json");
    let mut state = dirty_state(&state_file);
    state.last_flushed_bytes = serde_json::to_vec(&state.config).unwrap();

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
    let bytes = serde_json::to_vec(&state.config).unwrap();

    let needs_rearm = commit_flush(&mut state, bytes.clone(), Ok(()));

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
    let stale_bytes = serde_json::to_vec(&state.config).unwrap();
    // The write raced a mutation: the flushed bytes are no longer current.
    state.config.sidebar_collapsed = !state.config.sidebar_collapsed;

    let needs_rearm = commit_flush(&mut state, stale_bytes, Ok(()));

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
    let bytes = serde_json::to_vec(&state.config).unwrap();
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
