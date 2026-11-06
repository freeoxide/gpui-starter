mod apply;
mod check;
mod types;

use gpui_kit::UpdateGlobal as _;

// Re-exported so external import paths stay stable.
pub use types::{PlatformAsset, UpdateManifest, UpdateSnapshot, UpdateStatus};

// The loops only exist behind #[cfg(not(target_family = "wasm"))] in
// initialize, so the pure decision helper is dead code on wasm.
#[cfg_attr(target_family = "wasm", allow(dead_code))]
fn auto_checks_enabled(update_check_on_startup: bool, manifest_url: &str) -> bool {
    update_check_on_startup && manifest_url != types::PLACEHOLDER_MANIFEST_URL
}

pub fn initialize(cx: &mut gpui_kit::App) {
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
        ..Default::default()
    });

    // Self-update is desktop-only (binary swap on disk); wasm ships updates
    // with the page reload.
    #[cfg(not(target_family = "wasm"))]
    {
        // A placeholder build endpoint must never auto-poll (RCA §7.5); the
        // config opt-in alone is not enough to arm the loops.
        if auto_checks_enabled(
            crate::app_state::with_config(cx, |config| config.update_check_on_startup),
            types::DEFAULT_MANIFEST_URL,
        ) {
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
        } else {
            tracing::info!(
                target: "gpui_starter::updater",
                "automatic update checks disabled (no opt-in or placeholder manifest url)"
            );
        }
    }

    tracing::info!(
        target: "gpui_starter::updater",
        version = %env!("CARGO_PKG_VERSION"),
        "updater service initialized"
    );
}

pub fn snapshot(cx: &gpui_kit::App) -> UpdateSnapshot {
    cx.try_global::<UpdateSnapshot>()
        .cloned()
        .unwrap_or_default()
}

pub fn check_pending_swap(cx: &mut gpui_kit::App) {
    apply::check_pending_swap(cx);
}

fn set_status(status: UpdateStatus, cx: &mut gpui_kit::App) {
    tracing::debug!(
        target: "gpui_starter::updater",
        status = ?status,
        "update status changed"
    );
    UpdateSnapshot::update_global(cx, |snap, _cx| {
        snap.status = status;
    });
}

fn reset_check_retry(cx: &mut gpui_kit::App) {
    UpdateSnapshot::update_global(cx, |snap, _cx| {
        snap.check_retry_count = 0;
    });
}

fn notify_update_available(version: &str, cx: &mut gpui_kit::App) {
    dispatch_background_notification(
        &format!(
            "{} v{version} {}",
            crate::i18n::localize("updates_title_prefix"),
            crate::i18n::localize("updates_available_suffix")
        ),
        &crate::i18n::localize("updates_available_body"),
        cx,
    );
}

/// Permanent update errors only (retries exhausted).
fn notify_update_error(cx: &mut gpui_kit::App) {
    dispatch_background_notification(
        &crate::i18n::localize("updates_error_title"),
        &crate::i18n::localize("updates_error_body"),
        cx,
    );
}

/// Send a notification via the notification service without requiring a window handle.
fn dispatch_background_notification(title: &str, body: &str, cx: &mut gpui_kit::App) {
    crate::notifications::inbox::record_attempt(
        crate::notifications::inbox::NotificationAttemptRecord {
            title: title.to_string(),
            body: body.to_string(),
            backend: crate::notifications::NotificationBackendKind::UiOnly,
            error_summary: None,
        },
        cx,
    );

    let state = match cx.try_global::<crate::notifications::NativeNotificationState>() {
        Some(state) => state,
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
