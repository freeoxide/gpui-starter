#[cfg(not(target_family = "wasm"))]
use std::sync::Arc;

use super::types::*;
use gpui_kit::{App, UpdateGlobal as _};

pub fn check_for_updates(cx: &mut App) {
    let current = super::snapshot(cx);
    if matches!(current.status, UpdateStatus::Checking) {
        tracing::debug!(
            target: "gpui_starter::updater",
            status = ?current.status,
            "skipping check — update operation already in progress"
        );
        return;
    }

    let connectivity = crate::connectivity::snapshot(cx);
    if connectivity.state != crate::connectivity::ConnectivityState::Online {
        tracing::warn!(
            target: "gpui_starter::updater",
            connectivity = ?connectivity.state,
            "skipping update check — not online"
        );
        handle_check_failure("no network connectivity".to_string(), cx);
        return;
    }

    // Wasm: self-update is a desktop-only concept (binary swap on disk).
    // Resolve every check to UpToDate instead of fetching a manifest.
    #[cfg(target_family = "wasm")]
    {
        tracing::debug!(
            target: "gpui_starter::updater",
            "update check skipped on wasm; reporting up-to-date"
        );
        super::reset_check_retry(cx);
        super::set_status(UpdateStatus::UpToDate, cx);
        return;
    }

    #[cfg(not(target_family = "wasm"))]
    super::set_status(UpdateStatus::Checking, cx);

    // The runtime global is documented as optional; degrade instead of
    // panicking when init could not install it.
    #[cfg(not(target_family = "wasm"))]
    let Some((rt, client)) = crate::services::tokio_runtime::runtime_and_client(cx) else {
        handle_check_failure("tokio runtime unavailable".to_string(), cx);
        return;
    };
    #[cfg(not(target_family = "wasm"))]
    let current_version = current.current_version;

    // A cached manifest is fresh for one periodic check interval (matching the
    // re-check cadence in mod.rs); within that window skip the network fetch.
    #[cfg(not(target_family = "wasm"))]
    let cached_manifest = current.cached_manifest;
    #[cfg(not(target_family = "wasm"))]
    let cache_fresh = cached_manifest.is_some()
        && current
            .last_check
            .as_ref()
            .and_then(|ts| chrono::DateTime::parse_from_rfc3339(ts).ok())
            .map(|checked_at| {
                let age = chrono::Utc::now()
                    .signed_duration_since(checked_at.with_timezone(&chrono::Utc));
                age.num_seconds() < PERIODIC_CHECK_INTERVAL_SECS as i64
            })
            .unwrap_or(false);

    #[cfg(not(target_family = "wasm"))]
    cx.spawn(async move |cx| {
        let manifest_result: Result<UpdateManifest, String> = match cached_manifest {
            Some(manifest) if cache_fresh => Ok(manifest),
            _ => fetch_manifest(rt, client).await,
        };

        cx.update(move |cx| match manifest_result {
            Ok(manifest) => {
                tracing::info!(
                    target: "gpui_starter::updater",
                    manifest_version = %manifest.version,
                    current_version = %current_version,
                    cache_hit = cache_fresh,
                    "fetched update manifest"
                );

                // Cache the manifest so the next check inside one interval
                // skips the fetch; re-storing on a cache hit is idempotent.
                UpdateSnapshot::update_global(cx, |snap, _cx| {
                    snap.cached_manifest = Some(manifest.clone());
                });

                // Stamp last_check only when the network was actually hit.
                if !cache_fresh {
                    let now = chrono::Utc::now().to_rfc3339();
                    UpdateSnapshot::update_global(cx, |snap, _cx| {
                        snap.last_check = Some(now.clone());
                    });
                    crate::app_state::update_config(cx, |config| {
                        config.last_update_check = Some(now);
                    });
                }

                match (
                    semver::Version::parse(&manifest.version),
                    semver::Version::parse(&current_version),
                ) {
                    (Ok(manifest_ver), Ok(cur_ver)) => {
                        super::reset_check_retry(cx);
                        if manifest_ver > cur_ver {
                            let status = UpdateStatus::Available {
                                version: manifest.version.clone(),
                                notes: manifest.release_notes.clone(),
                            };
                            super::set_status(status, cx);
                            super::notify_update_available(&manifest.version, cx);
                        } else {
                            // Equal or older manifest — up to date (or ahead).
                            super::set_status(UpdateStatus::UpToDate, cx);
                        }
                    }
                    (Err(e), _) | (_, Err(e)) => {
                        tracing::error!(
                            target: "gpui_starter::updater",
                            error = %e,
                            "failed to parse version for comparison"
                        );
                        super::set_status(
                            UpdateStatus::Error(format!("version parse error: {e}")),
                            cx,
                        );
                    }
                }
            }
            Err(err) => {
                tracing::error!(
                    target: "gpui_starter::updater",
                    error = %err,
                    "update check failed"
                );
                handle_check_failure(err, cx);
            }
        });
    })
    .detach();
}

/// Returns false when retries are exhausted — the caller owns the terminal
/// error path and the interim status.
pub(super) fn schedule_retry<R>(cx: &mut App, retry_kind: &'static str, rearm: R) -> bool
where
    R: FnOnce(&mut App) + Send + 'static,
{
    let retry_count = cx.global_mut::<UpdateSnapshot>().check_retry_count;
    if retry_count >= MAX_UPDATE_RETRIES {
        return false;
    }
    let new_count = retry_count + 1;
    let delay_secs = RETRY_BASE_DELAY_SECS * 2u64.pow(new_count - 1);
    tracing::warn!(
        target: "gpui_starter::updater",
        kind = retry_kind,
        retry = new_count,
        max = MAX_UPDATE_RETRIES,
        delay_secs,
        "update operation failed — scheduling retry"
    );
    UpdateSnapshot::update_global(cx, |snap, _cx| {
        snap.check_retry_count = new_count;
    });

    cx.spawn(async move |cx| {
        cx.background_executor()
            .timer(std::time::Duration::from_secs(delay_secs))
            .await;
        cx.update(|cx| {
            tracing::info!(
                target: "gpui_starter::updater",
                kind = retry_kind,
                retry = new_count,
                "retrying update operation"
            );
            rearm(cx);
        });
    })
    .detach();
    true
}

fn handle_check_failure(error: String, cx: &mut App) {
    let scheduled = schedule_retry(cx, "check", check_for_updates);
    super::set_status(UpdateStatus::Error(error.clone()), cx);
    if !scheduled {
        tracing::error!(
            target: "gpui_starter::updater",
            error = %error,
            "update check failed — retries exhausted, setting permanent error"
        );
        super::notify_update_error(cx);
    }
}

#[cfg(not(target_family = "wasm"))]
pub(crate) async fn fetch_manifest(
    rt: Arc<tokio::runtime::Runtime>,
    client: reqwest::Client,
) -> Result<UpdateManifest, String> {
    let response = rt
        .spawn(async move {
            client
                .get(DEFAULT_MANIFEST_URL)
                .timeout(std::time::Duration::from_secs(30))
                .send()
                .await
        })
        .await
        .map_err(|e| format!("manifest request panicked: {e}"))?
        .map_err(|e| format!("manifest request failed: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("manifest returned status {}", response.status()));
    }

    let body = response
        .bytes()
        .await
        .map_err(|e| format!("failed to read manifest body: {e}"))?;

    serde_json::from_slice::<UpdateManifest>(&body)
        .map_err(|e| format!("failed to parse manifest: {e}"))
}
