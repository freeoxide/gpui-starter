use super::*;

#[test]
fn lifecycle_stage_starts_as_starting() {
    let state = LifecycleState::default();
    assert_eq!(state.stage, LifecycleStage::Starting);
}

#[test]
fn render_panic_flag_roundtrip() {
    let _guard = hook_test_lock();
    // The flag is process-global; consume any stale value first so the
    // assertions below are deterministic even with parallel tests.
    let _ = take_render_panic();
    RENDER_PANIC_OCCURRED.store(true, Ordering::SeqCst);
    assert!(take_render_panic(), "first read should be true");
    assert!(
        !take_render_panic(),
        "second read should be false (flag reset)"
    );
}

#[test]
fn enter_render_path_sets_and_clears() {
    assert!(!in_render_path());
    {
        let _guard = enter_render_path();
        assert!(in_render_path());
    }
    assert!(!in_render_path());
}

#[test]
fn track_recent_error_keeps_limit() {
    for i in 0..25 {
        track_recent_error(format!("err-{i}"));
    }
    let slot = RECENT_ERRORS.get().unwrap();
    let guard = slot.lock().unwrap();
    assert!(guard.len() <= 20, "ring buffer must cap at 20 entries");
    assert_eq!(guard.last().map(String::as_str), Some("err-24"));
}

#[test]
fn crash_marker_roundtrip_in_data_dir() {
    let marker = test_data_dir().join("crash-marker");
    let _ = std::fs::remove_file(&marker);

    write_crash_marker();
    #[cfg(target_family = "unix")]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mode = std::fs::metadata(&marker).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "marker must stay user-only");
    }
    let contents = check_previous_crash();
    assert!(contents.is_some(), "marker should exist after write");
    assert!(
        contents.unwrap().starts_with("pid="),
        "marker should start with pid="
    );

    remove_crash_marker();
    assert!(check_previous_crash().is_none(), "marker should be removed");
    assert!(!marker.exists());
}

/// Point the once-set data dir at a shared temp dir for the marker tests.
fn test_data_dir() -> &'static PathBuf {
    static DIR: OnceLock<PathBuf> = OnceLock::new();
    DIR.get_or_init(|| {
        let dir = tempfile::tempdir().expect("tempdir").keep();
        set_app_data_dir(dir.clone());
        dir
    })
}

/// Handler tests flip process-global state; serialize them against each
/// other. Other modules' deliberate panics cannot take this lock, so the
/// writer tests retry around that documented bail window.
static HOOK_TESTS: Mutex<()> = Mutex::new(());

fn hook_test_lock() -> std::sync::MutexGuard<'static, ()> {
    HOOK_TESTS
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn reports_containing(
    dir: &std::path::Path,
    needle: &str,
) -> Vec<crate::services::crash_report::CrashReport> {
    crate::services::crash_report::detect_pending_reports(dir)
        .into_iter()
        .filter(|report| report.panic_message.contains(needle))
        .collect()
}

#[test]
fn panic_handler_writes_valid_report_without_gpui() {
    let _guard = hook_test_lock();
    let dir = test_data_dir();
    let _ = take_render_panic();

    let needle = "gpui-less writer sentinel";
    let mut written = reports_containing(dir, needle);
    for _ in 0..5 {
        if !written.is_empty() {
            break;
        }
        handle_panic(needle, false);
        written = reports_containing(dir, needle);
    }

    let report = written
        .first()
        .expect("handler must write a parseable report with no GPUI context");
    assert!(
        !report.render_path,
        "non-render payload must not flag the boundary"
    );
    assert_eq!(report.app_version, env!("CARGO_PKG_VERSION"));
    assert!(
        !take_render_panic(),
        "a non-render panic must not arm the error boundary"
    );
}

#[test]
fn nested_panic_handler_call_bails_and_writes_nothing() {
    let _guard = hook_test_lock();
    let dir = test_data_dir();

    for attempt in 0..5 {
        let needle = format!("re-entrancy sentinel {attempt}");
        HANDLING_PANIC.store(true, Ordering::SeqCst);
        handle_panic(&needle, false);
        let still_set = HANDLING_PANIC.load(Ordering::SeqCst);
        HANDLING_PANIC.store(false, Ordering::SeqCst);
        let leaked = reports_containing(dir, &needle);
        if still_set && leaked.is_empty() {
            return; // bailed: nothing written, guard not cleared by the handler
        }
        // A concurrent handler cleared our flag; retry with a fresh needle.
    }
    panic!("nested handler call must bail without writing a report");
}

#[test]
fn render_path_panic_sets_error_boundary_flag() {
    let _guard = hook_test_lock();

    for _ in 0..5 {
        let _ = take_render_panic();
        handle_panic("render-path sentinel", true);
        if take_render_panic() {
            return;
        }
    }
    panic!("render-originating payload must arm the error boundary");
}

#[test]
fn installed_hook_captures_panic_and_process_survives() {
    let _guard = hook_test_lock();
    let dir = test_data_dir();
    install_panic_hook();

    for attempt in 0..5 {
        let needle = format!("capture-and-survive sentinel {attempt}");
        let caught = std::panic::catch_unwind(|| {
            panic!("{needle}");
        });
        assert!(
            caught.is_err(),
            "catch_unwind must capture; reaching this assert is the survive half"
        );
        if !reports_containing(dir, &needle).is_empty() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("installed hook must write a report for a captured panic");
}
