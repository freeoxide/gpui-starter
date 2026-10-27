//! Collapsed pill and expanded widget rows; pure composition, no listeners.

use gpui_kit::component::{ActiveTheme as _, Icon, IconName, h_flex, v_flex};
use gpui_kit::*;

use crate::i18n::localize;

pub(crate) fn collapsed_pill(minute: &str, cx: &App) -> impl IntoElement {
    div()
        .size_full()
        .flex()
        .justify_center()
        .items_center()
        .text_color(cx.theme().foreground)
        .child(
            h_flex()
                .gap_1()
                .child(Icon::new(IconName::ChevronUp).text_color(cx.theme().muted_foreground))
                .child(div().text_xs().child(minute.to_string())),
        )
}

pub(crate) fn clock_block(minute: &str, date: &str, cx: &App) -> impl IntoElement {
    v_flex()
        .gap_1()
        .child(
            div()
                .id("notch-clock")
                .text_xl()
                .aria_label(localize("notch_time_label"))
                .child(minute.to_string()),
        )
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(date.to_string()),
        )
}

pub(crate) fn now_playing(cx: &App) -> impl IntoElement {
    v_flex()
        .gap_1()
        .min_w_0()
        .child(
            h_flex().gap_1().child(Icon::new(IconName::Play)).child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(localize("notch_now_playing")),
            ),
        )
        .child(div().text_sm().child(localize("notch_track_title")))
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(localize("notch_track_artist")),
        )
        .child(
            div()
                .h_1()
                .w_full()
                .rounded_full()
                .bg(cx.theme().muted)
                .overflow_hidden()
                .child(div().h_full().w_2_5().rounded_full().bg(cx.theme().primary)),
        )
}

pub(crate) fn battery_block(cx: &App) -> impl IntoElement {
    v_flex()
        .gap_1()
        .items_end()
        .child(
            h_flex()
                .gap_1()
                .child(Icon::new(IconName::BatteryFull))
                .child(div().text_sm().child("84%")),
        )
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(localize("notch_battery")),
        )
}
