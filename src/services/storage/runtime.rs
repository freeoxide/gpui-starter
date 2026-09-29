use std::sync::Arc;

use gpui_kit::{App, BorrowAppContext as _, Global};

use super::{StorageBackend, StorageSnapshot};

#[cfg(not(target_family = "wasm"))]
use super::StorageError;

#[cfg(not(target_family = "wasm"))]
use std::path::{Path, PathBuf};

/// Native: the SQLite backend (`backend::SqliteStorage`). Wasm: the
/// OPFS-worker backend (`web::WebSqliteStorage`). Both sit behind the same
/// target-neutral trait, so the global (and every consumer) is un-gated.
#[derive(Clone)]
pub struct StorageRuntime {
    pub(crate) backend: Arc<dyn StorageBackend>,
}

impl Global for StorageRuntime {}

#[cfg(not(target_family = "wasm"))]
pub fn initialize(cx: &mut App) {
    use super::backend::SqliteStorage;

    let path = db_path(cx);
    let backend = Arc::new(SqliteStorage::new(path.clone()));
    let mut snapshot = StorageSnapshot {
        db_path: path.display().to_string(),
        ..StorageSnapshot::default()
    };

    match init_db(&path) {
        Ok(schema_version) => {
            snapshot.available = true;
            snapshot.schema_version = schema_version;
            snapshot.last_migration_result =
                Some(format!("schema version {} ready", schema_version));
            tracing::info!(
                target: "gpui_starter::storage",
                db_path = %snapshot.db_path,
                schema_version,
                "storage initialized"
            );
        }
        Err(err) => {
            let error = err.to_string();
            snapshot.last_error = Some(error.clone());
            snapshot.last_migration_result = Some("migration failed".to_string());
            tracing::error!(
                target: "gpui_starter::storage",
                db_path = %snapshot.db_path,
                error = %error,
                "storage initialization failed"
            );
        }
    }

    if snapshot.available {
        // The health check runs as a spawned task and must re-derive the
        // capability from the patched snapshot: this synchronous call sees
        // `healthy: false`, and the registry has no observers to refresh it.
        let backend_for_health = Arc::clone(&backend);
        let bg = cx.background_executor().clone();
        cx.spawn(async move |cx| {
            let result = bg
                .spawn(async move { backend_for_health.health_check().await })
                .await;
            let _ = cx.update(|cx| {
                let updated = cx.update_global::<StorageSnapshot, _>(|snap, _cx| {
                    record_health_result(snap, result);
                    snap.clone()
                });
                set_capabilities(true, &updated, cx);
            });
        })
        .detach();
    }

    set_capabilities(true, &snapshot, cx);

    cx.set_global(snapshot);
    cx.set_global(StorageRuntime { backend });
}

/// Wasm: boot the OPFS sqlite worker backend (`wasm/sqlite/worker.js`).
/// Worker spawn + engine init are asynchronous, so the snapshot starts in a
/// provisional "worker starting" state and the spawned boot task upgrades it.
/// Without the Worker API or OPFS the app keeps booting on the
/// unavailable-snapshot behavior — storage is never load-bearing for boot.
#[cfg(target_family = "wasm")]
pub fn initialize(cx: &mut App) {
    let boot = super::web::boot_storage_worker();

    let (initial, runtime) = match boot {
        Ok(backend) => {
            tracing::info!(
                target: "gpui_starter::storage",
                "storage worker spawned, initializing opfs sqlite"
            );
            let initial = StorageSnapshot {
                db_path: super::web::DB_PATH_LABEL.to_string(),
                last_migration_result: Some("sqlite worker starting".to_string()),
                ..StorageSnapshot::default()
            };
            (initial, Some(Arc::new(backend) as Arc<dyn StorageBackend>))
        }
        Err(reason) => {
            tracing::warn!(
                target: "gpui_starter::storage",
                error = %reason,
                "storage degraded: opfs sqlite worker unavailable"
            );
            let initial = StorageSnapshot {
                last_migration_result: Some("opfs sqlite unavailable".to_string()),
                last_error: Some(reason),
                ..StorageSnapshot::default()
            };
            (initial, None)
        }
    };
    let supported = runtime.is_some();

    set_capabilities(supported, &initial, cx);
    cx.set_global(initial);

    if let Some(backend) = runtime {
        cx.spawn(async move |cx| {
            // The worker queues requests until its engine is ready, so the
            // first health/version round-trip doubles as the readiness
            // probe: errors here mean engine/OPFS init failed.
            let health = backend.health_check().await;
            let version = backend.schema_version().await;

            let _ = cx.update(|cx| {
                cx.update_global::<StorageSnapshot, _>(|snap, _cx| match (health, version) {
                    (Ok(()), Ok(schema_version)) => {
                        snap.available = true;
                        snap.healthy = true;
                        snap.schema_version = schema_version;
                        snap.last_error = None;
                        snap.last_migration_result = Some(format!(
                            "schema version {schema_version} ready (sqlite-wasm/opfs)"
                        ));
                        tracing::info!(
                            target: "gpui_starter::storage",
                            schema_version,
                            "opfs storage initialized"
                        );
                    }
                    (health_result, version_result) => {
                        let error = health_result
                            .err()
                            .map(|err| err.to_string())
                            .or_else(|| version_result.err().map(|err| err.to_string()))
                            .unwrap_or_else(|| "storage worker failed to initialize".to_string());
                        snap.last_error = Some(error.clone());
                        snap.last_migration_result = Some("migration failed".to_string());
                        tracing::error!(
                            target: "gpui_starter::storage",
                            error = %error,
                            "opfs storage initialization failed"
                        );
                    }
                });
                set_capabilities(true, &snapshot(cx), cx);
            });
        })
        .detach();
    }
}

/// Re-derive the capability-registry status from a snapshot. Backends settle
/// asynchronously after the synchronous boot, and the registry has no
/// observers — every snapshot update must call this again or a healthy boot
/// freezes in as `degraded: true`.
fn set_capabilities(supported: bool, snapshot: &StorageSnapshot, cx: &mut App) {
    crate::capabilities::set("storage", capability_status(supported, snapshot), cx);
}

/// Pure snapshot → capability-status derivation, unit-tested in storage.test.rs
/// (the full boot ordering needs a GPUI app context tests can't construct).
pub(super) fn capability_status(
    supported: bool,
    snapshot: &StorageSnapshot,
) -> crate::capabilities::CapabilityStatus {
    crate::capabilities::CapabilityStatus {
        supported,
        enabled: snapshot.available,
        degraded: snapshot.last_error.is_some() || !snapshot.healthy,
        reason: snapshot
            .last_error
            .as_ref()
            .map(|err| format!("storage issue: {err}").into()),
        last_error: snapshot.last_error.clone().map(Into::into),
    }
}

/// Fold a health-check outcome into a snapshot (native boot task; the wasm
/// boot task folds health + schema version together instead).
#[cfg(not(target_family = "wasm"))]
pub(super) fn record_health_result(
    snapshot: &mut StorageSnapshot,
    result: Result<(), StorageError>,
) {
    match result {
        Ok(()) => snapshot.healthy = true,
        Err(err) => {
            snapshot.healthy = false;
            snapshot.last_error = Some(err.to_string());
        }
    }
}

pub fn snapshot(cx: &App) -> StorageSnapshot {
    cx.try_global::<StorageSnapshot>()
        .cloned()
        .unwrap_or_default()
}

/// Run a health check against the storage backend without blocking the main
/// GPUI render loop.
///
/// The result is written back to the global [`StorageSnapshot`] via an
/// `cx.update` callback once the check completes. Native runs the check on
/// a background thread (synchronous SQLite I/O); wasm awaits the worker
/// reply on the foreground executor (message-passing, non-blocking).
pub fn run_health_check(cx: &mut App) {
    let Some(runtime) = cx.try_global::<StorageRuntime>().cloned() else {
        return;
    };
    #[cfg(not(target_family = "wasm"))]
    let bg = cx.background_executor().clone();
    cx.spawn(async move |cx| {
        let backend = runtime.backend.clone();
        let version_backend = Arc::clone(&backend);
        #[cfg(not(target_family = "wasm"))]
        let (result, version_result) = {
            let health_task = bg.spawn(async move { backend.health_check().await });
            let version_task = bg.spawn(async move { version_backend.schema_version().await });
            (health_task.await, version_task.await)
        };
        #[cfg(target_family = "wasm")]
        let (result, version_result) = {
            let health = backend.health_check().await;
            let version = version_backend.schema_version().await;
            (health, version)
        };

        let _ = cx.update(|cx| {
            cx.update_global::<StorageSnapshot, _>(|snap, _cx| match result {
                Ok(()) => {
                    snap.healthy = true;
                    snap.last_error = None;
                    if let Ok(version) = version_result {
                        snap.schema_version = version;
                    }
                }
                Err(err) => {
                    snap.healthy = false;
                    snap.last_error = Some(err.to_string());
                }
            });
        });
    })
    .detach();
}

/// Run storage maintenance (e.g. `PRAGMA optimize`) without blocking the
/// main GPUI render loop.
///
/// The result is written back to the global [`StorageSnapshot`] via an
/// `cx.update` callback once maintenance completes. Native runs it on a
/// background thread; wasm awaits the worker reply (see
/// [`run_health_check`]).
pub fn run_maintenance(cx: &mut App) {
    let Some(runtime) = cx.try_global::<StorageRuntime>().cloned() else {
        return;
    };
    #[cfg(not(target_family = "wasm"))]
    let bg = cx.background_executor().clone();
    cx.spawn(async move |cx| {
        let backend = runtime.backend.clone();
        #[cfg(not(target_family = "wasm"))]
        let result = bg.spawn(async move { backend.maintenance().await }).await;
        #[cfg(target_family = "wasm")]
        let result = backend.maintenance().await;

        let _ = cx.update(|cx| {
            cx.update_global::<StorageSnapshot, _>(|snap, _cx| match result {
                Ok(()) => {
                    snap.last_maintenance_at = Some(chrono::Utc::now().to_rfc3339());
                    snap.last_error = None;
                }
                Err(err) => {
                    snap.last_error = Some(err.to_string());
                }
            });
        });
    })
    .detach();
}

pub fn shutdown(cx: &mut App) {
    let snapshot = snapshot(cx);
    tracing::debug!(
        target: "gpui_starter::storage",
        available = snapshot.available,
        healthy = snapshot.healthy,
        db_path = %snapshot.db_path,
        "storage shutdown requested"
    );
}

#[cfg(not(target_family = "wasm"))]
fn db_path(cx: &App) -> PathBuf {
    crate::app_state::paths(cx).data_dir.join("app.db")
}

/// Open (or create) the database and apply migrations from `persistence` —
/// the single source of truth for the schema.
#[cfg(not(target_family = "wasm"))]
pub(crate) fn init_db(path: &Path) -> anyhow::Result<i64> {
    let conn = rusqlite::Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode = WAL;")?;
    Ok(crate::db_migrations::run_migrations(&conn)? as i64)
}
