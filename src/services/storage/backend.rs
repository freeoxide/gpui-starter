use std::sync::Arc;

#[cfg(test)]
use std::path::PathBuf;

use async_trait::async_trait;
use rusqlite::Connection;
use std::sync::Mutex;

use super::{StorageBackend, StorageError};

use crate::error_surface::MAX_RECORDS;

/// Uploaded crash reports older than this are pruned by maintenance.
const CRASH_REPORT_RETENTION_DAYS: i64 = 30;

/// SQLite-backed storage sharing one connection. `Connection` is `Send` but
/// not `Sync`, so all calls serialise behind a mutex; the async bodies have
/// no await points, so the `MutexGuard` never crosses a yield.
#[derive(Clone, Debug)]
pub(crate) struct SqliteStorage {
    conn: Arc<Mutex<Connection>>,
}

impl SqliteStorage {
    /// Wrap the connection `runtime::init_db` opened, configured, and
    /// migrated; every later query answers on that same connection.
    pub(crate) fn new(conn: Connection) -> Self {
        Self {
            conn: Arc::new(Mutex::new(conn)),
        }
    }

    /// Test-only: open a pragma-configured connection without migrations;
    /// callers build their own tables.
    #[cfg(test)]
    pub fn new_for_test(path: PathBuf) -> Self {
        let conn = super::runtime::open_storage_db(&path).expect("open test db");
        Self::new(conn)
    }

    fn conn(&self) -> std::sync::MutexGuard<'_, Connection> {
        // A poisoned lock means an earlier query panicked; this one should
        // still proceed (same policy as crash_report::write_crash_report).
        self.conn
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

/// Unit-variant enums serialize to a bare JSON string; the TEXT columns
/// store exactly that text, without the JSON quotes.
fn enum_text<'a>(value: &'a serde_json::Value, op: &'static str) -> Result<&'a str, StorageError> {
    value.as_str().ok_or_else(|| {
        StorageError::sqlite(
            op,
            rusqlite::Error::ToSqlConversionFailure("enum did not serialize to text".into()),
        )
    })
}

fn encode_failure(op: &'static str) -> impl Fn(serde_json::Error) -> StorageError {
    move |err| StorageError::sqlite(op, rusqlite::Error::ToSqlConversionFailure(Box::new(err)))
}

fn decode_failure(err: impl std::fmt::Display) -> StorageError {
    StorageError::Decode(err.to_string())
}

#[async_trait]
impl StorageBackend for SqliteStorage {
    async fn schema_version(&self) -> Result<i64, StorageError> {
        let conn = self.conn();
        crate::db_migrations::schema_version(&conn)
            .map(|version| version as i64)
            .map_err(|err| StorageError::sqlite("schema_version", err))
    }

    async fn health_check(&self) -> Result<(), StorageError> {
        let conn = self.conn();
        let _: i64 = conn
            .query_row("SELECT 1", [], |row| row.get(0))
            .map_err(|err| StorageError::sqlite("health_check", err))?;
        Ok(())
    }

    async fn maintenance(&self) -> Result<(), StorageError> {
        let conn = self.conn();
        conn.execute_batch("PRAGMA optimize;")
            .map_err(|err| StorageError::sqlite("maintenance", err))?;
        // The in-memory store truncates to MAX_RECORDS; without this the
        // database is the one unbounded copy of the error log.
        conn.execute(
            &format!(
                "DELETE FROM error_log WHERE id NOT IN (
                     SELECT id FROM error_log ORDER BY occurred_at DESC LIMIT {MAX_RECORDS}
                 )"
            ),
            [],
        )
        .map_err(|err| StorageError::sqlite("maintenance", err))?;
        let cutoff =
            (chrono::Utc::now() - chrono::Duration::days(CRASH_REPORT_RETENTION_DAYS)).to_rfc3339();
        conn.execute(
            "DELETE FROM crash_reports WHERE uploaded = 1 AND uploaded_at < ?1",
            [cutoff],
        )
        .map_err(|err| StorageError::sqlite("maintenance", err))?;
        Ok(())
    }

    async fn persist_error_record(
        &self,
        error: &crate::error_surface::ErrorRecord,
    ) -> Result<(), StorageError> {
        let conn = self.conn();
        let severity =
            serde_json::to_value(error.severity).map_err(encode_failure("persist_error_record"))?;
        let category =
            serde_json::to_value(error.category).map_err(encode_failure("persist_error_record"))?;
        let actions_json = serde_json::to_string(&error.actions)
            .map_err(encode_failure("persist_error_record"))?;
        conn.execute(
            "INSERT OR IGNORE INTO error_log (id, occurred_at, severity, category, message, actions)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![
                error.id.to_string(),
                error.occurred_at.to_rfc3339(),
                enum_text(&severity, "persist_error_record")?,
                enum_text(&category, "persist_error_record")?,
                error.message,
                actions_json,
            ],
        )
        .map_err(|err| StorageError::sqlite("persist_error_record", err))?;
        Ok(())
    }

    async fn load_error_history(
        &self,
        limit: usize,
    ) -> Result<Vec<crate::error_surface::ErrorRecord>, StorageError> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, occurred_at, severity, category, message, actions
                 FROM error_log
                 ORDER BY occurred_at DESC
                 LIMIT ?1",
            )
            .map_err(|err| StorageError::sqlite("load_error_history", err))?;
        let rows = stmt
            .query_map([limit], |row| {
                let id_str: String = row.get(0)?;
                let occurred_at_str: String = row.get(1)?;
                let severity_str: String = row.get(2)?;
                let category_str: String = row.get(3)?;
                let message: String = row.get(4)?;
                let actions_json: String = row.get(5)?;
                Ok((
                    id_str,
                    occurred_at_str,
                    severity_str,
                    category_str,
                    message,
                    actions_json,
                ))
            })
            .map_err(|err| StorageError::sqlite("load_error_history", err))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|err| StorageError::sqlite("load_error_history", err))?;

        let mut records = Vec::with_capacity(rows.len());
        for (id_str, occurred_at_str, severity_str, category_str, message, actions_json) in rows {
            let id = uuid::Uuid::parse_str(&id_str)
                .map(crate::ids::EventId)
                .map_err(decode_failure)?;
            let occurred_at = chrono::DateTime::parse_from_rfc3339(&occurred_at_str)
                .map(|dt| crate::time::AppTimestamp(dt.to_utc()))
                .map_err(decode_failure)?;
            let severity: crate::errors::AppErrorSeverity =
                serde_json::from_value(serde_json::Value::String(severity_str))
                    .map_err(decode_failure)?;
            let category: crate::error_surface::ErrorCategory =
                serde_json::from_value(serde_json::Value::String(category_str))
                    .map_err(decode_failure)?;
            let actions: Vec<crate::error_surface::ErrorAction> =
                serde_json::from_str(&actions_json).map_err(decode_failure)?;
            records.push(crate::error_surface::ErrorRecord {
                id,
                occurred_at,
                severity,
                category,
                message,
                actions,
            });
        }
        Ok(records)
    }

    async fn persist_crash_report(
        &self,
        report: &crate::services::crash_report::CrashReport,
    ) -> Result<(), StorageError> {
        let conn = self.conn();
        let recent_errors_json = serde_json::to_string(&report.recent_errors)
            .map_err(encode_failure("persist_crash_report"))?;
        conn.execute(
            "INSERT OR IGNORE INTO crash_reports (id, panic_message, backtrace, app_version, os, arch, timestamp, render_path, recent_errors)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                report.id,
                report.panic_message,
                report.backtrace,
                report.app_version,
                report.os,
                report.arch,
                report.timestamp,
                report.render_path,
                recent_errors_json,
            ],
        )
        .map_err(|err| StorageError::sqlite("persist_crash_report", err))?;
        Ok(())
    }

    async fn load_pending_crash_reports(
        &self,
        limit: usize,
    ) -> Result<Vec<crate::services::crash_report::CrashReport>, StorageError> {
        let conn = self.conn();
        let mut stmt = conn
            .prepare(
                "SELECT id, panic_message, backtrace, app_version, os, arch, timestamp, render_path, recent_errors
                 FROM crash_reports
                 WHERE uploaded = 0
                 ORDER BY timestamp DESC
                 LIMIT ?1",
            )
            .map_err(|err| StorageError::sqlite("load_pending_crash_reports", err))?;
        let rows = stmt
            .query_map([limit], |row| {
                let id: String = row.get(0)?;
                let panic_message: String = row.get(1)?;
                let backtrace: String = row.get(2)?;
                let app_version: String = row.get(3)?;
                let os: String = row.get(4)?;
                let arch: String = row.get(5)?;
                let timestamp: String = row.get(6)?;
                let render_path: bool = row.get(7)?;
                let recent_errors_json: String = row.get(8)?;
                Ok((
                    id,
                    panic_message,
                    backtrace,
                    app_version,
                    os,
                    arch,
                    timestamp,
                    render_path,
                    recent_errors_json,
                ))
            })
            .map_err(|err| StorageError::sqlite("load_pending_crash_reports", err))?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|err| StorageError::sqlite("load_pending_crash_reports", err))?;

        let mut reports = Vec::with_capacity(rows.len());
        for (
            id,
            panic_message,
            backtrace,
            app_version,
            os,
            arch,
            timestamp,
            render_path,
            recent_errors_json,
        ) in rows
        {
            let recent_errors: Vec<String> =
                serde_json::from_str(&recent_errors_json).map_err(decode_failure)?;
            reports.push(crate::services::crash_report::CrashReport {
                id,
                panic_message,
                backtrace,
                app_version,
                os,
                arch,
                timestamp,
                render_path,
                recent_errors,
            });
        }
        Ok(reports)
    }

    async fn mark_crash_report_uploaded(
        &self,
        id: &str,
        uploaded_at: &str,
    ) -> Result<(), StorageError> {
        let conn = self.conn();
        conn.execute(
            "UPDATE crash_reports SET uploaded = 1, uploaded_at = ?1 WHERE id = ?2",
            rusqlite::params![uploaded_at, id],
        )
        .map_err(|err| StorageError::sqlite("mark_crash_report_uploaded", err))?;
        Ok(())
    }
}
