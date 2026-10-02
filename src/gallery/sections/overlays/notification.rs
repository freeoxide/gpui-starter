//! Notification section, ported from the upstream `NotificationStory`.

use gpui_kit::component::{
    ActiveTheme as _, Theme, WindowExt as _,
    button::{Button, ButtonVariants as _, DropdownButton},
    h_flex,
    menu::PopupMenuItem,
    notification::{Notification, NotificationType},
    text::markdown,
    v_flex,
};
use gpui_kit::*;

use crate::gallery::registry::GallerySection;

use super::demo::{demo_toolbar, section};

const NOTIFICATION_MARKDOWN: &str = r#"
This is a custom notification.
- List item 1
- List item 2
- [Click here](https://github.com/longbridge/gpui-kit)
"#;

const ANCHORS: [Anchor; 8] = [
    Anchor::TopLeft,
    Anchor::TopCenter,
    Anchor::TopRight,
    Anchor::LeftCenter,
    Anchor::RightCenter,
    Anchor::BottomLeft,
    Anchor::BottomCenter,
    Anchor::BottomRight,
];

const MAX_ITEMS: [usize; 5] = [1, 2, 3, 5, 10];

pub struct NotificationSection;

impl NotificationSection {
    pub fn view(_window: &mut Window, _cx: &mut App) -> Entity<Self> {
        _cx.new(|_| Self)
    }
}

impl Render for NotificationSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let view = cx.entity();

        v_flex()
            .w_full()
            .gap_3()
            .p_4()
            .child(demo_toolbar(vec![
                DropdownButton::new("notification-placement")
                    .button(
                        Button::new(SharedString::from("notification-placement-trigger")).label(
                            format!("Placement: {:?}", cx.theme().notification.placement),
                        ),
                    )
                    .dropdown_menu({
                        let view = view.clone();
                        move |menu, window, cx| {
                            ANCHORS.into_iter().fold(menu, |menu, placement| {
                                menu.item(
                                    PopupMenuItem::new(format!("{placement:?}"))
                                        .checked(cx.theme().notification.placement == placement)
                                        .on_click(window.listener_for(
                                            &view,
                                            move |_, _, _, cx| {
                                                Theme::update(cx, |theme| {
                                                    theme.notification.placement = placement
                                                });
                                            },
                                        )),
                                )
                            })
                        }
                    })
                    .into_any_element(),
                DropdownButton::new("notification-max-items")
                    .button(
                        Button::new(SharedString::from("notification-max-items-trigger"))
                            .label(format!("Max items: {}", cx.theme().notification.max_items)),
                    )
                    .dropdown_menu(move |menu, window, cx| {
                        MAX_ITEMS.into_iter().fold(menu, |menu, max_items| {
                            menu.item(
                                PopupMenuItem::new(format!("{max_items}"))
                                    .checked(cx.theme().notification.max_items == max_items)
                                    .on_click(window.listener_for(&view, move |_, _, _, cx| {
                                        Theme::update(cx, |theme| {
                                            theme.notification.max_items = max_items
                                        });
                                    })),
                            )
                        })
                    })
                    .into_any_element(),
            ]))
            .child(
                section("notification-default", "Default")
                    .description("Show a short message.")
                    .child(
                        Button::new("notification-show")
                            .outline()
                            .label("Show Notification")
                            .on_click(|_, window, cx| {
                                window.push_notification("This is a notification.", cx)
                            }),
                    ),
            )
            .child(
                section("notification-types", "Types")
                    .description("Use semantic treatments for common outcomes.")
                    .child(
                        Button::new("notification-info")
                            .info()
                            .label("Info")
                            .on_click(|_, window, cx| {
                                window.push_notification(
                                    (NotificationType::Info, "Your file was saved."),
                                    cx,
                                )
                            }),
                    )
                    .child(
                        Button::new("notification-success")
                            .success()
                            .label("Success")
                            .on_click(|_, window, cx| {
                                window.push_notification(
                                    (NotificationType::Success, "We have received your payment."),
                                    cx,
                                )
                            }),
                    )
                    .child(
                        Button::new("notification-warning")
                            .warning()
                            .label("Warning")
                            .on_click(|_, window, cx| {
                                window.push_notification(
                                    (
                                        NotificationType::Warning,
                                        "The network is not stable, please check your connection.",
                                    ),
                                    cx,
                                )
                            }),
                    )
                    .child(
                        Button::new("notification-error")
                            .danger()
                            .label("Error")
                            .on_click(|_, window, cx| {
                                window.push_notification(
                                    (
                                        NotificationType::Error,
                                        "Something went wrong. Please try again later.",
                                    ),
                                    cx,
                                )
                            }),
                    ),
            )
            .child(
                section("notification-title", "Title and description")
                    .description("Pair a concise title with supporting detail.")
                    .child(
                        Button::new("notification-typed-info")
                            .info()
                            .label("Info")
                            .on_click(|_, window, cx| {
                                window.push_notification(
                                    Notification::info(
                                        "Your changes have been saved to the cloud \
                                        and will sync across all of your devices.",
                                    )
                                    .title("All changes saved"),
                                    cx,
                                )
                            }),
                    )
                    .child(
                        Button::new("notification-typed-success")
                            .success()
                            .label("Success")
                            .on_click(|_, window, cx| {
                                window.push_notification(
                                    Notification::success(
                                        "Your payment of $99.00 was processed and a \
                                        receipt has been emailed to you.",
                                    )
                                    .title("Payment received"),
                                    cx,
                                )
                            }),
                    )
                    .child(
                        Button::new("notification-typed-warning")
                            .warning()
                            .label("Warning")
                            .on_click(|_, window, cx| {
                                window.push_notification(
                                    Notification::warning(
                                        "Your network connection is unstable. \
                                        Some changes may take longer to save.",
                                    )
                                    .title("Connection unstable"),
                                    cx,
                                )
                            }),
                    )
                    .child(
                        Button::new("notification-typed-error")
                            .danger()
                            .label("Error")
                            .on_click(|_, window, cx| {
                                window.push_notification(
                                    Notification::error(
                                        "We couldn't reach the server. Check your \
                                        internet connection and try again.",
                                    )
                                    .title("Request failed"),
                                    cx,
                                )
                            }),
                    ),
            )
            .child(
                section("notification-unique", "Unique")
                    .description("Replace duplicate notifications by type.")
                    .child(
                        Button::new("notification-unique")
                            .outline()
                            .label("Unique Notification")
                            .on_click(|_, window, cx| {
                                window.push_notification(
                                    Notification::info("This is a unique notification.")
                                        .id::<NotificationSection>(),
                                    cx,
                                )
                            }),
                    ),
            )
            .child(
                section("notification-keyed", "Keyed")
                    .description("Keep separate unique notifications with keys.")
                    .child(
                        h_flex()
                            .gap_3()
                            .child(
                                Button::new("notification-key-a")
                                    .outline()
                                    .label("A Notification")
                                    .on_click(|_, window, cx| {
                                        window.push_notification(
                                            Notification::info("This is A unique notification.")
                                                .id1::<NotificationSection>(1),
                                            cx,
                                        )
                                    }),
                            )
                            .child(
                                Button::new("notification-key-b")
                                    .outline()
                                    .label("B Notification")
                                    .on_click(|_, window, cx| {
                                        window.push_notification(
                                            Notification::info("This is B unique notification.")
                                                .id1::<NotificationSection>(2),
                                            cx,
                                        )
                                    }),
                            ),
                    ),
            )
            .child(
                section("notification-action", "Action")
                    .description("Add an inline action to the notification.")
                    .child(
                        Button::new("notification-with-action")
                            .outline()
                            .label("Notification with Title")
                            .on_click(|_, window, cx| {
                                struct ActionNotification;

                                window.push_notification(
                                    Notification::new()
                                        .id::<ActionNotification>()
                                        .title("Uh oh! Something went wrong.")
                                        .message("There was a problem with your request.")
                                        .action(|_, _, cx| {
                                            Button::new("notification-retry")
                                                .primary()
                                                .label("Retry")
                                                .on_click(cx.listener(|this, _, window, cx| {
                                                    this.dismiss(window, cx);
                                                }))
                                        }),
                                    cx,
                                )
                            }),
                    ),
            )
            .child(
                section("notification-lifecycle", "Lifecycle")
                    .description("Handle body clicks and close events independently.")
                    .child(
                        Button::new("notification-click-close")
                            .outline()
                            .label("Click vs Close")
                            .on_click(|_, window, cx| {
                                struct ClickCloseNotification;

                                window.push_notification(
                                    Notification::info(
                                        "Click the body to fire on_click; click the X to close.",
                                    )
                                    .id::<ClickCloseNotification>()
                                    .title("on_click vs on_close")
                                    .autohide(false)
                                    .on_click(|_, window, cx| {
                                        window.push_notification("on_click fired.", cx);
                                    })
                                    .on_close(|window, cx| {
                                        window.push_notification("on_close fired.", cx);
                                    }),
                                    cx,
                                )
                            }),
                    ),
            )
            .child(
                section("notification-per-placement", "Placement per notification")
                    .description("Override the global placement for a single notification.")
                    .children(ANCHORS.into_iter().map(|placement| {
                        Button::new(SharedString::from(format!("notification-at-{placement:?}")))
                            .outline()
                            .label(format!("{placement:?}"))
                            .on_click(move |_, window, cx| {
                                window.push_notification(
                                    Notification::info(format!(
                                        "This notification is at {placement:?}."
                                    ))
                                    .placement(placement),
                                    cx,
                                )
                            })
                    })),
            )
            .child({
                struct SystemNotificationKind;

                section("notification-system", "System notification")
                    .description(
                        "Deliver to the OS notification center; click the system \
                        notification to refocus the app. macOS shows them only when \
                        running from a bundled .app; Windows requires \
                        cx.set_app_identity().",
                    )
                    .child(
                        Button::new("notification-system")
                            .outline()
                            .label("System only")
                            .on_click(|_, window, cx| {
                                window.push_notification(
                                    Notification::info(
                                        "Delivered straight to the notification center.",
                                    )
                                    .id::<SystemNotificationKind>()
                                    .title("Build finished")
                                    .system(),
                                    cx,
                                )
                            }),
                    )
                    .child(
                        Button::new("notification-in-app-and-system")
                            .outline()
                            .label("In-app and system")
                            .on_click(|_, window, cx| {
                                window.push_notification(
                                    Notification::info(
                                        "Shown as a toast and in the notification center.",
                                    )
                                    .id::<SystemNotificationKind>()
                                    .title("Build finished")
                                    .in_app_and_system()
                                    .autohide(false),
                                    cx,
                                )
                            }),
                    )
            })
            .child(
                section("notification-custom-content", "Custom content")
                    .description("Render application-owned notification content.")
                    .child(
                        Button::new("notification-custom")
                            .outline()
                            .label("Show Custom Notification")
                            .on_click(|_, window, cx| {
                                window.push_notification(
                                    Notification::new().content(|_, _, _| {
                                        markdown(NOTIFICATION_MARKDOWN).into_any_element()
                                    }),
                                    cx,
                                )
                            }),
                    ),
            )
            .child({
                struct ManualCloseNotification;

                section("notification-manual-close", "Manual close")
                    .description("Keep a notification visible until it is dismissed.")
                    .child(
                        Button::new("notification-manual-show")
                            .outline()
                            .label("Show")
                            .on_click(|_, window, cx| {
                                window.push_notification(
                                    Notification::new()
                                        .id::<ManualCloseNotification>()
                                        .message(
                                            "You can close this notification by \
                                            clicking the Close button.",
                                        )
                                        .autohide(false),
                                    cx,
                                );
                            }),
                    )
                    .child(
                        Button::new("notification-manual-dismiss")
                            .outline()
                            .label("Dismiss All")
                            .on_click(|_, window, cx| {
                                window.remove_notification::<ManualCloseNotification>(cx);
                            }),
                    )
            })
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "notification",
        "Notification",
        "Show transient feedback without interrupting the current task.",
        NotificationSection::view(window, cx),
    ));
}
