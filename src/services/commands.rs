use gpui_kit::component::{IconName, ThemeMode};
use gpui_kit::{App, SharedString};
use serde::{Deserialize, Serialize};

use crate::{
    events::{self, AppEventKind},
    routes::AppRoute,
    sidebar::Page,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CommandId {
    OpenHome,
    OpenForm,
    OpenSettings,
    OpenNotifications,
    OpenDiagnostics,
    OpenAbout,
    ThemeLight,
    ThemeDark,
    StartDemoTask,
    CheckConnectivity,
    CopyDiagnostics,
    OpenLogsFolder,
    OpenConfigFolder,
    Undo,
    Redo,
    Restart,
}

#[derive(Clone, Debug)]
pub struct CommandAvailability {
    pub enabled: bool,
    pub disabled_reason: Option<SharedString>,
}

#[derive(Clone)]
pub struct CommandSpec {
    pub id: CommandId,
    pub title: SharedString,
    pub subtitle: SharedString,
    pub icon: IconName,
}

pub fn availability(id: CommandId, cx: &App) -> CommandAvailability {
    let desktop = crate::desktop_actions::snapshot(cx);
    match id {
        CommandId::OpenHome
        | CommandId::OpenForm
        | CommandId::OpenSettings
        | CommandId::OpenNotifications
        | CommandId::OpenDiagnostics
        | CommandId::OpenAbout
        | CommandId::ThemeLight
        | CommandId::ThemeDark
        | CommandId::StartDemoTask => CommandAvailability {
            enabled: true,
            disabled_reason: None,
        },
        CommandId::CheckConnectivity => CommandAvailability {
            enabled: true,
            disabled_reason: None,
        },
        CommandId::CopyDiagnostics => CommandAvailability {
            enabled: desktop.clipboard_available,
            disabled_reason: disabled_reason(
                "command_reason_clipboard_unavailable",
                !desktop.clipboard_available,
            ),
        },
        CommandId::OpenLogsFolder | CommandId::OpenConfigFolder => CommandAvailability {
            enabled: desktop.opener_available,
            disabled_reason: disabled_reason(
                "command_reason_opener_unavailable",
                !desktop.opener_available,
            ),
        },
        CommandId::Undo => {
            let undo = crate::undo_stack::can_undo(cx);
            CommandAvailability {
                enabled: undo.is_some(),
                disabled_reason: disabled_reason("command_reason_no_undo", undo.is_none()),
            }
        }
        CommandId::Redo => {
            let redo = crate::undo_stack::can_redo(cx);
            CommandAvailability {
                enabled: redo.is_some(),
                disabled_reason: disabled_reason("command_reason_no_redo", redo.is_none()),
            }
        }
        CommandId::Restart => CommandAvailability {
            enabled: cfg!(any(unix, windows)),
            disabled_reason: disabled_reason(
                "command_reason_restart_unavailable",
                !cfg!(any(unix, windows)),
            ),
        },
    }
}

pub fn registry() -> Vec<CommandSpec> {
    vec![
        command(
            CommandId::OpenHome,
            "command_home",
            "command_home_subtitle",
            IconName::Inbox,
        ),
        command(
            CommandId::OpenForm,
            "command_form",
            "command_form_subtitle",
            IconName::File,
        ),
        command(
            CommandId::OpenSettings,
            "command_settings",
            "command_settings_subtitle",
            IconName::Settings2,
        ),
        command(
            CommandId::OpenNotifications,
            "command_notifications",
            "command_notifications_subtitle",
            IconName::Bell,
        ),
        command(
            CommandId::OpenDiagnostics,
            "command_diagnostics",
            "command_diagnostics_subtitle",
            IconName::Info,
        ),
        command(
            CommandId::OpenAbout,
            "command_about",
            "command_about_subtitle",
            IconName::Info,
        ),
        command(
            CommandId::ThemeLight,
            "command_light_mode",
            "command_light_mode_subtitle",
            IconName::Sun,
        ),
        command(
            CommandId::ThemeDark,
            "command_dark_mode",
            "command_dark_mode_subtitle",
            IconName::Moon,
        ),
        command(
            CommandId::StartDemoTask,
            "command_start_demo_task",
            "command_start_demo_task_subtitle",
            IconName::Play,
        ),
        command(
            CommandId::CheckConnectivity,
            "command_check_connectivity",
            "command_check_connectivity_subtitle",
            IconName::Globe,
        ),
        command(
            CommandId::CopyDiagnostics,
            "command_copy_diagnostics",
            "command_copy_diagnostics_subtitle",
            IconName::Info,
        ),
        command(
            CommandId::OpenLogsFolder,
            "command_open_logs_folder",
            "command_open_logs_folder_subtitle",
            IconName::Info,
        ),
        command(
            CommandId::OpenConfigFolder,
            "command_open_config_folder",
            "command_open_config_folder_subtitle",
            IconName::Info,
        ),
        command(
            CommandId::Undo,
            "command_undo",
            "command_undo_subtitle",
            IconName::Info,
        ),
        command(
            CommandId::Redo,
            "command_redo",
            "command_redo_subtitle",
            IconName::Info,
        ),
        command(
            CommandId::Restart,
            "command_restart",
            "command_restart_subtitle",
            IconName::Info,
        ),
    ]
}

pub fn execute(id: CommandId, cx: &mut App) {
    match id {
        CommandId::OpenHome => navigate(Page::Home, cx),
        CommandId::OpenForm => navigate(Page::Form, cx),
        CommandId::OpenSettings => navigate(Page::Settings, cx),
        CommandId::OpenNotifications => navigate(Page::Notifications, cx),
        CommandId::OpenDiagnostics => navigate(Page::Diagnostics, cx),
        CommandId::OpenAbout => navigate(Page::About, cx),
        CommandId::ThemeLight => crate::app::set_theme_mode(ThemeMode::Light, cx),
        CommandId::ThemeDark => crate::app::set_theme_mode(ThemeMode::Dark, cx),
        CommandId::StartDemoTask => crate::tasks::start_demo_task(cx),
        CommandId::CheckConnectivity => crate::connectivity::check_now(cx),
        CommandId::CopyDiagnostics => {
            if let Err(error) = crate::desktop_actions::copy_diagnostics(cx) {
                tracing::warn!(target: "gpui_starter::commands", %error, "copy diagnostics failed");
                report_error(
                    format!(
                        "{}: {error}",
                        crate::i18n::localize("command_copy_diagnostics_failed")
                    ),
                    crate::error_surface::ErrorCategory::System,
                    vec![
                        crate::error_surface::ErrorAction::Retry,
                        crate::error_surface::ErrorAction::Dismiss,
                    ],
                    cx,
                );
            }
        }
        CommandId::OpenLogsFolder => {
            if let Err(error) = crate::desktop_actions::open_logs_folder(cx) {
                tracing::warn!(target: "gpui_starter::commands", %error, "open logs folder failed");
                report_error(
                    format!(
                        "{}: {error}",
                        crate::i18n::localize("command_open_logs_failed")
                    ),
                    crate::error_surface::ErrorCategory::Storage,
                    vec![
                        crate::error_surface::ErrorAction::OpenSettings,
                        crate::error_surface::ErrorAction::Dismiss,
                    ],
                    cx,
                );
            }
        }
        CommandId::OpenConfigFolder => {
            if let Err(error) = crate::desktop_actions::open_config_folder(cx) {
                tracing::warn!(target: "gpui_starter::commands", %error, "open config folder failed");
                report_error(
                    format!(
                        "{}: {error}",
                        crate::i18n::localize("command_open_config_failed")
                    ),
                    crate::error_surface::ErrorCategory::Config,
                    vec![
                        crate::error_surface::ErrorAction::OpenSettings,
                        crate::error_surface::ErrorAction::Dismiss,
                    ],
                    cx,
                );
            }
        }
        CommandId::Undo => {
            let _ = crate::undo_stack::undo(cx);
        }
        CommandId::Redo => {
            let _ = crate::undo_stack::redo(cx);
        }
        CommandId::Restart => {
            #[cfg(any(unix, windows))]
            cx.dispatch_action(&crate::app::Restart);
            #[cfg(not(any(unix, windows)))]
            {
                let _ = cx;
                tracing::warn!(
                    target: "gpui_starter::commands",
                    "restart command unavailable on this platform"
                );
            }
        }
    }
}

fn navigate(page: Page, cx: &mut App) {
    events::emit(AppEventKind::Navigate(AppRoute::page(page)), cx);
}

fn report_error(
    message: impl Into<String>,
    category: crate::error_surface::ErrorCategory,
    actions: Vec<crate::error_surface::ErrorAction>,
    cx: &mut App,
) {
    crate::error_surface::report(
        message,
        crate::errors::AppErrorSeverity::Error,
        category,
        actions,
        cx,
    );
}

fn command(id: CommandId, title_key: &str, subtitle_key: &str, icon: IconName) -> CommandSpec {
    CommandSpec {
        id,
        title: crate::i18n::localize(title_key).into(),
        subtitle: crate::i18n::localize(subtitle_key).into(),
        icon,
    }
}

fn disabled_reason(key: &str, disabled: bool) -> Option<SharedString> {
    disabled.then(|| crate::i18n::localize(key).into())
}
