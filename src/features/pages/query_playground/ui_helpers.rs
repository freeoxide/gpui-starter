use gpui::prelude::*;
use gpui::*;

use gpui_component::{ActiveTheme as _, v_flex};

use gpui_query::core::QueryStatus;

use crate::accessibility::A11yExt as _;

use super::PlaygroundUser;

pub fn section_card(title: &str, description: &str, cx: &App) -> Div {
    div()
        .rounded(cx.theme().radius_lg)
        .border_1()
        .border_color(cx.theme().border)
        .overflow_hidden()
        .child(
            div()
                .px_4()
                .py_3()
                .bg(cx.theme().muted)
                .border_b_1()
                .border_color(cx.theme().border)
                .child(
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .id(ElementId::Name(SharedString::from(format!(
                                    "pg-section-title-{title}"
                                ))))
                                .a11y(Role::Heading, title.to_string())
                                .aria_level(2)
                                .text_base()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(title.to_string()),
                        )
                        .child(
                            div()
                                .id(ElementId::Name(SharedString::from(format!(
                                    "pg-section-desc-{title}"
                                ))))
                                .a11y(Role::Paragraph, description.to_string())
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(description.to_string()),
                        ),
                ),
        )
}

/// Demo card. `key` must be unique per call site: the a11y node id derives
/// from it, and label text repeats (duplicate ids panic in debug builds).
pub fn mini_card(key: &str, label: &str, cx: &App) -> Div {
    v_flex()
        .gap_2()
        .p_3()
        .rounded(cx.theme().radius_lg)
        .border_1()
        .border_color(cx.theme().border)
        .bg(cx.theme().background)
        .flex_1()
        .child(
            div()
                .id(ElementId::Name(SharedString::from(format!(
                    "pg-mini-title-{key}"
                ))))
                .a11y(Role::Heading, label.to_string())
                .aria_level(3)
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .child(label.to_string()),
        )
}

/// Status read-out. `key` names the call site, never the status: all
/// sections render Idle at load, and duplicate ids panic in debug builds.
pub fn status_badge(key: &str, status: QueryStatus, cx: &App) -> Stateful<Div> {
    let color = match status {
        QueryStatus::Idle => cx.theme().muted_foreground,
        QueryStatus::LoadingEmpty | QueryStatus::LoadingWithData => cx.theme().info,
        QueryStatus::Success => cx.theme().success,
        QueryStatus::Failure => cx.theme().danger,
        QueryStatus::Cancelled => cx.theme().warning,
    };
    let label = format!("Status: {}", status.label());
    div()
        .id(ElementId::Name(SharedString::from(format!(
            "pg-status-{key}"
        ))))
        .a11y(Role::Paragraph, label)
        .px_3()
        .py_1()
        .rounded(cx.theme().radius_lg)
        .border_1()
        .border_color(color)
        .text_sm()
        .text_color(color)
        .child(status.label().to_string())
}

/// Inline read-only chip. `key` must be unique per call site for the same
/// reason as [`mini_card`]: labels carry runtime data and repeat.
pub fn chip(key: &str, label: &str, background: Hsla, cx: &App) -> Stateful<Div> {
    div()
        .id(ElementId::Name(SharedString::from(format!(
            "pg-chip-{key}"
        ))))
        .a11y(Role::Paragraph, label.to_string())
        .px_3()
        .py_1()
        .rounded(cx.theme().radius_lg)
        .border_1()
        .border_color(cx.theme().border)
        .bg(background)
        .text_sm()
        .child(label.to_string())
}

/// Render each entry on its own line (joined `\n` does not line-break in GPUI).
pub fn source_preview(data: &Option<Vec<PlaygroundUser>>) -> Stateful<Div> {
    let label = match data {
        Some(users) => users
            .iter()
            .map(|u| format!("{} ({}): {}", u.id, u.name, u.email))
            .collect::<Vec<_>>()
            .join("; "),
        None => "No data".to_string(),
    };
    let lines = match data {
        Some(users) => users
            .iter()
            .fold(v_flex().gap_0p5(), |el, u| {
                el.child(div().child(format!("{} ({}): {}", u.id, u.name, u.email)))
            })
            .into_any_element(),
        None => v_flex().child(div().child("No data")).into_any_element(),
    };
    v_flex()
        .id("pg-source-preview")
        .a11y(Role::Paragraph, format!("Source data: {label}"))
        .child(lines)
}

pub fn mapped_preview(data: &Option<Vec<String>>) -> Stateful<Div> {
    let label = match data {
        Some(names) => format!("[{}]", names.join(", ")),
        None => "No data".to_string(),
    };
    v_flex()
        .id("pg-mapped-preview")
        .a11y(Role::Paragraph, format!("Mapped data: {label}"))
        .child(div().child(label))
}
