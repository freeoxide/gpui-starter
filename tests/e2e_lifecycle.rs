#[cfg(test)]
mod tests {
    /// Smoke test for the init sequence minus the GPUI event loop.
    #[test]
    fn test_app_initializes() {
        gpui_starter::lifecycle::install_panic_hook();

        let data_dir = tempfile::tempdir().expect("tempdir");
        gpui_starter::lifecycle::set_app_data_dir(data_dir.path().to_path_buf());

        gpui_starter::lifecycle::write_crash_marker();
        let marker = gpui_starter::lifecycle::check_previous_crash();
        assert!(marker.is_some(), "crash marker should exist after write");

        gpui_starter::lifecycle::remove_crash_marker();
        assert!(
            gpui_starter::lifecycle::check_previous_crash().is_none(),
            "crash marker should be gone after removal"
        );
    }

    /// First preflight allows startup; a second preflight against its live
    /// lock does not.
    #[test]
    fn test_single_instance_enforcement() {
        // Mirrors pub(crate) RUNTIME_DIR_ENV (src/platform/process/single_instance.rs),
        // which integration tests cannot import.
        const SEAM_ENV: &str = "GPUI_STARTER_SINGLE_INSTANCE_DIR";

        // Removes the var on drop so a panic cannot leak the seam into other tests.
        struct SeamGuard;
        impl Drop for SeamGuard {
            fn drop(&mut self) {
                unsafe { std::env::remove_var(SEAM_ENV) };
            }
        }

        let sandbox = tempfile::tempdir().expect("tempdir");
        unsafe { std::env::set_var(SEAM_ENV, sandbox.path()) };
        let _seam = SeamGuard;

        let first = gpui_starter::single_instance::preflight();
        assert!(
            first.should_start,
            "first preflight should allow the app to start"
        );
        assert!(
            first.runtime.is_some(),
            "the first instance must hold the lock, or the second is not refused"
        );

        // The second preflight must hit the Busy path while the first holds the lock.
        let second = gpui_starter::single_instance::preflight();
        assert!(
            !second.should_start,
            "second preflight against a live holder must refuse to start"
        );
    }

    /// The argv scan must pick the URL out of Explorer's quoted launch
    /// shape and never mistake argv[0] (the exe path) for a link.
    #[test]
    fn test_preflight_argv_accepts_quoted_command_line() {
        let args = vec![
            "C:\\Program Files\\GPUI Starter\\gpui-starter.exe".to_string(),
            "gpui-starter://settings".to_string(),
        ];
        assert_eq!(
            gpui_starter::single_instance::initial_deep_link(&args).as_deref(),
            Some("gpui-starter://settings"),
            "quoted protocol launch must yield the URL argument"
        );

        let no_link = vec!["C:\\Program Files\\GPUI Starter\\gpui-starter.exe".to_string()];
        assert_eq!(
            gpui_starter::single_instance::initial_deep_link(&no_link),
            None,
            "a plain launch must not report a deep link"
        );
    }

    /// `normalized()` preserves valid defaults; the defaults themselves are
    /// pinned field-by-field by snapshot_tests::test_config_default_serialization.
    #[test]
    fn test_config_loads() {
        use gpui_starter::app_state::AppConfig;

        let config = AppConfig::default();

        let mut normalized = config.clone();
        normalized.normalized();
        assert_eq!(config, normalized);
    }

    /// SQLite opens, migrates, and answers queries outside the GPUI runtime.
    #[test]
    fn test_storage_initializes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db_path = dir.path().join("app.db");

        let conn = rusqlite::Connection::open(&db_path).expect("open connection");
        let version = gpui_starter::db_migrations::run_migrations(&conn).expect("run migrations");
        assert_eq!(version, 3, "migrations should bring schema to version 3");

        let one: i64 = conn
            .query_row("SELECT 1", [], |row| row.get(0))
            .expect("health check query");
        assert_eq!(one, 1);

        let stored: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |row| row.get(0),
            )
            .expect("read schema version");
        assert_eq!(stored, 3);
    }
}
