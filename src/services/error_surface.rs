use gpui_kit::{App, BorrowAppContext, Global};
use serde::{Deserialize, Serialize};

use crate::{ids::EventId, time::AppTimestamp};

/// Cap on stored records (in-memory ring and sqlite retention); the newest
/// are kept.
pub(crate) const MAX_RECORDS: usize = 200;

pub use crate::errors::ErrorCategory;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ErrorAction {
    Retry,
    OpenSettings,
    Dismiss,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ErrorRecord {
    pub id: EventId,
    pub occurred_at: AppTimestamp,
    pub severity: crate::errors::AppErrorSeverity,
    pub category: ErrorCategory,
    pub message: String,
    pub actions: Vec<ErrorAction>,
}

/// In-memory error surface state (primary store).
#[derive(Clone, Debug, Default)]
pub struct ErrorSurfaceState {
    pub records: Vec<ErrorRecord>,
}

impl Global for ErrorSurfaceState {}

pub fn initialize(cx: &mut App) {
    cx.set_global(ErrorSurfaceState::default());
}

pub fn report(
    message: impl Into<String>,
    severity: crate::errors::AppErrorSeverity,
    category: ErrorCategory,
    actions: Vec<ErrorAction>,
    cx: &mut App,
) -> EventId {
    // Error text reaches the screen, the sqlite log, and crash reports — scrub
    // home paths so none of them leak the local username.
    let message = crate::crash_report::scrub_home(&message.into());
    let record = ErrorRecord {
        id: EventId::new(),
        occurred_at: AppTimestamp::now(),
        severity,
        category,
        message,
        actions,
    };
    let id = record.id;
    let record_clone = record.clone();

    // The panic handler attaches recent messages to crash reports.
    crate::lifecycle::track_recent_error(record.message.clone());

    cx.update_global::<ErrorSurfaceState, _>(|state, _cx| {
        state.records.insert(0, record);
        state.records.truncate(MAX_RECORDS);

        // The in-memory mutation stays synchronous; the INSERT is dispatched so
        // the UI thread never blocks on database I/O (native: bg executor, wasm: OPFS).
        if let Some(runtime) = _cx.try_global::<crate::storage::StorageRuntime>() {
            let backend = runtime.backend.clone();
            #[cfg(not(target_family = "wasm"))]
            _cx.background_executor()
                .spawn(async move {
                    if let Err(err) = persist_error(&*backend, &record_clone).await {
                        tracing::warn!(
                            target: "gpui_starter::error_surface",
                            error = %err,
                            "failed to persist error to sqlite"
                        );
                    }
                })
                .detach();
            #[cfg(target_family = "wasm")]
            _cx.spawn(async move |_| {
                if let Err(err) = persist_error(&*backend, &record_clone).await {
                    tracing::warn!(
                        target: "gpui_starter::error_surface",
                        error = %err,
                        "failed to persist error to opfs storage"
                    );
                }
            })
            .detach();
        }
    });

    id
}

/// Returns the number of stored error records.
pub fn record_count(cx: &App) -> usize {
    cx.try_global::<ErrorSurfaceState>()
        .map(|state| state.records.len())
        .unwrap_or_default()
}

pub fn latest(cx: &App) -> Option<ErrorRecord> {
    cx.try_global::<ErrorSurfaceState>()
        .and_then(|state| state.records.first().cloned())
}

/// Returns the message of the latest error record, without cloning the full record.
pub fn latest_message(cx: &App) -> Option<String> {
    cx.try_global::<ErrorSurfaceState>()
        .and_then(|state| state.records.first().map(|r| r.message.clone()))
}

pub fn dismiss(id: EventId, cx: &mut App) {
    cx.update_global::<ErrorSurfaceState, _>(|state, _cx| {
        state.records.retain(|r| r.id != id);
    });
}

/// Persist a single error record to the `error_log` table. If the table does
/// not exist yet this fails and the in-memory store remains authoritative.
async fn persist_error(
    db: &dyn crate::storage::StorageBackend,
    error: &ErrorRecord,
) -> Result<(), crate::storage::StorageError> {
    db.persist_error_record(error).await
}

#[cfg(test)]
#[path = "error_surface.test.rs"]
mod error_surface_test;
