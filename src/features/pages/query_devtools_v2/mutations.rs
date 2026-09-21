use gpui::{prelude::*, *};
use gpui_component::{ActiveTheme as _, h_flex, v_flex};

use gpui_query::client::ClientDiagnostic;
use gpui_query::core::MutationStatus;

use crate::accessibility::A11yExt as _;

use super::dashboard::QueryDevToolsV2Page;

pub(super) fn render_mutations_table(
    diagnostic: &Option<ClientDiagnostic>,
    cx: &mut Context<QueryDevToolsV2Page>,
) -> Div {
    let theme = cx.theme();
    let radius_lg = theme.radius_lg;
    let border = theme.border;
    let muted = theme.muted;
    let muted_foreground = theme.muted_foreground;
    let primary = theme.primary;
    let danger = theme.danger;
    let radius = theme.radius;

    let mutations: Vec<_> = diagnostic
        .as_ref()
        .map(|d| d.mutations.clone())
        .unwrap_or_default();

    // Header
    let header_cell = |id: &'static str, label: &'static str| {
        div()
            .id(id)
            .role(Role::ColumnHeader)
            .aria_label(label)
            .text_xs()
            .font_weight(FontWeight::BOLD)
            .flex_1()
            .child(label)
    };
    let header = h_flex()
        .id("v2-mutations-header")
        .role(Role::Row)
        .gap_3()
        .px_3()
        .py_2()
        .children(vec![
            header_cell("v2-mutations-h-key", "Key"),
            header_cell("v2-mutations-h-status", "Status"),
            header_cell("v2-mutations-h-retries", "Retry Count"),
        ]);

    if mutations.is_empty() {
        return div()
            .rounded(radius_lg)
            .border_1()
            .border_color(border)
            .bg(muted)
            .p_4()
            .child(
                div()
                    .id("v2-mutations-title")
                    .a11y(Role::Heading, "Mutations")
                    .aria_level(2)
                    .text_sm()
                    .font_weight(FontWeight::SEMIBOLD)
                    .mb_2()
                    .child("Mutations"),
            )
            .child(
                div().py_4().flex().justify_center().child(
                    div()
                        .id("v2-mutations-empty")
                        .a11y(Role::Paragraph, "No mutations registered.")
                        .text_sm()
                        .text_color(muted_foreground)
                        .child("No mutations registered."),
                ),
            );
    }

    let row_count = mutations.len() + 1;

    let rows: Vec<_> = mutations
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let key_display = m.key.as_deref().unwrap_or("anonymous");

            let status_color = match m.status {
                MutationStatus::Idle => muted_foreground,
                MutationStatus::Loading => primary,
                MutationStatus::Success => primary,
                MutationStatus::Failure => danger,
            };

            let status_label = m.status.label();

            // Stable id from key + index so ids don't shift on removal.
            let row_id = format!(
                "v2-mutation-row-{}-{}",
                m.key.as_deref().unwrap_or("anon"),
                i
            );
            let row_label = format!("{key_display}: {status_label}, {} retries", m.retry_count);

            div()
                .id(ElementId::Name(SharedString::from(row_id)))
                .role(Role::Row)
                .aria_row_index(i + 2)
                .aria_label(row_label)
                .rounded(radius)
                .px_3()
                .py_2()
                .child(h_flex().gap_3().items_center().children(vec![
                        div()
                            .text_sm()
                            .font_family("monospace")
                            .flex_1()
                            .child(key_display.to_string()),
                        div()
                            .text_xs()
                            .font_weight(FontWeight::SEMIBOLD)
                            .px_1()
                            .text_color(status_color)
                            .child(status_label.to_string()),
                        div()
                            .text_xs()
                            .text_color(muted_foreground)
                            .child(format!("{}", m.retry_count)),
                    ]))
                .into_any_element()
        })
        .collect();

    let table = v_flex()
        .id("v2-mutations-table")
        .role(Role::Table)
        .aria_label("Mutations")
        .aria_row_count(row_count)
        .aria_column_count(3)
        .gap_0p5()
        .child(header)
        .children(rows);

    div()
        .rounded(radius_lg)
        .border_1()
        .border_color(border)
        .bg(muted)
        .p_4()
        .child(
            div()
                .id("v2-mutations-title")
                .a11y(Role::Heading, "Mutations")
                .aria_level(2)
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .mb_2()
                .child("Mutations"),
        )
        .child(table)
}
