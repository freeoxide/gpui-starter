use super::*;
use tempfile::tempdir;

#[test]
fn crash_report_new_populates_fields() {
    let report = CrashReport::new(
        "panic message".into(),
        "backtrace".into(),
        true,
        vec!["err1".into()],
    );
    assert_eq!(report.panic_message, "panic message");
    assert_eq!(report.backtrace, "backtrace");
    assert!(report.render_path);
    assert_eq!(report.recent_errors, vec!["err1"]);
    assert!(!report.id.is_empty());
    assert!(!report.app_version.is_empty());
    assert!(!report.os.is_empty());
    assert!(!report.arch.is_empty());
    assert!(!report.timestamp.is_empty());
}

#[test]
fn write_and_detect_crash_report() {
    let dir = tempdir().expect("tempdir");
    let report = CrashReport::new("test panic".into(), "fake backtrace".into(), false, vec![]);

    write_crash_report(&report, dir.path()).expect("write");

    let detected = detect_pending_reports(dir.path());
    assert_eq!(detected.len(), 1);
    assert_eq!(detected[0].id, report.id);
    assert_eq!(detected[0].panic_message, "test panic");
}

#[test]
fn detect_pending_ignores_malformed_files() {
    let dir = tempdir().expect("tempdir");
    let reports_dir = dir.path().join("crash_reports");
    std::fs::create_dir_all(&reports_dir).expect("mkdir");

    let report = CrashReport::new("good".into(), "bt".into(), false, vec![]);
    write_crash_report(&report, dir.path()).expect("write");

    let bad_path = reports_dir.join("bad.json");
    std::fs::write(&bad_path, "not json").expect("write bad");

    let detected = detect_pending_reports(dir.path());
    assert_eq!(detected.len(), 1);
    assert_eq!(detected[0].panic_message, "good");
}

#[test]
fn detect_pending_returns_empty_when_no_dir() {
    let dir = tempdir().expect("tempdir");
    let detected = detect_pending_reports(dir.path());
    assert!(detected.is_empty());
}

#[test]
fn snapshot_default_values() {
    let snap = CrashReportSnapshot::default();
    assert_eq!(snap.pending_count, 0);
    assert!(snap.last_crash_timestamp.is_none());
    assert!(snap.upload_endpoint.is_empty());
    assert!(snap.last_upload_error.is_none());
}

#[test]
fn write_crash_report_creates_directory() {
    let dir = tempdir().expect("tempdir");
    let nested = dir.path().join("does_not_exist");
    let report = CrashReport::new("msg".into(), "bt".into(), false, vec![]);
    write_crash_report(&report, &nested).expect("write should create dir");
    assert!(nested.join("crash_reports").exists());
}

#[test]
fn detect_pending_sorts_newest_first() {
    let dir = tempdir().expect("tempdir");

    // Write two reports with a small delay so timestamps differ.
    let r1 = CrashReport::new("first".into(), "bt".into(), false, vec![]);
    write_crash_report(&r1, dir.path()).expect("write");
    std::thread::sleep(std::time::Duration::from_millis(10));
    let r2 = CrashReport::new("second".into(), "bt".into(), false, vec![]);
    write_crash_report(&r2, dir.path()).expect("write");

    let detected = detect_pending_reports(dir.path());
    assert_eq!(detected.len(), 2);
    assert_eq!(detected[0].panic_message, "second");
    assert_eq!(detected[1].panic_message, "first");
}

#[test]
fn new_scrubs_home_dir_from_reported_text() {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_default();
    let report = CrashReport::new(
        format!("panicked at {home}/src/main.rs"),
        format!("frame 0: {home}/src/lib.rs"),
        false,
        vec![format!("error reading {home}/secret.txt")],
    );
    if home.is_empty() {
        return; // nothing to scrub without a home dir
    }
    assert!(!report.panic_message.contains(&home));
    assert!(!report.backtrace.contains(&home));
    assert!(!report.recent_errors[0].contains(&home));
    assert!(report.panic_message.contains("~/src/main.rs"));
}

#[test]
fn new_caps_report_sizes() {
    let long_error = "x".repeat(10_000);
    let many_errors: Vec<String> = (0..50)
        .map(|i| format!("err{i}-{}", "y".repeat(600)))
        .collect();
    let report = CrashReport::new(long_error.clone(), long_error, false, many_errors);
    assert_eq!(report.panic_message.chars().count(), 2048);
    assert_eq!(report.backtrace.chars().count(), 8192);
    assert_eq!(report.recent_errors.len(), 20);
    assert_eq!(report.recent_errors[0].chars().count(), 512);
}

#[test]
fn rfc3339_pins_known_instants() {
    assert_eq!(rfc3339(0), "1970-01-01T00:00:00.000000000+00:00");
    assert_eq!(
        rfc3339(1_000_000_000),
        "1970-01-01T00:00:01.000000000+00:00"
    );
    assert_eq!(
        rfc3339(951_825_600_000_000_000),
        "2000-02-29T12:00:00.000000000+00:00",
        "century leap day",
    );
    assert_eq!(
        rfc3339(1_709_213_445_123_456_789),
        "2024-02-29T13:30:45.123456789+00:00",
        "leap day with nanoseconds",
    );
    assert_eq!(
        rfc3339(1_793_145_599_999_999_999),
        "2026-10-27T23:59:59.999999999+00:00",
        "end of day",
    );
}

#[test]
fn report_ids_stay_unique_and_filename_safe() {
    let mut seen = std::collections::HashSet::new();
    for _ in 0..32 {
        let report = CrashReport::new("m".into(), "bt".into(), false, vec![]);
        assert!(
            report
                .id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "id is used as a filename and must stay safe: {}",
            report.id
        );
        assert!(
            seen.insert(report.id.clone()),
            "duplicate report id: {}",
            report.id
        );
    }
}

#[test]
fn hand_written_json_round_trips_through_the_serde_reader() {
    let nasty = "quote \" backslash \\ newline \n tab \t control \u{1} unicode 日本語 slash /";
    let report = CrashReport::new(
        nasty.to_string(),
        format!("bt {nasty}"),
        true,
        vec![nasty.to_string(), String::new()],
    );
    let dir = tempdir().expect("tempdir");
    write_crash_report(&report, dir.path()).expect("write");

    // The on-disk schema is whatever the serde reader accepts: readers and
    // the storage/upload paths parse reports through CrashReport's derive.
    let disk = std::fs::read_to_string(
        dir.path()
            .join("crash_reports")
            .join(format!("{}.json", report.id)),
    )
    .expect("read written report");
    let parsed: CrashReport = serde_json::from_str(&disk).expect("valid JSON, existing schema");
    assert_eq!(parsed, report);

    let detected = detect_pending_reports(dir.path());
    assert_eq!(detected.len(), 1);
    assert_eq!(detected[0], report);
}

#[test]
fn timestamps_sort_lexicographically_in_write_order() {
    let stamps: Vec<String> = (0..8)
        .map(|i| {
            let report =
                CrashReport::new(format!("ordered-{i}").into(), "bt".into(), false, vec![]);
            report.timestamp
        })
        .collect();
    let mut sorted = stamps.clone();
    sorted.sort();
    assert_eq!(
        stamps, sorted,
        "fixed-width RFC3339 must sort the way detect_pending_reports sorts"
    );
}
