use std::sync::Arc;

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::Input,
    v_flex,
};
use gpui_kit::{prelude::*, *};

use gpui_query::core::{MutationStatus, QueryStatus};

use crate::accessibility::A11yExt as _;

use super::super::ui_helpers::{
    chip, fetch_label, mapped_preview, mutation_status_label, section_card, source_preview,
    status_badge,
};
use super::super::{PlaygroundPage, QueryPlaygroundPage};

impl QueryPlaygroundPage {
    pub(in super::super) fn render_mutation(&mut self, cx: &mut Context<Self>) -> Div {
        let m_status = self
            .mutation_entity
            .as_ref()
            .map_or(MutationStatus::Idle, |(e, _)| {
                e.read_with(cx, |r, _| r.status())
            });
        let m_data = self
            .mutation_entity
            .as_ref()
            .and_then(|(e, _)| e.read_with(cx, |r, _| r.data().cloned()));
        let m_error = self
            .mutation_entity
            .as_ref()
            .and_then(|(e, _)| e.read_with(cx, |r, _| r.error().cloned()));
        let m_vars = self
            .mutation_entity
            .as_ref()
            .and_then(|(e, _)| e.read_with(cx, |r, _| r.variables().cloned()));
        let m_loading = m_status == MutationStatus::Loading;

        let status_color = match m_status {
            MutationStatus::Idle => cx.theme().muted_foreground,
            MutationStatus::Loading => cx.theme().info,
            MutationStatus::Success => cx.theme().success,
            MutationStatus::Failure => cx.theme().danger,
        };

        section_card(
            "mutation",
            &crate::i18n::localize("query_playground_mutation_title"),
            &crate::i18n::localize("query_playground_mutation_description"),
            cx,
        )
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .flex_wrap()
                .px_4()
                .py_2()
                .child(
                    div().flex_1().min_w(px(140.)).child(
                        Input::new(&self.mutation_input_state).aria_label(crate::i18n::localize(
                            "query_playground_mutation_variables",
                        )),
                    ),
                )
                .child(
                    Button::new("pg-mutate")
                        .primary()
                        .label(if m_loading {
                            crate::i18n::localize("query_playground_mutating")
                        } else {
                            crate::i18n::localize("query_playground_mutate")
                        })
                        .disabled(m_loading)
                        .on_click(cx.listener(|this, _, _, cx| this.do_mutate(cx))),
                )
                .child(
                    Button::new("pg-mutate-cb")
                        .outline()
                        .label(crate::i18n::localize(
                            "query_playground_mutate_with_callbacks",
                        ))
                        .disabled(m_loading)
                        .on_click(cx.listener(|this, _, _, cx| this.do_mutate_with_callbacks(cx))),
                )
                .child(
                    Button::new("pg-mutate-reset")
                        .outline()
                        .label(crate::i18n::localize("query_playground_reset"))
                        .on_click(cx.listener(|this, _, _, cx| this.reset_mutation(cx))),
                ),
        )
        .child(
            v_flex().gap_1().px_4().pb_3().child(
                h_flex()
                    .gap_3()
                    .items_center()
                    .child({
                        let status_text = mutation_status_label(m_status);
                        let label = format!(
                            "{}: {status_text}",
                            crate::i18n::localize("query_playground_status")
                        );
                        div()
                            .id("pg-mutation-status")
                            .a11y(Role::Paragraph, label)
                            .px_3()
                            .py_1()
                            .rounded(cx.theme().radius_lg)
                            .border_1()
                            .border_color(status_color)
                            .text_sm()
                            .text_color(status_color)
                            .child(status_text)
                    })
                    .when_some(m_data, |el, d| {
                        el.child(chip(
                            "mutation-data",
                            &format!("{}: {d}", crate::i18n::localize("query_playground_data")),
                            cx.theme().background,
                            cx,
                        ))
                    })
                    .when_some(m_vars, |el, v| {
                        el.child(chip(
                            "mutation-vars",
                            &format!("{}: {v}", crate::i18n::localize("query_playground_vars")),
                            cx.theme().background,
                            cx,
                        ))
                    })
                    .when_some(m_error, |el, e| {
                        el.child(
                            div()
                                .id("pg-mutation-error")
                                .a11y(
                                    Role::Paragraph,
                                    format!(
                                        "{}: {e}",
                                        crate::i18n::localize("query_playground_error")
                                    ),
                                )
                                .a11y_live(accesskit::Live::Polite)
                                .px_3()
                                .py_1()
                                .rounded(cx.theme().radius_lg)
                                .border_1()
                                .border_color(cx.theme().danger)
                                .text_sm()
                                .text_color(cx.theme().danger)
                                .child(format!(
                                    "{}: {e}",
                                    crate::i18n::localize("query_playground_error")
                                )),
                        )
                    }),
            ),
        )
    }

    pub(in super::super) fn render_infinite_query(&mut self, cx: &mut Context<Self>) -> Div {
        let page_count = self
            .infinite_entity
            .as_ref()
            .map_or(0, |(e, _)| e.read_with(cx, |r, _| r.page_count()));
        let inf_status = self
            .infinite_entity
            .as_ref()
            .map_or(QueryStatus::Idle, |(e, _)| {
                e.read_with(cx, |r, _| r.status())
            });
        let loading = inf_status.is_loading();

        // Derive button state from the actual loaded page range: the crate's
        // has_next/has_previous flags only track the last-fetched direction.
        let (first_pg, last_pg) = self
            .infinite_entity
            .as_ref()
            .map_or((None, None), |(e, _)| {
                e.read_with(cx, |r, _| {
                    (
                        r.pages().front().map(|p| p.page_number),
                        r.pages().back().map(|p| p.page_number),
                    )
                })
            });
        let can_next = !loading && last_pg.map_or(true, |n| n < 10);
        let can_prev = !loading && first_pg.map_or(true, |n| n > 0);

        let pages: Vec<(usize, Arc<PlaygroundPage>)> = self
            .infinite_entity
            .as_ref()
            .map(|(e, _)| {
                let mut result = Vec::new();
                e.read_with(cx, |r, _| {
                    // gpui-query stores pages as Arc<T>; cloning yields the shared Arc.
                    for (i, page) in r.pages().iter().enumerate() {
                        result.push((i, page.clone()));
                    }
                });
                result
            })
            .unwrap_or_default();

        let range_label = match (first_pg, last_pg) {
            (Some(f), Some(l)) => format!(
                "{}: {} {f}..{l}",
                crate::i18n::localize("query_playground_range"),
                crate::i18n::localize("query_playground_range_page")
            ),
            _ => format!(
                "{}: ({})",
                crate::i18n::localize("query_playground_range"),
                crate::i18n::localize("query_playground_empty")
            ),
        };

        section_card(
            "infinite-query",
            &crate::i18n::localize("query_playground_infinite_query_title"),
            &crate::i18n::localize("query_playground_infinite_query_description"),
            cx,
        )
        .child(
            h_flex()
                .gap_2()
                .flex_wrap()
                .px_4()
                .py_3()
                .child(
                    Button::new("pg-inf-next")
                        .primary()
                        .label(crate::i18n::localize("query_playground_load_next_page"))
                        .disabled(!can_next)
                        .on_click(cx.listener(|this, _, _, cx| this.load_next_page(cx))),
                )
                .child(
                    Button::new("pg-inf-prev")
                        .outline()
                        .label(crate::i18n::localize("query_playground_load_previous_page"))
                        .disabled(!can_prev)
                        .on_click(cx.listener(|this, _, _, cx| this.load_prev_page(cx))),
                )
                .child(
                    Button::new("pg-inf-reset")
                        .outline()
                        .label(crate::i18n::localize("query_playground_reset"))
                        .on_click(cx.listener(|this, _, _, cx| this.reset_infinite(cx))),
                )
                .child(status_badge("infinite", inf_status, cx))
                .child(chip(
                    "infinite-pages",
                    &format!(
                        "{}: {}/3 ({})",
                        crate::i18n::localize("query_playground_pages"),
                        page_count,
                        crate::i18n::localize("query_playground_max")
                    ),
                    cx.theme().background,
                    cx,
                ))
                .child(chip(
                    "infinite-range",
                    &range_label,
                    cx.theme().background,
                    cx,
                )),
        )
        .when(!pages.is_empty(), |el| {
            el.child(
                v_flex()
                    .gap_2()
                    .px_4()
                    .pb_3()
                    .children(pages.into_iter().map(|(idx, page)| {
                        div()
                            .px_3()
                            .py_2()
                            .rounded(cx.theme().radius)
                            .bg(cx.theme().muted)
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(div().text_xs().font_weight(FontWeight::SEMIBOLD).child(
                                        format!(
                                            "{} {} ({} {})",
                                            crate::i18n::localize("query_playground_page"),
                                            page.page_number,
                                            crate::i18n::localize("query_playground_index"),
                                            idx
                                        ),
                                    ))
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(page.items.join(", ")),
                                    ),
                            )
                    })),
            )
        })
    }

    pub(in super::super) fn render_select_transform(&mut self, cx: &mut Context<Self>) -> Div {
        let source_data = self
            .select_source
            .as_ref()
            .and_then(|e| e.read_with(cx, |r, _| r.data().cloned()));
        let mapped_data = self
            .select_mapped
            .as_ref()
            .and_then(|e| e.read_with(cx, |r, _| r.data()));
        let source_status = self
            .select_source
            .as_ref()
            .map_or(QueryStatus::Idle, |e| e.read_with(cx, |r, _| r.status()));
        let loading = source_status.is_loading();

        section_card(
            "select-transform",
            &crate::i18n::localize("query_playground_select_transform_title"),
            &crate::i18n::localize("query_playground_select_transform_description"),
            cx,
        )
        .child(
            h_flex()
                .gap_2()
                .px_4()
                .py_3()
                .child(
                    Button::new("pg-select-fetch")
                        .primary()
                        .label(if loading {
                            crate::i18n::localize("query_playground_fetching")
                        } else {
                            crate::i18n::localize("query_playground_fetch_source")
                        })
                        .disabled(loading)
                        .on_click(cx.listener(|this, _, _, cx| this.fetch_select(cx))),
                )
                .child(status_badge("select", source_status, cx)),
        )
        .child(
            h_flex()
                .gap_4()
                .px_4()
                .pb_3()
                .child(
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(format!(
                                    "{} (Vec<User>)",
                                    crate::i18n::localize("query_playground_source")
                                )),
                        )
                        .child(
                            div()
                                .p_2()
                                .rounded(cx.theme().radius)
                                .bg(cx.theme().muted)
                                .text_sm()
                                .child(source_preview(&source_data)),
                        ),
                )
                .child(
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(format!(
                                    "{} (Vec<String>)",
                                    crate::i18n::localize("query_playground_mapped")
                                )),
                        )
                        .child(
                            div()
                                .p_2()
                                .rounded(cx.theme().radius)
                                .bg(cx.theme().muted)
                                .text_sm()
                                .child(mapped_preview(&mapped_data)),
                        ),
                ),
        )
    }

    pub(in super::super) fn render_imperative_fetch(&mut self, cx: &mut Context<Self>) -> Div {
        let status = self
            .imperative_query
            .as_ref()
            .map_or(QueryStatus::Idle, |(e, _)| {
                e.read_with(cx, |r, _| r.status())
            });
        let data = self
            .imperative_query
            .as_ref()
            .and_then(|(e, _)| e.read_with(cx, |r, _| r.data().cloned()));
        let signal_cancelled = self.imperative_query.as_ref().map_or(false, |(e, _)| {
            e.read_with(cx, |r, _| {
                r.signal().map(|s| s.is_cancelled()).unwrap_or(false)
            })
        });
        let loading = status.is_loading();

        section_card(
            "imperative-fetch",
            &crate::i18n::localize("query_playground_imperative_fetch_title"),
            &crate::i18n::localize("query_playground_imperative_fetch_description"),
            cx,
        )
        .child(
            h_flex()
                .gap_2()
                .flex_wrap()
                .px_4()
                .py_3()
                .child(
                    Button::new("pg-imp-fetch")
                        .primary()
                        .label(fetch_label(loading))
                        .disabled(loading)
                        .on_click(cx.listener(|this, _, _, cx| this.fetch_imperative(cx))),
                )
                .child(
                    Button::new("pg-imp-cancel")
                        .outline()
                        .label(crate::i18n::localize("query_playground_cancel_mid_flight"))
                        .disabled(!loading)
                        .on_click(cx.listener(|this, _, _, cx| this.cancel_imperative(cx))),
                )
                .child(
                    Button::new("pg-imp-reset")
                        .outline()
                        .label(crate::i18n::localize("query_playground_reset"))
                        .on_click(cx.listener(|this, _, _, cx| this.reset_imperative(cx))),
                ),
        )
        .child(
            h_flex()
                .gap_3()
                .items_center()
                .px_4()
                .pb_3()
                .child(status_badge("imperative", status, cx))
                .when_some(data, |el, d| {
                    el.child(chip("imperative-data", &d, cx.theme().background, cx))
                })
                .child(chip(
                    "imperative-signal",
                    &format!(
                        "{}: {}",
                        crate::i18n::localize("query_playground_signal_cancelled"),
                        if signal_cancelled {
                            crate::i18n::localize("query_playground_yes")
                        } else {
                            crate::i18n::localize("query_playground_no")
                        }
                    ),
                    cx.theme().background,
                    cx,
                )),
        )
    }
}
