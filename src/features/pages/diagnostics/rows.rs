use gpui_kit::*;

use crate::{
    accessibility, app_state, capabilities, commands, connectivity, crash_report, desktop_actions,
    error_surface,
    lifecycle::{LifecycleStage, LifecycleState},
    logging, notifications, secure_storage, session, shortcuts, storage, telemetry, undo_stack,
};

use super::{capability_row, row};

pub fn build_diagnostic_rows(cx: &App) -> Vec<Stateful<Div>> {
    let lifecycle = cx
        .try_global::<LifecycleState>()
        .cloned()
        .unwrap_or_else(LifecycleState::starting);
    let notifications = notifications::snapshot(cx);
    let connectivity = connectivity::snapshot(cx);
    let secure_storage = secure_storage::snapshot(cx);
    let session = session::snapshot(cx);
    let logging = logging::snapshot(cx);
    let storage = storage::snapshot(cx);
    let telemetry = telemetry::snapshot(cx);
    let accessibility = accessibility::snapshot(cx);
    let capabilities = capabilities::snapshot(cx);
    let shortcuts = shortcuts::snapshot(cx);
    let desktop_actions = desktop_actions::snapshot(cx);
    let undo = undo_stack::snapshot(cx);
    let latest_error = error_surface::latest(cx);
    let crash_snap = crash_report::snapshot(cx);

    let none_label = crate::i18n::localize("diagnostics_none");
    let command_registry = commands::registry();
    let mut command_titles = Vec::with_capacity(command_registry.len());
    let mut command_states = Vec::with_capacity(command_registry.len());
    for command in &command_registry {
        command_titles.push(command.title.to_string());
        let availability = commands::availability(command.id, cx);
        let reason = availability
            .disabled_reason
            .map(|value| value.to_string())
            .unwrap_or_else(|| crate::i18n::localize("diagnostics_no_reason"));
        command_states.push(format!(
            "{}: enabled={} reason={}",
            command.title, availability.enabled, reason
        ));
    }
    let command_titles = command_titles.join(", ");
    let command_states = command_states.join(" | ");

    let (undo_last_label, undo_last_timestamp) = undo
        .past
        .last()
        .map(|entry| (entry.label.clone(), entry.created_at.to_rfc3339()))
        .unwrap_or_else(|| (none_label.clone(), none_label.clone()));

    let lifecycle_label = crate::i18n::localize(match lifecycle.stage {
        LifecycleStage::Starting => "diagnostics_lifecycle_starting",
        LifecycleStage::Running => "diagnostics_lifecycle_running",
        LifecycleStage::ShuttingDown => "diagnostics_lifecycle_shutting_down",
        LifecycleStage::Crashed => "diagnostics_lifecycle_crashed",
    });
    let permission_label = match &notifications.permission {
        notifications::NotificationPermissionState::Unknown => {
            crate::i18n::localize("diagnostics_permission_unknown")
        }
        notifications::NotificationPermissionState::Unsupported => {
            crate::i18n::localize("diagnostics_permission_unsupported")
        }
        notifications::NotificationPermissionState::Unavailable(reason) => format!(
            "{}: {reason}",
            crate::i18n::localize("diagnostics_permission_unavailable")
        ),
        notifications::NotificationPermissionState::NotDetermined => {
            crate::i18n::localize("diagnostics_permission_not_determined")
        }
        notifications::NotificationPermissionState::Denied => {
            crate::i18n::localize("diagnostics_permission_denied")
        }
        notifications::NotificationPermissionState::Authorized => {
            crate::i18n::localize("diagnostics_permission_authorized")
        }
    };
    let lifecycle_panic_summary =
        crate::lifecycle::last_panic_summary().unwrap_or(none_label.clone());

    let mut rows: Vec<Stateful<Div>> = vec![
        row("diagnostics_app", env!("CARGO_PKG_NAME")),
        row("diagnostics_version", env!("CARGO_PKG_VERSION")),
        row("diagnostics_lifecycle", &lifecycle_label),
        row(
            "diagnostics_lifecycle_startup_step",
            lifecycle
                .startup_step
                .as_deref()
                .unwrap_or(none_label.as_str()),
        ),
        row(
            "diagnostics_lifecycle_shutdown_step",
            lifecycle
                .shutdown_step
                .as_deref()
                .unwrap_or(none_label.as_str()),
        ),
        row(
            "diagnostics_lifecycle_startup_error",
            lifecycle
                .last_startup_error
                .as_deref()
                .unwrap_or(none_label.as_str()),
        ),
        row(
            "diagnostics_lifecycle_shutdown_error",
            lifecycle
                .last_shutdown_error
                .as_deref()
                .unwrap_or(none_label.as_str()),
        ),
        row(
            "diagnostics_lifecycle_panic_summary",
            &lifecycle_panic_summary,
        ),
        row(
            "diagnostics_notification_backend",
            &notifications.active_backend.to_string(),
        ),
        row("diagnostics_notification_permission", &permission_label),
        row(
            "diagnostics_notification_degraded",
            notifications
                .degraded_reason
                .as_deref()
                .unwrap_or(&yes_no(false)),
        ),
        row(
            "diagnostics_connectivity",
            &format!("{:?}", connectivity.state),
        ),
        row(
            "diagnostics_connectivity_probe_url",
            &connectivity.probe_url,
        ),
        row(
            "diagnostics_connectivity_last_error",
            connectivity
                .last_error
                .as_deref()
                .unwrap_or(none_label.as_str()),
        ),
        row(
            "diagnostics_secure_storage_available",
            &yes_no(secure_storage.available),
        ),
        row(
            "diagnostics_secure_storage_error",
            secure_storage
                .last_error
                .as_deref()
                .unwrap_or(none_label.as_str()),
        ),
        row("diagnostics_session", &format!("{:?}", session.state)),
        row("diagnostics_commands", &command_registry.len().to_string()),
        row("diagnostics_command_titles", &command_titles),
        row("diagnostics_command_availability", &command_states),
        row(
            "diagnostics_first_run_pending",
            &yes_no(crate::first_run::is_pending(cx)),
        ),
        row("diagnostics_logging_enabled", &yes_no(logging.enabled)),
        row(
            "diagnostics_logging_guard_active",
            &yes_no(logging.has_guard),
        ),
        row(
            "diagnostics_logging_error",
            logging.last_error.as_deref().unwrap_or(none_label.as_str()),
        ),
        row("diagnostics_storage_available", &yes_no(storage.available)),
        row("diagnostics_storage_healthy", &yes_no(storage.healthy)),
        row("diagnostics_storage_db_path", &storage.db_path),
        row(
            "diagnostics_storage_schema_version",
            &storage.schema_version.to_string(),
        ),
        row(
            "diagnostics_storage_last_maintenance",
            storage
                .last_maintenance_at
                .as_deref()
                .unwrap_or(none_label.as_str()),
        ),
        row(
            "diagnostics_storage_last_migration",
            storage
                .last_migration_result
                .as_deref()
                .unwrap_or(none_label.as_str()),
        ),
        row(
            "diagnostics_storage_error",
            storage.last_error.as_deref().unwrap_or(none_label.as_str()),
        ),
        row(
            "diagnostics_telemetry_compiled",
            &yes_no(telemetry.compiled),
        ),
        row(
            "diagnostics_telemetry_consented",
            &yes_no(telemetry.consented),
        ),
        row("diagnostics_telemetry_enabled", &yes_no(telemetry.enabled)),
        row(
            "diagnostics_telemetry_mode",
            &format!("{:?}", telemetry.mode),
        ),
        row(
            "diagnostics_telemetry_endpoint",
            telemetry
                .endpoint_redacted
                .as_deref()
                .unwrap_or(none_label.as_str()),
        ),
        row(
            "diagnostics_telemetry_error",
            telemetry
                .last_error
                .as_deref()
                .unwrap_or(none_label.as_str()),
        ),
        row(
            "diagnostics_telemetry_export_error",
            telemetry
                .last_export_error
                .as_deref()
                .unwrap_or(none_label.as_str()),
        ),
        row(
            "diagnostics_telemetry_events_recorded",
            &telemetry.events_recorded.to_string(),
        ),
        row(
            "diagnostics_accessibility_accesskit_linked",
            &yes_no(accessibility.accesskit_linked),
        ),
        row(
            "diagnostics_accessibility_bridge_enabled",
            &yes_no(accessibility.bridge_enabled),
        ),
        row("diagnostics_accessibility_status", &accessibility.status),
        row(
            "diagnostics_desktop_clipboard_available",
            &yes_no(desktop_actions.clipboard_available),
        ),
        row(
            "diagnostics_desktop_picker_available",
            &yes_no(desktop_actions.picker_available),
        ),
        row(
            "diagnostics_desktop_opener_available",
            &yes_no(desktop_actions.opener_available),
        ),
        row(
            "diagnostics_desktop_active_watchers",
            &desktop_actions.active_watchers.to_string(),
        ),
        row(
            "diagnostics_desktop_last_error",
            desktop_actions
                .last_error
                .as_deref()
                .unwrap_or(none_label.as_str()),
        ),
        row("diagnostics_undo_stack_size", &undo.past.len().to_string()),
        row(
            "diagnostics_redo_stack_size",
            &undo.future.len().to_string(),
        ),
        row("diagnostics_undo_last_label", &undo_last_label),
        row("diagnostics_undo_last_timestamp", &undo_last_timestamp),
        row(
            "diagnostics_undo_last_rejected",
            undo.last_rejected.as_deref().unwrap_or(none_label.as_str()),
        ),
        row(
            "diagnostics_shortcut_enabled_config",
            &yes_no(shortcuts.enabled),
        ),
        row(
            "diagnostics_shortcut_registered",
            &yes_no(shortcuts.registered),
        ),
        row("diagnostics_shortcut_accelerator", &shortcuts.accelerator),
        row(
            "diagnostics_shortcut_error",
            shortcuts
                .last_error
                .as_deref()
                .unwrap_or(none_label.as_str()),
        ),
        row(
            "diagnostics_error_surface_count",
            &error_surface::record_count(cx).to_string(),
        ),
        row(
            "diagnostics_latest_error",
            latest_error
                .as_ref()
                .map(|error| error.message.as_str())
                .unwrap_or(none_label.as_str()),
        ),
        row(
            "diagnostics_crash_reports_pending",
            &crash_snap.pending_count.to_string(),
        ),
        row(
            "diagnostics_crash_reports_last_timestamp",
            crash_snap
                .last_crash_timestamp
                .as_deref()
                .unwrap_or(none_label.as_str()),
        ),
        row(
            "diagnostics_crash_reports_upload_endpoint",
            if crash_snap.upload_endpoint.is_empty() {
                none_label.as_str()
            } else {
                &crash_snap.upload_endpoint
            },
        ),
        row(
            "diagnostics_crash_reports_last_upload_error",
            crash_snap
                .last_upload_error
                .as_deref()
                .unwrap_or(none_label.as_str()),
        ),
    ];

    if app_state::config_handle(cx).is_some() {
        let paths = app_state::paths(cx);
        let fallback_log_dir = paths.log_dir.display().to_string();
        let display_log_dir = if logging.log_dir.is_empty() {
            fallback_log_dir.as_str()
        } else {
            logging.log_dir.as_str()
        };
        rows.push(row(
            "diagnostics_config_dir",
            &paths.config_dir.display().to_string(),
        ));
        rows.push(row(
            "diagnostics_data_dir",
            &paths.data_dir.display().to_string(),
        ));
        rows.push(row(
            "diagnostics_cache_dir",
            &paths.cache_dir.display().to_string(),
        ));
        rows.push(row("diagnostics_log_dir", display_log_dir));
        rows.push(row("diagnostics_log_file_prefix", &logging.file_prefix));
        rows.push(row(
            "diagnostics_state_file",
            &paths.state_file.display().to_string(),
        ));
        rows.push(row(
            "diagnostics_active_route",
            &app_state::with_config(cx, |config| config.active_route.to_url().to_string()),
        ));
        rows.push(row(
            "diagnostics_config_version",
            &app_state::with_config(cx, |config| config.version.to_string()),
        ));
        rows.push(row(
            "diagnostics_state_load_error",
            app_state::load_error(cx)
                .as_deref()
                .unwrap_or(none_label.as_str()),
        ));
        rows.push(row(
            "diagnostics_state_save_error",
            app_state::save_error(cx)
                .as_deref()
                .unwrap_or(none_label.as_str()),
        ));
    }

    for (name, status) in capabilities {
        let value = format!(
            "supported={} enabled={} degraded={} reason={} error={}",
            status.supported,
            status.enabled,
            status.degraded,
            status.reason.as_deref().unwrap_or("-"),
            status.last_error.as_deref().unwrap_or("-")
        );
        rows.push(capability_row(&name, &value));
    }

    rows
}

fn yes_no(value: bool) -> String {
    crate::i18n::localize(if value {
        "diagnostics_yes"
    } else {
        "diagnostics_no"
    })
}
