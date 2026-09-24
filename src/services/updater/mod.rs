mod apply;
mod check;
mod download;
mod types;

use gpui::UpdateGlobal as _;

// Re-exported so external import paths stay stable.
pub use types::{CheckForUpdates, PlatformAsset, UpdateManifest, UpdateSnapshot, UpdateStatus};

pub fn initialize(cx: &mut gpui::App) {
    let channel = crate::app_state::update_channel(cx);
    cx.set_global(UpdateSnapshot {
        status: UpdateStatus::Idle,
        current_version: types::current_app_version(),
        last_check: None,
        update_channel: if channel.is_empty() {
            "stable".to_string()
        } else {
            channel
        },
        check_retry_count: 0,
        download_retry_count: 0,
        ..Default::default()
    });

    cx.on_action(|_: &CheckForUpdates, cx| {
        check::check_for_updates(cx);
    });

    // Self-update is desktop-only (binary swap on disk); wasm ships updates
    // with the page reload. The action handler above still runs either way.
    #[cfg(not(target_family = "wasm"))]
    {
        // GPUI background-executor timers instead of a spawned tokio sleep task.
        cx.spawn(async move |cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(
                    types::STARTUP_CHECK_DELAY_SECS,
                ))
                .await;
            cx.update(|cx| {
                tracing::info!(
                    target: "gpui_starter::updater",
                    "running startup update check"
                );
                check::check_for_updates(cx);
            });
        })
        .detach();

        cx.spawn(async move |cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(
                        types::PERIODIC_CHECK_INTERVAL_SECS,
                    ))
                    .await;

                let should_check: bool = cx.update(|cx| {
                    let snap = snapshot(cx);
                    matches!(
                        snap.status,
                        UpdateStatus::Idle | UpdateStatus::UpToDate | UpdateStatus::Error(_)
                    )
                });
                if should_check {
                    cx.update(|cx| {
                        tracing::info!(
                            target: "gpui_starter::updater",
                            "running periodic update check"
                        );
                        check::check_for_updates(cx);
                    });
                }
            }
        })
        .detach();
    }

    tracing::info!(
        target: "gpui_starter::updater",
        version = %env!("CARGO_PKG_VERSION"),
        "updater service initialized"
    );
}

pub fn snapshot(cx: &gpui::App) -> UpdateSnapshot {
    cx.try_global::<UpdateSnapshot>()
        .cloned()
        .unwrap_or_default()
}

pub fn check_for_updates(cx: &mut gpui::App) {
    check::check_for_updates(cx);
}

pub fn download_update(cx: &mut gpui::App) {
    download::download_update(cx);
}

pub fn apply_update(cx: &mut gpui::App) {
    apply::apply_update(cx);
}

pub fn check_pending_swap(cx: &mut gpui::App) {
    apply::check_pending_swap(cx);
}

pub fn set_channel(channel: &str, cx: &mut gpui::App) {
    let ch = if channel.is_empty() {
        "stable".to_string()
    } else {
        channel.to_string()
    };
    UpdateSnapshot::update_global(cx, |snap, _cx| {
        snap.update_channel = ch.clone();
    });
    crate::app_state::update_config(cx, |config| {
        config.update_channel = ch;
    });
    tracing::info!(
        target: "gpui_starter::updater",
        channel = %channel,
        "update channel set"
    );
}

fn set_status(status: UpdateStatus, cx: &mut gpui::App) {
    tracing::debug!(
        target: "gpui_starter::updater",
        status = ?status,
        "update status changed"
    );
    UpdateSnapshot::update_global(cx, |snap, _cx| {
        snap.status = status;
    });
}

fn reset_check_retry(cx: &mut gpui::App) {
    UpdateSnapshot::update_global(cx, |snap, _cx| {
        snap.check_retry_count = 0;
    });
}

fn reset_download_retry(cx: &mut gpui::App) {
    UpdateSnapshot::update_global(cx, |snap, _cx| {
        snap.download_retry_count = 0;
    });
}

fn notify_update_available(version: &str, cx: &mut gpui::App) {
    dispatch_background_notification(
        &format!("Update v{version} available"),
        "A new version is available. Open settings to download and install.",
        cx,
    );
}

fn notify_update_downloaded(version: &str, cx: &mut gpui::App) {
    dispatch_background_notification(
        &format!("Update v{version} ready"),
        "Update downloaded — restart to apply.",
        cx,
    );
}

/// Permanent update errors only (retries exhausted).
fn notify_update_error(cx: &mut gpui::App) {
    dispatch_background_notification(
        "Update check failed",
        "Could not check or download updates. Please try again later.",
        cx,
    );
}

/// Send a notification via the notification service without requiring a window handle.
fn dispatch_background_notification(title: &str, body: &str, cx: &mut gpui::App) {
    crate::notifications::inbox::record_attempt(
        crate::notifications::inbox::NotificationAttemptRecord {
            title: title.to_string(),
            body: body.to_string(),
            backend: crate::notifications::NotificationBackendKind::UiOnly,
            delivered_natively: false,
            degraded: false,
            error_summary: None,
            kind: crate::notifications::inbox::NotificationInboxKind::SettingsUpdate,
        },
        cx,
    );

    let state = match cx.try_global::<crate::notifications::NativeNotificationState>() {
        Some(s) => s.clone(),
        None => return,
    };
    let enabled_by_user = state.snapshot.enabled_by_user
        && state.snapshot.permission != crate::notifications::NotificationPermissionState::Denied;
    if !enabled_by_user {
        return;
    }

    let request = crate::notifications::NotificationRequest::background_worthy(title, body);
    let service = state.service.clone();

    cx.spawn(async move |cx| {
        let result = service.send(request, enabled_by_user).await;
        tracing::info!(
            target: "gpui_starter::updater",
            backend = %result.backend_used,
            delivered = result.delivered_natively,
            "update notification send completed"
        );
        cx.update(|cx| {
            crate::notifications::apply_send_result_static(&result, cx);
        });
    })
    .detach();
}

#[cfg(test)]
#[path = "../updater.test.rs"]
mod updater_test;
