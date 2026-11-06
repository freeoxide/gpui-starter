use tempfile::tempdir;

use super::runtime::{capability_status, record_health_result};
use super::{SqliteStorage, StorageBackend, StorageError, StorageSnapshot, init_db};

/// The trait is async (wasm answers via worker round-trips); the native
/// bodies resolve on their first poll, so a plain tokio runtime drives the
/// futures in tests (same pattern as the notification-backend tests).
fn block_on<F: std::future::Future>(future: F) -> F::Output {
    tokio::runtime::Runtime::new()
        .expect("tokio runtime for test")
        .block_on(future)
}

#[test]
fn initializes_schema_and_migration_table() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("app.db");
    let (conn, version) = init_db(&db_path).expect("init db");
    assert_eq!(version, 3);

    // Both pragmas live on the connection the backend keeps, not on a
    // throwaway one: busy_timeout is per-connection, WAL is file-level.
    let busy_timeout: i64 = conn
        .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
        .expect("busy_timeout pragma");
    assert_eq!(busy_timeout, 5000);
    let journal_mode: String = conn
        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
        .expect("journal_mode pragma");
    assert_eq!(journal_mode, "wal");

    let verifier = rusqlite::Connection::open(&db_path).expect("open db");
    let count: i64 = verifier
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = 3",
            [],
            |row| row.get(0),
        )
        .expect("read migrations");
    assert_eq!(count, 1);
}

#[test]
fn init_db_degrades_on_an_unopenable_database() {
    let dir = tempdir().expect("tempdir");
    // A directory at the db path cannot be opened as a database.
    let db_path = dir.path().join("app.db");
    std::fs::create_dir(&db_path).expect("make db path a directory");
    assert!(init_db(&db_path).is_err());
}

#[test]
fn backend_health_and_maintenance_work() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("app.db");
    let (conn, _) = init_db(&db_path).expect("init db");
    let backend = SqliteStorage::new(conn);
    block_on(backend.health_check()).expect("health check");
    block_on(backend.maintenance()).expect("maintenance");
    assert_eq!(
        block_on(backend.schema_version()).expect("schema version"),
        3
    );
}

#[test]
fn persist_and_load_crash_report_roundtrip() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("test_crash.db");
    let (conn, _) = init_db(&db_path).expect("init db");
    let backend = SqliteStorage::new(conn);

    let report = crate::services::crash_report::CrashReport::new(
        "test panic".to_string(),
        "backtrace here".to_string(),
        true,
        vec!["error1".to_string(), "error2".to_string()],
    );

    block_on(backend.persist_crash_report(&report)).expect("persist crash report");

    let loaded = block_on(backend.load_pending_crash_reports(10)).expect("load pending");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, report.id);
    assert_eq!(loaded[0].panic_message, "test panic");
    assert_eq!(loaded[0].backtrace, "backtrace here");
    assert!(loaded[0].render_path);
    assert_eq!(loaded[0].recent_errors, vec!["error1", "error2"]);
}

#[test]
fn mark_crash_report_uploaded() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("test_upload.db");
    let (conn, _) = init_db(&db_path).expect("init db");
    let backend = SqliteStorage::new(conn);

    let report = crate::services::crash_report::CrashReport::new(
        "panic".to_string(),
        "bt".to_string(),
        false,
        vec![],
    );

    block_on(backend.persist_crash_report(&report)).expect("persist");
    let pending = block_on(backend.load_pending_crash_reports(10)).expect("load");
    assert_eq!(pending.len(), 1);

    block_on(backend.mark_crash_report_uploaded(&report.id, "2025-01-01T00:00:00Z"))
        .expect("mark uploaded");

    let pending_after = block_on(backend.load_pending_crash_reports(10)).expect("load after");
    assert_eq!(pending_after.len(), 0);
}

#[test]
fn load_pending_crash_reports_respects_limit() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("test_limit.db");
    let (conn, _) = init_db(&db_path).expect("init db");
    let backend = SqliteStorage::new(conn);

    for _ in 0..5 {
        let report = crate::services::crash_report::CrashReport::new(
            "panic".to_string(),
            "bt".to_string(),
            false,
            vec![],
        );
        block_on(backend.persist_crash_report(&report)).expect("persist");
    }

    let loaded = block_on(backend.load_pending_crash_reports(3)).expect("load");
    assert_eq!(loaded.len(), 3);
}

/// Regression guard: maintenance is the only writer that bounds error_log;
/// without the DELETE the table grows forever while the in-memory store
/// truncates to MAX_RECORDS.
#[test]
fn maintenance_bounds_error_log_to_max_records() {
    let dir = tempdir().expect("tempdir");
    let (conn, _) = init_db(&dir.path().join("retention.db")).expect("init db");
    conn.execute(
        "INSERT INTO error_log (id, occurred_at, severity, category, message, actions)
         VALUES (?1, '2020-01-01T00:00:00Z', 'Error', 'storage', 'oldest', '[]')",
        [uuid::Uuid::new_v4().to_string()],
    )
    .expect("seed oldest row");
    for _ in 0..crate::error_surface::MAX_RECORDS {
        conn.execute(
            "INSERT INTO error_log (id, occurred_at, severity, category, message, actions)
             VALUES (?1, '2021-01-01T00:00:00Z', 'Error', 'storage', 'later', '[]')",
            [uuid::Uuid::new_v4().to_string()],
        )
        .expect("seed row");
    }

    let backend = SqliteStorage::new(conn);
    block_on(backend.maintenance()).expect("maintenance");

    let loaded = block_on(backend.load_error_history(1000)).expect("load");
    assert_eq!(loaded.len(), crate::error_surface::MAX_RECORDS);
    assert!(loaded.iter().all(|record| record.message != "oldest"));
}

#[test]
fn maintenance_purges_uploaded_crash_reports_past_retention() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("crash_retention.db");
    let (conn, _) = init_db(&db_path).expect("init db");
    let stale = crate::services::crash_report::CrashReport::new(
        "old panic".to_string(),
        "bt".to_string(),
        false,
        vec![],
    );
    let fresh = crate::services::crash_report::CrashReport::new(
        "new panic".to_string(),
        "bt".to_string(),
        false,
        vec![],
    );
    conn.execute(
        "INSERT INTO crash_reports (id, panic_message, backtrace, app_version, os, arch, timestamp, render_path, recent_errors, uploaded, uploaded_at)
         VALUES (?1, ?2, 'bt', 'test', 'test', 'test', '2020-01-01T00:00:00Z', 0, '[]', 1, '2020-01-02T00:00:00Z')",
        rusqlite::params![stale.id, stale.panic_message],
    )
    .expect("seed uploaded stale report");
    conn.execute(
        "INSERT INTO crash_reports (id, panic_message, backtrace, app_version, os, arch, timestamp, render_path, recent_errors)
         VALUES (?1, ?2, 'bt', 'test', 'test', 'test', '2020-01-01T00:00:00Z', 0, '[]')",
        rusqlite::params![fresh.id, fresh.panic_message],
    )
    .expect("seed pending report");

    let backend = SqliteStorage::new(conn);
    block_on(backend.maintenance()).expect("maintenance");

    // The uploaded row is gone entirely; the pending one survives its age.
    let verifier = rusqlite::Connection::open(&db_path).expect("open verifier");
    let remaining: i64 = verifier
        .query_row("SELECT COUNT(*) FROM crash_reports", [], |row| row.get(0))
        .expect("count crash reports");
    assert_eq!(remaining, 1);
    let pending = block_on(backend.load_pending_crash_reports(10)).expect("load pending");
    assert_eq!(pending.len(), 1);
    assert_eq!(pending[0].id, fresh.id);
}

#[test]
fn load_error_history_reports_decode_failures() {
    let dir = tempdir().expect("tempdir");
    let (conn, _) = init_db(&dir.path().join("decode.db")).expect("init db");
    conn.execute(
        "INSERT INTO error_log (id, occurred_at, severity, category, message, actions)
         VALUES ('not-a-uuid', '2025-01-01T00:00:00Z', 'Error', 'storage', 'bad row', '[]')",
        [],
    )
    .expect("seed undecodable row");
    let backend = SqliteStorage::new(conn);

    let err = block_on(backend.load_error_history(10)).expect_err("row must fail to decode");
    assert!(matches!(err, StorageError::Decode(_)));
}

/// Regression guard: the boot health check lands after the synchronous
/// capability set, so every snapshot update must re-derive the status or a
/// healthy native boot freezes in as `degraded: true`. The full boot needs a
/// GPUI app context, so this replays the initialize sequence over the helpers.
#[test]
fn native_boot_capability_not_degraded_after_health_check_lands() {
    // What native `initialize` constructs after a successful `init_db`:
    // available, schema recorded, health check not yet run.
    let mut snapshot = StorageSnapshot {
        db_path: "/data/app.db".to_string(),
        available: true,
        schema_version: 3,
        last_migration_result: Some("schema version 3 ready".to_string()),
        ..StorageSnapshot::default()
    };

    let provisional = capability_status(true, &snapshot);
    assert!(provisional.degraded);

    record_health_result(&mut snapshot, Ok(()));
    let status = capability_status(true, &snapshot);
    assert!(status.supported);
    assert!(status.enabled);
    assert!(!status.degraded, "healthy native boot must not be degraded");
    assert_eq!(status.reason, None);
    assert_eq!(status.last_error, None);

    record_health_result(
        &mut snapshot,
        Err(StorageError::sqlite(
            "health_check",
            rusqlite::Error::InvalidQuery,
        )),
    );
    let degraded = capability_status(true, &snapshot);
    assert!(degraded.degraded);
    assert!(degraded.reason.is_some());
    assert!(degraded.last_error.is_some());
}
