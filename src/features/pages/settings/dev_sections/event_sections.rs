use gpui_kit::component::{ActiveTheme as _, button::Button, label::Label, v_flex};
use gpui_kit::{prelude::*, *};

use crate::accessibility::A11yExt as _;
use crate::ids::EventId;

pub fn render_event_emitter_section(
    event_log: &[(EventId, String)],
    cx: &mut Context<super::super::SettingsPage>,
) -> impl IntoElement {
    let log_total = event_log.len();
    let emitter_title = crate::i18n::localize("settings_event_emitter");
    let emitter_desc = crate::i18n::localize("settings_event_emitter_desc");
    let receiver_title = crate::i18n::localize("settings_event_receiver");
    let empty_note = crate::i18n::localize("settings_no_events");

    super::settings_card_base(cx)
        .child(
            div()
                .id("settings-events-title")
                .a11y(Role::Heading, emitter_title.clone())
                .aria_level(2)
                .child(Label::new(emitter_title)),
        )
        .child(
            div()
                .id("settings-events-desc")
                .a11y(Role::Paragraph, emitter_desc.clone())
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(emitter_desc),
        )
        .child(
            div()
                .flex()
                .flex_wrap()
                .items_center()
                .gap_2()
                .child(
                    Button::new("emit-test-noop")
                        .outline()
                        .label(crate::i18n::localize("settings_emit_test_noop"))
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
                        .label(crate::i18n::localize("settings_emit_navigate_home"))
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
                        .label(crate::i18n::localize(
                            "settings_emit_navigate_notifications",
                        ))
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
        .child(
            div()
                .id("settings-event-receiver-title")
                .a11y(Role::Heading, receiver_title.clone())
                .aria_level(3)
                .child(Label::new(receiver_title)),
        )
        .child(
            v_flex()
                .id("settings-event-log")
                .a11y(
                    Role::List,
                    crate::i18n::localize("settings_received_events"),
                )
                .aria_orientation(Orientation::Vertical)
                // AT-SPI derives each item's setsize from the nearest
                // ancestor that declares one; item-level values are ignored.
                .aria_size_of_set(log_total)
                .gap_1()
                .when(event_log.is_empty(), |el| {
                    el.child(
                        div()
                            .id("settings-event-log-empty")
                            .a11y(Role::Paragraph, empty_note.clone())
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(empty_note),
                    )
                })
                .children(event_log.iter().rev().enumerate().map(|(ix, (id, entry))| {
                    div()
                        // Keyed by event id: the log drains from the front,
                        // so a position key would move under a fixed row.
                        .id(ElementId::Name(SharedString::from(format!(
                            "settings-event-{id}"
                        ))))
                        .a11y(Role::ListItem, entry.clone())
                        // accesskit stores position_in_set 0-based; AT
                        // bridges report the stored value +1.
                        .aria_position_in_set(ix)
                        .aria_size_of_set(log_total)
                        .text_xs()
                        .p_1()
                        .rounded_sm()
                        .bg(cx.theme().muted)
                        .child(entry.clone())
                })),
        )
}
