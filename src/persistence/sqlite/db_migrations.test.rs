use tempfile::tempdir;

use super::*;

#[test]
fn creates_schema_table_from_empty_db() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("test.db");
    let conn = Connection::open(&db_path).expect("open db");

    let version = run_migrations(&conn).expect("run migrations");
    assert_eq!(version, 3);

    let recorded = schema_version(&conn).expect("read version");
    assert_eq!(recorded, 3);
}

#[test]
fn is_idempotent() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("test.db");

    let conn = Connection::open(&db_path).expect("open db");
    let first = run_migrations(&conn).expect("first run");
    let second = run_migrations(&conn).expect("second run");
    assert_eq!(first, second);

    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM schema_migrations", [], |row| {
            row.get(0)
        })
        .expect("count rows");
    assert_eq!(count, 3);
}

#[test]
fn failing_migration_is_rejected_and_stays_unrecorded() {
    let dir = tempdir().expect("tempdir");
    let conn = Connection::open(dir.path().join("test.db")).expect("open db");

    let migrations = &[
        Migration {
            version: 1,
            name: "good_table",
            up_sql: "CREATE TABLE good_table (id INTEGER PRIMARY KEY);",
        },
        Migration {
            version: 2,
            name: "bad_table",
            up_sql: "INSERT INTO no_such_table (id) VALUES (1);",
        },
    ];

    let error = run_migrations_with(&conn, migrations).expect_err("failing migration");
    assert!(
        error.to_string().contains("no_such_table"),
        "the real cause must surface, got: {error}"
    );

    assert_eq!(
        schema_version(&conn).expect("read version"),
        1,
        "the migration before the failure stays recorded"
    );

    let failed: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = 2",
            [],
            |row| row.get(0),
        )
        .expect("count failed version");
    assert_eq!(failed, 0);
}

#[test]
fn unreadable_schema_version_is_not_treated_as_a_fresh_database() {
    let dir = tempdir().expect("tempdir");
    let conn = Connection::open(dir.path().join("test.db")).expect("open db");

    // A TEXT version column breaks the u32 read; swallowing that error would
    // re-run every migration against a database it failed to read.
    conn.execute_batch(
        "CREATE TABLE schema_migrations (
            version TEXT PRIMARY KEY,
            applied_at TEXT NOT NULL
         );
         INSERT INTO schema_migrations (version, applied_at) VALUES ('bad', datetime('now'));",
    )
    .expect("seed unreadable version");

    assert!(run_migrations(&conn).is_err());
}

#[test]
fn schema_newer_than_this_build_is_rejected() {
    let dir = tempdir().expect("tempdir");
    let conn = Connection::open(dir.path().join("test.db")).expect("open db");

    conn.execute_batch(
        "CREATE TABLE schema_migrations (
            version INTEGER PRIMARY KEY,
            applied_at TEXT NOT NULL
         );
         INSERT INTO schema_migrations (version, applied_at) VALUES (99, datetime('now'));",
    )
    .expect("seed newer version");

    let error = run_migrations(&conn).expect_err("newer schema must be rejected");
    assert!(error.to_string().contains("newer"), "got: {error}");

    assert_eq!(schema_version(&conn).expect("read version"), 99);
}

#[test]
fn kv_store_table_is_usable() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("test.db");
    let conn = Connection::open(&db_path).expect("open db");

    run_migrations(&conn).expect("run migrations");

    conn.execute(
        "INSERT INTO kv_store (key, value, updated_at) VALUES ('k', 'v', datetime('now'))",
        [],
    )
    .expect("insert");

    let value: String = conn
        .query_row("SELECT value FROM kv_store WHERE key = 'k'", [], |row| {
            row.get(0)
        })
        .expect("select");
    assert_eq!(value, "v");
}

#[test]
fn error_log_writer_runs_against_the_real_migration() {
    use crate::error_surface::{ErrorAction, ErrorCategory, ErrorRecord};
    use crate::ids::EventId;
    use crate::storage::{SqliteStorage, StorageBackend as _};
    use crate::time::AppTimestamp;

    let dir = tempdir().expect("tempdir");
    let conn = Connection::open(dir.path().join("test.db")).expect("open db");

    run_migrations(&conn).expect("run migrations");
    let backend = SqliteStorage::new(conn);

    let record = ErrorRecord {
        id: EventId::new(),
        occurred_at: AppTimestamp::now(),
        severity: crate::errors::AppErrorSeverity::Error,
        category: ErrorCategory::Storage,
        message: "disk full".into(),
        actions: vec![ErrorAction::Dismiss],
    };

    // The trait is async for the wasm backend; the native body resolves on
    // first poll, so a plain tokio runtime drives it (same as storage tests).
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime for test");
    runtime
        .block_on(backend.persist_error_record(&record))
        .expect("persist");
    let loaded = runtime
        .block_on(backend.load_error_history(10))
        .expect("load");

    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].id, record.id);
    assert_eq!(loaded[0].message, "disk full");
}

#[test]
fn crash_reports_table_is_usable() {
    let dir = tempdir().expect("tempdir");
    let db_path = dir.path().join("test.db");
    let conn = Connection::open(&db_path).expect("open db");

    run_migrations(&conn).expect("run migrations");

    conn.execute(
        "INSERT INTO crash_reports (id, panic_message, backtrace, app_version, os, arch, timestamp, render_path, recent_errors)
         VALUES ('crash-1', 'index out of bounds', 'frame1\nframe2', '0.2.0', 'macos', 'aarch64', '2025-06-01T12:00:00Z', 1, '[\"err1\"]')",
        [],
    )
    .expect("insert crash report");

    let msg: String = conn
        .query_row(
            "SELECT panic_message FROM crash_reports WHERE id = 'crash-1'",
            [],
            |row| row.get(0),
        )
        .expect("select");
    assert_eq!(msg, "index out of bounds");

    let render_path: bool = conn
        .query_row(
            "SELECT render_path FROM crash_reports WHERE id = 'crash-1'",
            [],
            |row| row.get(0),
        )
        .expect("select render_path");
    assert!(render_path);

    let uploaded: bool = conn
        .query_row(
            "SELECT uploaded FROM crash_reports WHERE id = 'crash-1'",
            [],
            |row| row.get(0),
        )
        .expect("select uploaded");
    assert!(!uploaded);
}
