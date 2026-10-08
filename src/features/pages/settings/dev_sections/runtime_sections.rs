use gpui_kit::component::{WindowExt as _, button::Button};
use gpui_kit::{prelude::*, *};

use crate::connectivity;
use crate::desktop_actions;
use crate::secure_storage;
use crate::session::{self, SessionState};

pub fn render_desktop_actions_section(
    cx: &mut Context<super::super::SettingsPage>,
) -> impl IntoElement {
    super::settings_card_base(cx)
        .child(super::section_heading(
            "settings-desktop-actions-title",
            crate::i18n::localize("settings_desktop_actions"),
        ))
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(
                    Button::new("desktop-copy-diagnostics")
                        .outline()
                        .label(crate::i18n::localize("diagnostics_copy"))
                        .on_click(|_, _, cx| {
                            let _ = desktop_actions::copy_diagnostics(cx);
                        }),
                )
                .child(
                    Button::new("desktop-open-logs")
                        .outline()
                        .label(crate::i18n::localize("diagnostics_open_logs"))
                        .on_click(|_, _, cx| {
                            let _ = desktop_actions::open_logs_folder(cx);
                        }),
                )
                .child(
                    Button::new("desktop-open-config")
                        .outline()
                        .label(crate::i18n::localize("diagnostics_open_config"))
                        .on_click(|_, _, cx| {
                            let _ = desktop_actions::open_config_folder(cx);
                        }),
                ),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(
                    Button::new("desktop-pick-file")
                        .outline()
                        .label(crate::i18n::localize("settings_pick_file"))
                        .on_click(|_, _, cx| {
                            let _ = desktop_actions::pick_file(cx);
                        }),
                )
                .child(
                    Button::new("desktop-pick-folder")
                        .outline()
                        .label(crate::i18n::localize("settings_pick_folder"))
                        .on_click(|_, _, cx| {
                            let _ = desktop_actions::pick_folder(cx);
                        }),
                ),
        )
        .child(
            div().flex().items_center().gap_2().child(
                Button::new("desktop-save-file")
                    .outline()
                    .label(crate::i18n::localize("settings_save_file"))
                    .on_click(|_, _, cx| {
                        let _ = desktop_actions::save_file(cx);
                    }),
            ),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(
                    Button::new("desktop-watch-logs")
                        .outline()
                        .label(crate::i18n::localize("settings_watch_logs_dir"))
                        .on_click(|_, _, cx| {
                            let _ = desktop_actions::watch_log_dir(cx);
                        }),
                )
                .child(
                    Button::new("desktop-watch-config")
                        .outline()
                        .label(crate::i18n::localize("settings_watch_config_dir"))
                        .on_click(|_, _, cx| {
                            let _ = desktop_actions::watch_config_dir(cx);
                        }),
                )
                .child(
                    Button::new("desktop-unwatch-all")
                        .outline()
                        .label(crate::i18n::localize("settings_unwatch_all"))
                        .on_click(|_, _, cx| {
                            let _ = desktop_actions::unwatch_all(cx);
                        }),
                )
                .child(
                    Button::new("desktop-open-support-url")
                        .outline()
                        .label(crate::i18n::localize("settings_open_support_url"))
                        .on_click(|_, _, cx| {
                            let _ = desktop_actions::open_url("https://example.com/support", cx);
                        }),
                ),
        )
}

pub fn render_runtime_boundaries_section(
    cx: &mut Context<super::super::SettingsPage>,
) -> impl IntoElement {
    super::settings_card_base(cx)
        .child(super::section_heading(
            "settings-runtime-boundaries-title",
            crate::i18n::localize("settings_runtime_boundaries"),
        ))
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(
                    Button::new("connectivity-check-now")
                        .outline()
                        .label(crate::i18n::localize("settings_check_connectivity_now"))
                        .on_click(|_, _, cx| {
                            connectivity::check_now(cx);
                        }),
                )
                .child(
                    Button::new("session-sign-in")
                        .outline()
                        .label(crate::i18n::localize("settings_session_sign_in_demo"))
                        .on_click(|_, _, cx| {
                            session::set_state(SessionState::SigningIn, cx);
                            session::set_state(
                                SessionState::SignedIn {
                                    account_label: "demo-user".to_string(),
                                },
                                cx,
                            );
                        }),
                )
                .child(
                    Button::new("session-sign-out")
                        .outline()
                        .label(crate::i18n::localize("settings_session_sign_out"))
                        .on_click(|_, _, cx| {
                            session::set_state(SessionState::SignedOut, cx);
                        }),
                )
                .child(
                    Button::new("session-error-demo")
                        .outline()
                        .label(crate::i18n::localize("settings_session_error_demo"))
                        .on_click(|_, _, cx| {
                            session::set_state(
                                SessionState::Error("demo session error".to_string()),
                                cx,
                            );
                        }),
                ),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(
                    Button::new("secure-storage-write-demo")
                        .outline()
                        .label(crate::i18n::localize("settings_write_secure_value_demo"))
                        .on_click(|_, window, cx| {
                            let message = match secure_storage::set_secret(
                                "gpui-starter",
                                "demo-token",
                                "demo-value",
                                cx,
                            ) {
                                Ok(()) => crate::i18n::localize("settings_secure_value_written"),
                                Err(err) => format!(
                                    "{}: {err}",
                                    crate::i18n::localize("settings_secure_write_failed")
                                ),
                            };
                            window.push_notification(message, cx);
                        }),
                )
                .child(
                    Button::new("secure-storage-delete-demo")
                        .outline()
                        .label(crate::i18n::localize("settings_delete_secure_value_demo"))
                        .on_click(|_, window, cx| {
                            let message = match secure_storage::delete_secret(
                                "gpui-starter",
                                "demo-token",
                                cx,
                            ) {
                                Ok(()) => crate::i18n::localize("settings_secure_value_deleted"),
                                Err(err) => format!(
                                    "{}: {err}",
                                    crate::i18n::localize("settings_secure_delete_failed")
                                ),
                            };
                            window.push_notification(message, cx);
                        }),
                ),
        )
        .child(
            Button::new("secure-storage-read-demo")
                .outline()
                .label(crate::i18n::localize("settings_read_secure_value_demo"))
                .on_click(|_, window, cx| {
                    let message = match secure_storage::get_secret("gpui-starter", "demo-token", cx)
                    {
                        Ok(Some(_)) => crate::i18n::localize("settings_secure_value_exists"),
                        Ok(None) => crate::i18n::localize("settings_secure_value_missing"),
                        Err(err) => format!(
                            "{}: {err}",
                            crate::i18n::localize("settings_secure_read_failed")
                        ),
                    };
                    window.push_notification(message, cx);
                }),
        )
}
