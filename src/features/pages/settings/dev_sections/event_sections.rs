use gpui::{prelude::*, *};
use gpui_component::{ActiveTheme as _, button::Button, label::Label, v_flex};

use crate::accessibility::A11yExt as _;

/// Renders the "Event Emitter" card (emit buttons + receiver log).
pub fn render_event_emitter_section(
    event_log: &[String],
    cx: &mut Context<super::super::SettingsPage>,
) -> impl IntoElement {
    let log_total = event_log.len();

    super::settings_card_base(cx)
        .child(
            div()
                .id("settings-events-title")
                .a11y(Role::Heading, "Event Emitter")
                .aria_level(2)
                .child(Label::new("Event Emitter")),
        )
        .child(
            div()
                .id("settings-events-desc")
                .a11y(
                    Role::Paragraph,
                    "Test the event pipeline. Emit events and verify they are received.",
                )
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child("Test the event pipeline. Emit events and verify they are received."),
        )
        // Emit buttons
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(
                    Button::new("emit-test-noop")
                        .outline()
                        .label("Emit Test (No-op)")
                        .on_click(|_, _, cx| {
                            crate::events::emit(
                                crate::events::AppEventKind::Test {
                                    message: "hello from settings".into(),
                                },
                                cx,
                            );
                        }),
                )
                .child(
                    Button::new("emit-navigate-home")
                        .outline()
                        .label("Emit Navigate \u{2192} Home")
                        .on_click(|_, _, cx| {
                            crate::events::emit(
                                crate::events::AppEventKind::Navigate(
                                    crate::routes::AppRoute::page(crate::sidebar::Page::Home),
                                ),
                                cx,
                            );
                        }),
                )
                .child(
                    Button::new("emit-navigate-notifications")
                        .outline()
                        .label("Emit Navigate \u{2192} Notifications")
                        .on_click(|_, _, cx| {
                            crate::events::emit(
                                crate::events::AppEventKind::Navigate(
                                    crate::routes::AppRoute::page(
                                        crate::sidebar::Page::Notifications,
                                    ),
                                ),
                                cx,
                            );
                        }),
                ),
        )
        // Receiver log
        .child(
            div()
                .id("settings-event-receiver-title")
                .a11y(Role::Heading, "Event Receiver")
                .aria_level(3)
                .child(Label::new("Event Receiver")),
        )
        .child(
            v_flex()
                .id("settings-event-log")
                .a11y(Role::List, "Received events")
                .aria_orientation(Orientation::Vertical)
                // AT-SPI derives each item's setsize from the nearest
                // ancestor that declares one; item-level values are ignored.
                .aria_size_of_set(log_total)
                .gap_1()
                .when(event_log.is_empty(), |el| {
                    el.child(
                        div()
                            .id("settings-event-log-empty")
                            .a11y(
                                Role::Paragraph,
                                "No events received yet. Click a button above.",
                            )
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("No events received yet. Click a button above."),
                    )
                })
                .children(event_log.iter().rev().enumerate().map(|(ix, entry)| {
                    div()
                        .id(ElementId::Name(SharedString::from(format!(
                            "settings-event-{ix}"
                        ))))
                        .a11y(Role::ListItem, entry.clone())
                        // accesskit stores position_in_set 0-based; AT
                        // bridges report the stored value +1.
                        .aria_position_in_set(ix)
                        .aria_size_of_set(log_total)
                        .text_xs()
                        .p_1()
                        .rounded(px(4.))
                        .bg(cx.theme().muted)
                        .child(entry.clone())
                })),
        )
}
