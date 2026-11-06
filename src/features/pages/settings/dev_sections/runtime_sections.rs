use gpui_kit::component::{WindowExt as _, button::Button};
use gpui_kit::{prelude::*, *};

use crate::connectivity;
use crate::desktop_actions;
use crate::secure_storage;

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
                            cx.spawn(async move |cx| {
                                desktop_actions::pick_file(cx).await;
                            })
                            .detach();
                        }),
                )
                .child(
                    Button::new("desktop-pick-folder")
                        .outline()
                        .label(crate::i18n::localize("settings_pick_folder"))
                        .on_click(|_, _, cx| {
                            cx.spawn(async move |cx| {
                                desktop_actions::pick_folder(cx).await;
                            })
                            .detach();
                        }),
                ),
        )
        .child(
            div().flex().items_center().gap_2().child(
                Button::new("desktop-save-file")
                    .outline()
                    .label(crate::i18n::localize("settings_save_file"))
                    .on_click(|_, _, cx| {
                        cx.spawn(async move |cx| {
                            desktop_actions::save_file(cx).await;
                        })
                        .detach();
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
            div().flex().flex_wrap().items_center().gap_2().child(
                Button::new("connectivity-check-now")
                    .outline()
                    .label(crate::i18n::localize("settings_check_connectivity_now"))
                    .on_click(|_, _, cx| {
                        connectivity::check_now(cx);
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
                            #[cfg(not(target_family = "wasm"))]
                            {
                                let written =
                                    crate::i18n::localize("settings_secure_value_written");
                                let failed = crate::i18n::localize("settings_secure_write_failed");
                                spawn_keyring_demo(window, cx, move || {
                                    match secure_storage::set_secret_blocking(
                                        "gpui-starter",
                                        "demo-token",
                                        "demo-value",
                                    ) {
                                        Ok(()) => (Some(None), written),
                                        Err(err) => (
                                            Some(Some(err.to_string())),
                                            format!("{failed}: {err}"),
                                        ),
                                    }
                                });
                            }
                            #[cfg(target_family = "wasm")]
                            {
                                let message = match secure_storage::set_secret(
                                    "gpui-starter",
                                    "demo-token",
                                    "demo-value",
                                    cx,
                                ) {
                                    Ok(()) => {
                                        crate::i18n::localize("settings_secure_value_written")
                                    }
                                    Err(err) => format!(
                                        "{}: {err}",
                                        crate::i18n::localize("settings_secure_write_failed")
                                    ),
                                };
                                window.push_notification(message, cx);
                            }
                        }),
                )
                .child(
                    Button::new("secure-storage-delete-demo")
                        .outline()
                        .label(crate::i18n::localize("settings_delete_secure_value_demo"))
                        .on_click(|_, window, cx| {
                            #[cfg(not(target_family = "wasm"))]
                            {
                                let deleted =
                                    crate::i18n::localize("settings_secure_value_deleted");
                                let failed = crate::i18n::localize("settings_secure_delete_failed");
                                spawn_keyring_demo(window, cx, move || {
                                    match secure_storage::delete_secret_blocking(
                                        "gpui-starter",
                                        "demo-token",
                                    ) {
                                        Ok(()) => (Some(None), deleted),
                                        Err(err) => (
                                            Some(Some(err.to_string())),
                                            format!("{failed}: {err}"),
                                        ),
                                    }
                                });
                            }
                            #[cfg(target_family = "wasm")]
                            {
                                let message = match secure_storage::delete_secret(
                                    "gpui-starter",
                                    "demo-token",
                                    cx,
                                ) {
                                    Ok(()) => {
                                        crate::i18n::localize("settings_secure_value_deleted")
                                    }
                                    Err(err) => format!(
                                        "{}: {err}",
                                        crate::i18n::localize("settings_secure_delete_failed")
                                    ),
                                };
                                window.push_notification(message, cx);
                            }
                        }),
                ),
        )
        .child(
            Button::new("secure-storage-read-demo")
                .outline()
                .label(crate::i18n::localize("settings_read_secure_value_demo"))
                .on_click(|_, window, cx| {
                    #[cfg(not(target_family = "wasm"))]
                    {
                        let exists = crate::i18n::localize("settings_secure_value_exists");
                        let missing = crate::i18n::localize("settings_secure_value_missing");
                        let failed = crate::i18n::localize("settings_secure_read_failed");
                        spawn_keyring_demo(window, cx, move || {
                            match secure_storage::get_secret_blocking("gpui-starter", "demo-token")
                            {
                                Ok(Some(_)) => (Some(None), exists),
                                Ok(None) => (Some(None), missing),
                                // Read failures leave the stored last_error as-is.
                                Err(err) => (None, format!("{failed}: {err}")),
                            }
                        });
                    }
                    #[cfg(target_family = "wasm")]
                    {
                        let message =
                            match secure_storage::get_secret("gpui-starter", "demo-token", cx) {
                                Ok(Some(_)) => {
                                    crate::i18n::localize("settings_secure_value_exists")
                                }
                                Ok(None) => crate::i18n::localize("settings_secure_value_missing"),
                                Err(err) => format!(
                                    "{}: {err}",
                                    crate::i18n::localize("settings_secure_read_failed")
                                ),
                            };
                        window.push_notification(message, cx);
                    }
                }),
        )
}

/// Runs a blocking keyring op off the UI thread and reports from the update
/// turn; the snapshot write's outer Option means "leave last_error untouched".
#[cfg(not(target_family = "wasm"))]
fn spawn_keyring_demo(
    window: &mut Window,
    cx: &mut App,
    op: impl FnOnce() -> (Option<Option<String>>, String) + Send + 'static,
) {
    let window_handle = window.window_handle();
    let exec = cx.background_executor().clone();
    cx.spawn(async move |cx| {
        let (snapshot_write, message) = exec.spawn(async move { op() }).await;
        let _ = cx.update(|cx| {
            if let Some(last_error) = snapshot_write {
                secure_storage::update_last_error(last_error, cx);
            }
            if let Err(err) = cx.update_window(window_handle, |_, window, cx| {
                window.push_notification(message, cx);
            }) {
                tracing::warn!(
                    target: "gpui_starter::settings",
                    ?err,
                    "keyring demo feedback dropped"
                );
            }
        });
    })
    .detach();
}
