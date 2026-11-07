use super::*;

#[test]
fn app_error_derives_category_and_severity_from_variant() {
    use crate::errors::AppError;

    let parse = AppError::StateParse {
        path: "state.json".into(),
        details: "bad json".into(),
    };
    assert_eq!(parse.category(), ErrorCategory::Config);
    assert_eq!(parse.severity(), crate::errors::AppErrorSeverity::Warning);

    let write = AppError::StateWrite {
        path: "state.json".into(),
        details: "disk full".into(),
    };
    assert_eq!(write.category(), ErrorCategory::Storage);
    assert_eq!(write.severity(), crate::errors::AppErrorSeverity::Error);

    let deep_link = AppError::invalid_deep_link("gpui-starter://", "no page");
    assert_eq!(deep_link.category(), ErrorCategory::System);

    assert_eq!(
        AppError::PathInitialization.category(),
        ErrorCategory::System
    );
    assert_eq!(
        AppError::Io(std::io::Error::other("boom")).category(),
        ErrorCategory::System
    );
}

#[test]
fn error_record_serializes_with_category() {
    let record = ErrorRecord {
        id: EventId::new(),
        occurred_at: AppTimestamp::now(),
        severity: crate::errors::AppErrorSeverity::Warning,
        category: ErrorCategory::Network,
        message: "connection timed out".into(),
        actions: vec![ErrorAction::Retry, ErrorAction::Dismiss],
    };
    let json = serde_json::to_string(&record).expect("serialize");
    assert!(json.contains("\"category\":\"network\""));
    let deserialized: ErrorRecord = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(deserialized.category, ErrorCategory::Network);
}

#[test]
fn persist_and_load_roundtrip() {
    use crate::storage::{SqliteStorage, StorageBackend as _, init_db};

    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("test_errors.db");

    // init_db runs the real migrations; hand-copied DDL here would drift.
    let (conn, _) = init_db(&db_path).expect("init db");
    let backend = SqliteStorage::new(conn);

    let record = ErrorRecord {
        id: EventId::new(),
        occurred_at: AppTimestamp::now(),
        severity: crate::errors::AppErrorSeverity::Error,
        category: ErrorCategory::Storage,
        message: "disk full".into(),
        actions: vec![ErrorAction::Dismiss],
    };

    // persist_error/load_error_history are async (the wasm backend answers
    // via worker round-trips); the native body resolves on first poll.
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime for test");
    runtime
        .block_on(persist_error(&backend, &record))
        .expect("persist");
    let loaded = runtime
        .block_on(backend.load_error_history(10))
        .expect("load");
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].message, "disk full");
    assert_eq!(loaded[0].category, ErrorCategory::Storage);
    assert_eq!(loaded[0].severity, crate::errors::AppErrorSeverity::Error);
}

#[test]
fn load_error_history_respects_limit() {
    use crate::storage::{SqliteStorage, StorageBackend as _, init_db};

    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("test_limit.db");

    let (conn, _) = init_db(&db_path).expect("init db");
    let backend = SqliteStorage::new(conn);
    let runtime = tokio::runtime::Runtime::new().expect("tokio runtime for test");

    // Fixed, strictly increasing timestamps: `now()` in a tight loop can tie,
    // and ties would leave the DESC order unpinned.
    let base = chrono::DateTime::parse_from_rfc3339("2024-01-01T00:00:00Z").expect("base time");
    for i in 0..5 {
        let record = ErrorRecord {
            id: EventId::new(),
            occurred_at: AppTimestamp((base + chrono::Duration::seconds(i)).to_utc()),
            severity: crate::errors::AppErrorSeverity::Warning,
            category: ErrorCategory::System,
            message: format!("error {i}"),
            actions: vec![],
        };
        runtime
            .block_on(persist_error(&backend, &record))
            .expect("persist");
    }

    let loaded = runtime
        .block_on(backend.load_error_history(3))
        .expect("load");
    let messages: Vec<_> = loaded
        .iter()
        .map(|record| record.message.as_str())
        .collect();
    assert_eq!(messages, ["error 4", "error 3", "error 2"]);
}
