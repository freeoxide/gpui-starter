use gpui_kit::component::{
    ActiveTheme as _, Disableable as _,
    button::{Button, ButtonVariants as _},
    label::Label,
    switch::Switch,
};
use gpui_kit::{prelude::*, *};

use crate::accessibility::A11yExt as _;
use crate::notifications::{
    self, NotificationPermissionState, NotificationRequest, NotificationRuntimeSnapshot,
};

/// `id` keys the row; the label is localized and would re-key the row on every
/// locale switch. The visible texts are plain strings, so the combined text is
/// the accessible label.
pub(super) fn status_row(
    id: &'static str,
    label: impl Into<SharedString>,
    value: impl Into<SharedString>,
) -> impl IntoElement {
    let label = label.into();
    let value = value.into();
    let text = format!("{label}: {value}");
    div()
        .id(ElementId::Name(SharedString::from(format!(
            "settings-status-{id}"
        ))))
        .a11y(Role::Paragraph, text)
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .child(Label::new(label))
        .child(div().text_sm().child(value))
}

pub(super) fn render_notifications_section(
    notifications_snapshot: &NotificationRuntimeSnapshot,
    cx: &mut Context<super::SettingsPage>,
) -> impl IntoElement {
    let can_request_permission = notifications_snapshot.capabilities.can_request_permission
        && matches!(
            notifications_snapshot.permission,
            NotificationPermissionState::NotDetermined
                | NotificationPermissionState::Unknown
                | NotificationPermissionState::Unavailable(_)
        );
    // Linux and Windows have no per-app permission model, so the button routes
    // to the OS notification settings; on macOS only once permission is denied.
    let can_open_settings = cfg!(target_os = "linux")
        || cfg!(target_os = "windows")
        || (cfg!(target_os = "macos")
            && matches!(
                notifications_snapshot.permission,
                NotificationPermissionState::Denied | NotificationPermissionState::Unavailable(_)
            ));

    let notifications_snapshot = notifications_snapshot.clone();
    let enabled_label = crate::i18n::localize("settings_native_notifications", None);

    super::dev_sections::settings_card_base(cx)
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .child(Label::new(enabled_label.clone()))
                .child(
                    Switch::new("native-notifications-enabled")
                        .accessibility_label(enabled_label)
                        .checked(notifications_snapshot.enabled_by_user)
                        .on_click(|checked, _, cx| {
                            notifications::set_native_notifications_enabled(*checked, cx);
                        }),
                ),
        )
        .child(status_row(
            "backend",
            crate::i18n::localize("settings_native_backend", None),
            notifications_snapshot.active_backend.to_string(),
        ))
        .child(status_row(
            "permission",
            crate::i18n::localize("settings_permission", None),
            notifications_snapshot.permission.label(),
        ))
        .when_some(
            notifications_snapshot.degraded_reason.clone(),
            |this, reason| {
                this.child(status_row(
                    "degraded",
                    crate::i18n::localize("settings_degraded", None),
                    reason,
                ))
            },
        )
        .when_some(
            notifications_snapshot.last_backend_error.clone(),
            |this, error| this.child(status_row("backend-error", "Last backend error", error)),
        )
        .when_some(
            notifications_snapshot.daemon_capabilities.clone(),
            |this, caps| {
                this.child(status_row(
                    "daemon-capabilities",
                    "Daemon capabilities",
                    caps,
                ))
            },
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(
                    Button::new("test-native-notification")
                        .primary()
                        .label(crate::i18n::localize(
                            "settings_test_native_notification",
                            None,
                        ))
                        .on_click(|_, window, cx| {
                            notifications::send_from_window(
                                NotificationRequest::test_notification(
                                    crate::i18n::localize(
                                        "settings_test_native_notification",
                                        None,
                                    ),
                                    crate::i18n::localize("settings_hello_notification", None),
                                ),
                                window,
                                cx,
                            );
                        }),
                )
                .child(
                    Button::new("request-notification-permission")
                        .outline()
                        .disabled(!can_request_permission)
                        .label(crate::i18n::localize("settings_request_permission", None))
                        .on_click(|_, window, cx| {
                            notifications::request_permission_from_window(window, cx);
                        }),
                )
                .child(
                    Button::new("open-notification-settings")
                        .outline()
                        .disabled(!can_open_settings)
                        .label(crate::i18n::localize(
                            "settings_open_notification_settings",
                            None,
                        ))
                        .on_click(|_, _, cx| {
                            notifications::open_system_settings(cx);
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
                    Button::new("test-action-notification")
                        .outline()
                        .label(crate::i18n::localize(
                            "settings_test_action_notification",
                            None,
                        ))
                        .on_click(|_, window, cx| {
                            notifications::send_from_window(
                                NotificationRequest::action_buttons(
                                    crate::i18n::localize(
                                        "settings_test_action_notification",
                                        None,
                                    ),
                                    crate::i18n::localize(
                                        "settings_action_notification_body",
                                        None,
                                    ),
                                ),
                                window,
                                cx,
                            );
                        }),
                )
                .child(
                    Button::new("test-reply-notification")
                        .outline()
                        .label(crate::i18n::localize(
                            "settings_test_reply_notification",
                            None,
                        ))
                        .on_click(|_, window, cx| {
                            notifications::send_from_window(
                                NotificationRequest::reply(
                                    crate::i18n::localize("settings_test_reply_notification", None),
                                    crate::i18n::localize("settings_reply_notification_body", None),
                                ),
                                window,
                                cx,
                            );
                        }),
                )
                .child(
                    Button::new("test-background-worthy-notification")
                        .outline()
                        .label(crate::i18n::localize(
                            "settings_test_background_notification",
                            None,
                        ))
                        .on_click(|_, window, cx| {
                            notifications::send_from_window(
                                NotificationRequest::background_worthy(
                                    crate::i18n::localize(
                                        "settings_test_background_notification",
                                        None,
                                    ),
                                    crate::i18n::localize(
                                        "settings_background_notification_body",
                                        None,
                                    ),
                                ),
                                window,
                                cx,
                            );
                        }),
                ),
        )
        .child(
            div()
                .id("settings-in-app-note")
                .a11y(
                    Role::Paragraph,
                    crate::i18n::localize("settings_in_app_notifications_note", None),
                )
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(crate::i18n::localize(
                    "settings_in_app_notifications_note",
                    None,
                )),
        )
        .child(
            div()
                .id("settings-push-note")
                .a11y(
                    Role::Paragraph,
                    crate::i18n::localize("settings_push_notifications_note", None),
                )
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(crate::i18n::localize(
                    "settings_push_notifications_note",
                    None,
                )),
        )
}
