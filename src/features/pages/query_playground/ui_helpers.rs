use gpui_kit::component::{ActiveTheme as _, v_flex};
use gpui_kit::{prelude::*, *};

use gpui_query::core::{MutationStatus, QueryStatus};

use crate::accessibility::A11yExt as _;

use super::PlaygroundUser;

/// Section frame. `key` is a stable per-section id (never localized, unique
/// by construction) that derives the heading/description element ids.
pub fn section_card(key: &str, title: &str, description: &str, cx: &App) -> Div {
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
                                    "pg-section-title-{key}"
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
                                    "pg-section-desc-{key}"
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

pub(super) fn fetch_label(loading: bool) -> String {
    if loading {
        crate::i18n::localize("query_playground_fetching")
    } else {
        crate::i18n::localize("query_playground_fetch")
    }
}

pub(super) fn status_label(status: QueryStatus) -> String {
    let key = match status {
        QueryStatus::Idle => "query_playground_status_idle",
        QueryStatus::LoadingEmpty => "query_playground_status_loading_empty",
        QueryStatus::LoadingWithData => "query_playground_status_loading_with_data",
        QueryStatus::Success => "query_playground_status_success",
        QueryStatus::Failure => "query_playground_status_failure",
        QueryStatus::Cancelled => "query_playground_status_cancelled",
    };
    crate::i18n::localize(key)
}

pub(super) fn mutation_status_label(status: MutationStatus) -> String {
    let key = match status {
        MutationStatus::Idle => "query_playground_status_idle",
        MutationStatus::Loading => "query_playground_status_loading",
        MutationStatus::Success => "query_playground_status_success",
        MutationStatus::Failure => "query_playground_status_failure",
    };
    crate::i18n::localize(key)
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
    let status_text = status_label(status);
    let label = format!(
        "{}: {status_text}",
        crate::i18n::localize("query_playground_status")
    );
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
        .child(status_text)
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
    let no_data = crate::i18n::localize("query_playground_no_data");
    let label = match data {
        Some(users) => users
            .iter()
            .map(|u| format!("{} ({}): {}", u.id, u.name, u.email))
            .collect::<Vec<_>>()
            .join("; "),
        None => no_data.clone(),
    };
    let lines = match data {
        Some(users) => users
            .iter()
            .fold(v_flex().gap_0p5(), |el, u| {
                el.child(div().child(format!("{} ({}): {}", u.id, u.name, u.email)))
            })
            .into_any_element(),
        None => v_flex().child(div().child(no_data)).into_any_element(),
    };
    v_flex()
        .id("pg-source-preview")
        .a11y(
            Role::Paragraph,
            format!(
                "{}: {label}",
                crate::i18n::localize("query_playground_source_data")
            ),
        )
        .child(lines)
}

pub fn mapped_preview(data: &Option<Vec<String>>) -> Stateful<Div> {
    let label = match data {
        Some(names) => format!("[{}]", names.join(", ")),
        None => crate::i18n::localize("query_playground_no_data"),
    };
    v_flex()
        .id("pg-mapped-preview")
        .a11y(
            Role::Paragraph,
            format!(
                "{}: {label}",
                crate::i18n::localize("query_playground_mapped_data")
            ),
        )
        .child(div().child(label))
}
