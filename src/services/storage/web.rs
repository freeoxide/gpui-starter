//! Wasm storage backend: SQLite on OPFS, hosted in a dedicated classic worker
//! (`wasm/sqlite/worker.js`, vendored `@sqlite.org/sqlite-wasm` engine),
//! speaking the JSON protocol from [`super::protocol`].
//!
//! OPFS sync access handles exist only inside workers, and the harness serves
//! the COOP/COEP headers the OPFS VFS requires. The trait's `#[async_trait]`
//! futures must be `Send`, but `Worker`/`Closure`/`JsValue` are not: all JS
//! state lives in a thread-local (wasm is single-threaded) and only the
//! `Send` `flume::Receiver` is held across an await.
//!
//! Degrade paths: missing Worker API/URL rejection → `boot_storage_worker`
//! returns `Err`; engine/OPFS init failure inside the worker → every request
//! answers `{"op":"error"}`; worker crash mid-session → pending requests fail
//! with the reason and later ones fail fast.

use std::cell::RefCell;
use std::collections::HashMap;

use async_trait::async_trait;
use wasm_bindgen::{JsCast, JsValue, prelude::Closure};
use web_sys::Worker;

use super::StorageBackend;
use super::StorageError;
use super::protocol::{WorkerRequest, WorkerResponse, WorkerResult};

const LOG: &str = "gpui_starter::storage::web";

/// Storage worker script URL, relative to the page — served from the wasm
/// harness statics root (`wasm/sqlite/worker.js` → `/sqlite/worker.js`).
const WORKER_URL: &str = "sqlite/worker.js";

/// Informational label for the OPFS database (the worker owns the real
/// path; snapshots only display it). Matches the flat OPFS file the worker
/// opens (`new OpfsDb("/gpui-starter-app.db")` in `wasm/sqlite/worker.js`).
pub(crate) const DB_PATH_LABEL: &str = "opfs:/gpui-starter-app.db";

type Reply = Result<WorkerResult, String>;

struct WorkerState {
    worker: Worker,
    pending: HashMap<u64, flume::Sender<Reply>>,
    next_id: u64,
    /// Set once the worker errors out; later requests fail fast with it.
    dead: Option<String>,
    // The message/error closures must live exactly as long as the worker;
    // storing them here (instead of `.forget()`-leaking) ties their
    // lifetime to the state they guard.
    _on_message: Closure<dyn FnMut(JsValue)>,
    _on_error: Closure<dyn FnMut(JsValue)>,
}

thread_local! {
    /// Thread-local because every field except `next_id` holds `!Send` JS
    /// values (see the module docs).
    static WORKER: RefCell<Option<WorkerState>> = const { RefCell::new(None) };
}

/// The wasm backend handle — carries no fields, so the trait's
/// `Send + Sync` bounds hold trivially; all state lives in [`WORKER`].
pub(crate) struct WebSqliteStorage;

/// Spawn the storage worker, install its handlers, and ask the browser for
/// persistent storage.
///
/// `Err(reason)` when the Worker API is missing or the script URL is
/// rejected — the caller degrades to the unavailable-snapshot behavior.
pub(crate) fn boot_storage_worker() -> Result<WebSqliteStorage, String> {
    install_worker()?;
    request_persistent_storage();
    Ok(WebSqliteStorage)
}

fn install_worker() -> Result<(), String> {
    if WORKER.with(|cell| cell.borrow().is_some()) {
        return Ok(());
    }

    let worker = Worker::new(WORKER_URL).map_err(|err| {
        format!(
            "failed to spawn storage worker ({WORKER_URL}): {}",
            js_value_message(&err)
        )
    })?;

    let on_message: Closure<dyn FnMut(JsValue)> = Closure::new(handle_worker_message);
    worker.set_onmessage(Some(
        on_message.as_ref().unchecked_ref::<js_sys::Function>(),
    ));
    let on_error: Closure<dyn FnMut(JsValue)> = Closure::new(handle_worker_error);
    worker.set_onerror(Some(on_error.as_ref().unchecked_ref::<js_sys::Function>()));

    WORKER.with(|cell| {
        *cell.borrow_mut() = Some(WorkerState {
            worker,
            pending: HashMap::new(),
            next_id: 1,
            dead: None,
            _on_message: on_message,
            _on_error: on_error,
        });
    });
    Ok(())
}

/// Ask `navigator.storage.persist()` to keep the OPFS database across
/// storage-pressure evictions. Best-effort: the result only decides whether
/// the browser may evict the origin's data, never correctness.
fn request_persistent_storage() {
    let Some(window) = web_sys::window() else {
        return;
    };
    // `navigator.storage` is typed non-nullable in web-sys but stays
    // undefined on old browsers — probe first (house pattern).
    let navigator = JsValue::from(window.navigator());
    let manager = match js_sys::Reflect::get(&navigator, &JsValue::from_str("storage")) {
        Ok(value) if !value.is_undefined() && !value.is_null() => value,
        _ => {
            tracing::debug!(
                target: LOG,
                "navigator.storage unavailable; skipping persist() request"
            );
            return;
        }
    };
    let manager: web_sys::StorageManager = manager.unchecked_into();
    let promise = match manager.persist() {
        Ok(promise) => promise,
        Err(err) => {
            tracing::warn!(
                target: LOG,
                error = %js_value_message(&err),
                "navigator.storage.persist() call failed"
            );
            return;
        }
    };
    let on_reject: Closure<dyn FnMut(JsValue)> = Closure::new(|err| {
        tracing::warn!(
            target: LOG,
            error = %js_value_message(&err),
            "navigator.storage.persist() rejected"
        );
    });
    let _ = promise.catch(&on_reject);
    on_reject.forget();
}

/// `None` when the worker was never spawned.
///
/// Failures (encode, post, dead worker) return a receiver that resolves to
/// `Err(reason)` instead of `None`, so callers always have one error path.
fn send_request(make_request: impl FnOnce(u64) -> WorkerRequest) -> Option<flume::Receiver<Reply>> {
    WORKER.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let state = borrow.as_mut()?;

        if let Some(reason) = state.dead.clone() {
            return Some(failed_reply(reason));
        }

        let id = state.next_id;
        state.next_id += 1;
        let request = make_request(id);
        let json = match serde_json::to_string(&request) {
            Ok(json) => json,
            Err(err) => {
                return Some(failed_reply(format!(
                    "failed to encode storage request: {err}"
                )));
            }
        };
        if let Err(err) = state.worker.post_message(&JsValue::from_str(&json)) {
            return Some(failed_reply(format!(
                "failed to post storage request: {}",
                js_value_message(&err)
            )));
        }

        let (sender, receiver) = flume::unbounded();
        state.pending.insert(id, sender);
        Some(receiver)
    })
}

fn failed_reply(reason: String) -> flume::Receiver<Reply> {
    let (sender, receiver) = flume::unbounded();
    let _ = sender.send(Err(reason));
    receiver
}

fn handle_worker_message(value: JsValue) {
    let event: web_sys::MessageEvent = value.unchecked_into();
    let Some(text) = event.data().as_string() else {
        tracing::warn!(target: LOG, "storage worker sent a non-string message");
        return;
    };
    let response: WorkerResponse = match serde_json::from_str(&text) {
        Ok(response) => response,
        Err(err) => {
            tracing::warn!(
                target: LOG,
                error = %err,
                "storage worker sent an undecodable message"
            );
            return;
        }
    };
    let (id, reply) = (response.id(), response.into_result());

    WORKER.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let Some(state) = borrow.as_mut() else {
            return;
        };
        if let Some(sender) = state.pending.remove(&id) {
            let _ = sender.send(reply);
        } else {
            tracing::warn!(
                target: LOG,
                id,
                "storage worker reply matched no pending request"
            );
        }
    });
}

/// Worker `error` event: record the reason and fail everything pending —
/// the worker is gone and will not answer.
fn handle_worker_error(value: JsValue) {
    let reason = format!("storage worker error: {}", js_value_message(&value));
    tracing::error!(target: LOG, %reason);
    WORKER.with(|cell| {
        let mut borrow = cell.borrow_mut();
        if let Some(state) = borrow.as_mut() {
            state.dead = Some(reason.clone());
            for (_, sender) in state.pending.drain() {
                let _ = sender.send(Err(reason.clone()));
            }
        }
    });
}

/// No thread-local borrow is held across the await.
async fn call(
    op: &'static str,
    make_request: impl FnOnce(u64) -> WorkerRequest,
) -> Result<WorkerResult, StorageError> {
    let Some(receiver) = send_request(make_request) else {
        return Err(StorageError::Web(
            "storage worker not running (boot failed or unavailable)".to_string(),
        ));
    };
    match receiver.recv_async().await {
        Ok(Ok(result)) => Ok(result),
        Ok(Err(message)) => Err(StorageError::Web(message)),
        // The sender is dropped on worker death with a final Err, so a
        // disconnect means the state was torn down underneath us.
        Err(_) => Err(StorageError::Web(format!(
            "storage worker dropped the reply for `{op}`"
        ))),
    }
}

fn mismatch(expected: &str, got: WorkerResult) -> StorageError {
    StorageError::Web(format!(
        "storage worker protocol mismatch: expected `{expected}`, got `{}`",
        got.op_name()
    ))
}

#[async_trait]
impl StorageBackend for WebSqliteStorage {
    async fn schema_version(&self) -> Result<i64, StorageError> {
        match call("schema-version", |id| WorkerRequest::SchemaVersion { id }).await? {
            WorkerResult::SchemaVersion(version) => Ok(version),
            other => Err(mismatch("schema-version", other)),
        }
    }

    async fn health_check(&self) -> Result<(), StorageError> {
        match call("health-check", |id| WorkerRequest::HealthCheck { id }).await? {
            WorkerResult::Unit => Ok(()),
            other => Err(mismatch("health-check", other)),
        }
    }

    async fn maintenance(&self) -> Result<(), StorageError> {
        match call("maintenance", |id| WorkerRequest::Maintenance { id }).await? {
            WorkerResult::Unit => Ok(()),
            other => Err(mismatch("maintenance", other)),
        }
    }

    async fn persist_error_record(
        &self,
        error: &crate::error_surface::ErrorRecord,
    ) -> Result<(), StorageError> {
        let record = error.clone();
        match call("persist-error-record", |id| {
            WorkerRequest::PersistErrorRecord { id, record }
        })
        .await?
        {
            WorkerResult::Unit => Ok(()),
            other => Err(mismatch("persist-error-record", other)),
        }
    }

    async fn load_error_history(
        &self,
        limit: usize,
    ) -> Result<Vec<crate::error_surface::ErrorRecord>, StorageError> {
        match call("load-error-history", |id| WorkerRequest::LoadErrorHistory {
            id,
            limit,
        })
        .await?
        {
            WorkerResult::ErrorRecords(records) => Ok(records),
            other => Err(mismatch("load-error-history", other)),
        }
    }

    async fn persist_crash_report(
        &self,
        report: &crate::services::crash_report::CrashReport,
    ) -> Result<(), StorageError> {
        let report = report.clone();
        match call("persist-crash-report", |id| {
            WorkerRequest::PersistCrashReport { id, report }
        })
        .await?
        {
            WorkerResult::Unit => Ok(()),
            other => Err(mismatch("persist-crash-report", other)),
        }
    }

    async fn load_pending_crash_reports(
        &self,
        limit: usize,
    ) -> Result<Vec<crate::services::crash_report::CrashReport>, StorageError> {
        match call("load-pending-crash-reports", |id| {
            WorkerRequest::LoadPendingCrashReports { id, limit }
        })
        .await?
        {
            WorkerResult::CrashReports(reports) => Ok(reports),
            other => Err(mismatch("load-pending-crash-reports", other)),
        }
    }

    async fn mark_crash_report_uploaded(
        &self,
        id: &str,
        uploaded_at: &str,
    ) -> Result<(), StorageError> {
        let (report_id, uploaded_at) = (id.to_string(), uploaded_at.to_string());
        match call("mark-crash-report-uploaded", |id| {
            WorkerRequest::MarkCrashReportUploaded {
                id,
                report_id,
                uploaded_at,
            }
        })
        .await?
        {
            WorkerResult::Unit => Ok(()),
            other => Err(mismatch("mark-crash-report-uploaded", other)),
        }
    }
}

fn js_value_message(value: &JsValue) -> String {
    value.as_string().unwrap_or_else(|| format!("{value:?}"))
}
