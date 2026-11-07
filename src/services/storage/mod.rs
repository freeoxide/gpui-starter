//! Native: [`backend::SqliteStorage`] holds one shared `rusqlite` connection
//! behind a mutex. Wasm: [`web::WebSqliteStorage`] talks to the OPFS sqlite
//! worker over the JSON protocol in [`protocol`], falling back to the
//! unavailable-snapshot behavior when OPFS or the Worker API is missing.
//! The trait is async ([`async_trait`]) because the wasm backend can only
//! answer via worker round-trips; native bodies stay synchronous internally.
#[cfg(not(target_family = "wasm"))]
mod backend;
// Target-neutral so the protocol tests run natively; the wasm-only web
// backend consumes it, and the test cfg keeps native builds warning-free.
#[cfg(any(target_family = "wasm", test))]
pub(crate) mod protocol;
mod runtime;
#[cfg(target_family = "wasm")]
mod web;

#[cfg(test)]
#[cfg(not(target_family = "wasm"))]
pub(crate) use backend::SqliteStorage;
pub use runtime::*;

use async_trait::async_trait;
use gpui_kit::Global;

#[derive(Clone, Debug, Default)]
pub struct StorageSnapshot {
    pub available: bool,
    pub db_path: String,
    pub schema_version: i64,
    pub healthy: bool,
    pub last_maintenance_at: Option<String>,
    pub last_migration_result: Option<String>,
    pub last_error: Option<String>,
}

impl Global for StorageSnapshot {}

/// Target-neutral storage error. `rusqlite::Error` exists only natively, so
/// that variant is compiled out on wasm; the web backend reports through
/// [`StorageError::Web`]. Callers format the variants the same way.
#[derive(Debug)]
pub enum StorageError {
    /// The named storage operation failed; `op` rides along so logs and
    /// surfaced records name the call site that failed.
    #[cfg(not(target_family = "wasm"))]
    Sqlite {
        op: &'static str,
        source: rusqlite::Error,
    },
    /// A stored row failed to decode (id, timestamp, or JSON-encoded field).
    Decode(String),
    /// Web backend failure: worker transport, OPFS, or worker-side SQL
    /// error text.
    Web(String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(not(target_family = "wasm"))]
            Self::Sqlite { op, source } => write!(f, "{op}: {source}"),
            Self::Decode(message) => write!(f, "decode failed: {message}"),
            Self::Web(message) => write!(f, "{message}"),
        }
    }
}

impl std::error::Error for StorageError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            #[cfg(not(target_family = "wasm"))]
            Self::Sqlite { source, .. } => Some(source),
            Self::Decode(_) | Self::Web(_) => None,
        }
    }
}

#[cfg(not(target_family = "wasm"))]
impl StorageError {
    /// Attach the failing operation name; every sqlite error carries one.
    pub(crate) fn sqlite(op: &'static str, source: rusqlite::Error) -> Self {
        Self::Sqlite { op, source }
    }
}

// Nightly clippy flags async_trait's macro-emitted wrapper fns as
// double_must_use; the allow must live at the macro use site.
#[allow(clippy::double_must_use)]
#[async_trait]
pub trait StorageBackend: Send + Sync {
    async fn schema_version(&self) -> Result<i64, StorageError>;
    async fn health_check(&self) -> Result<(), StorageError>;
    async fn maintenance(&self) -> Result<(), StorageError>;
    async fn persist_error_record(
        &self,
        error: &crate::error_surface::ErrorRecord,
    ) -> Result<(), StorageError>;
    async fn load_error_history(
        &self,
        limit: usize,
    ) -> Result<Vec<crate::error_surface::ErrorRecord>, StorageError>;
    async fn persist_crash_report(
        &self,
        report: &crate::services::crash_report::CrashReport,
    ) -> Result<(), StorageError>;
    async fn load_pending_crash_reports(
        &self,
        limit: usize,
    ) -> Result<Vec<crate::services::crash_report::CrashReport>, StorageError>;
    async fn mark_crash_report_uploaded(
        &self,
        id: &str,
        uploaded_at: &str,
    ) -> Result<(), StorageError>;
}

#[cfg(test)]
#[path = "../storage.test.rs"]
mod storage_test;
