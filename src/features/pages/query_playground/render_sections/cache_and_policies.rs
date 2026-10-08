use gpui_kit::component::{
    ActiveTheme as _, Disableable as _,
    button::{Button, ButtonVariants as _},
    h_flex,
};
use gpui_kit::{prelude::*, *};

use gpui_query::core::{QueryResource, QueryStatus, RetryPolicy};

use crate::accessibility::A11yExt as _;

use super::super::QueryPlaygroundPage;
use super::super::ui_helpers::{chip, fetch_label, mini_card, section_card, status_badge};

// Readers for the recurring `Option<(Entity<QueryResource<..>>, Subscription)>`
// shape; MutationStatus/infinite/bare-Entity sites don't fit and stay inline.

fn query_status<T: 'static, E: 'static>(
    opt: Option<&(Entity<QueryResource<T, E>>, Subscription)>,
    cx: &App,
) -> QueryStatus {
    opt.map_or(QueryStatus::Idle, |(e, _)| {
        e.read_with(cx, |r, _| r.status())
    })
}

fn query_data<T: Clone + 'static, E: 'static>(
    opt: Option<&(Entity<QueryResource<T, E>>, Subscription)>,
    cx: &App,
) -> Option<T> {
    opt.and_then(|(e, _)| e.read_with(cx, |r, _| r.data().cloned()))
}

fn query_error<T: 'static, E: Clone + 'static>(
    opt: Option<&(Entity<QueryResource<T, E>>, Subscription)>,
    cx: &App,
) -> Option<E> {
    opt.and_then(|(e, _)| e.read_with(cx, |r, _| r.error().cloned()))
}

impl QueryPlaygroundPage {
    pub(in super::super) fn render_cache_policies(&mut self, cx: &mut Context<Self>) -> Div {
        let nocache_status = query_status(self.nocache_query.as_ref(), cx);
        let nocache_loading = nocache_status.is_loading();

        let ttl_status = query_status(self.ttl_query.as_ref(), cx);
        let ttl_loading = ttl_status.is_loading();

        let swr_status = query_status(self.swr_query.as_ref(), cx);
        let swr_loading = swr_status.is_loading();

        section_card(
            "cache-policies",
            &crate::i18n::localize("query_playground_cache_policies_title"),
            &crate::i18n::localize("query_playground_cache_policies_description"),
            cx,
        )
        .child(
            h_flex()
                .gap_4()
                .px_4()
                .py_3()
                .child(
                    mini_card("nocache", "NoCache", cx)
                        .child(status_badge("nocache", nocache_status, cx))
                        .child(
                            Button::new("pg-nocache-fetch")
                                .primary()
                                .label(fetch_label(nocache_loading))
                                .disabled(nocache_loading)
                                .on_click(cx.listener(|this, _, _, cx| this.fetch_nocache(cx))),
                        ),
                )
                .child(
                    mini_card("ttl", "TTL 5s", cx)
                        .child(status_badge("ttl", ttl_status, cx))
                        .child(
                            Button::new("pg-ttl-fetch")
                                .primary()
                                .label(fetch_label(ttl_loading))
                                .disabled(ttl_loading)
                                .on_click(cx.listener(|this, _, _, cx| this.fetch_ttl(cx))),
                        ),
                )
                .child(
                    mini_card("swr", "SWR 3s/7s", cx)
                        .child(status_badge("swr", swr_status, cx))
                        .child(
                            Button::new("pg-swr-fetch")
                                .primary()
                                .label(fetch_label(swr_loading))
                                .disabled(swr_loading)
                                .on_click(cx.listener(|this, _, _, cx| this.fetch_swr(cx))),
                        ),
                ),
        )
    }

    pub(in super::super) fn render_request_policies(&mut self, cx: &mut Context<Self>) -> Div {
        let latest_status = query_status(self.latest_wins_query.as_ref(), cx);
        let latest_data = query_data(self.latest_wins_query.as_ref(), cx);
        let latest_loading = latest_status.is_loading();

        let ignore_status = query_status(self.ignore_query.as_ref(), cx);
        let ignore_data = query_data(self.ignore_query.as_ref(), cx);
        let ignore_loading = ignore_status.is_loading();

        section_card(
            "request-policies",
            &crate::i18n::localize("query_playground_request_policies_title"),
            &crate::i18n::localize("query_playground_request_policies_description"),
            cx,
        )
        .child(
            h_flex()
                .gap_4()
                .px_4()
                .py_3()
                .child(
                    mini_card("latest-wins", "LatestWins", cx)
                        .child(status_badge("latest-wins", latest_status, cx))
                        .when_some(latest_data, |el, d| {
                            el.child(chip("latest-wins-data", &d, cx.theme().background, cx))
                        })
                        .child(
                            Button::new("pg-latest-spam")
                                .primary()
                                .label(crate::i18n::localize("query_playground_spam_fetch"))
                                .disabled(latest_loading)
                                .on_click(cx.listener(|this, _, _, cx| this.spam_latest_wins(cx))),
                        ),
                )
                .child(
                    mini_card("ignore", "IgnoreWhileLoading", cx)
                        .child(status_badge("ignore", ignore_status, cx))
                        .when_some(ignore_data, |el, d| {
                            el.child(chip("ignore-data", &d, cx.theme().background, cx))
                        })
                        .child(
                            Button::new("pg-ignore-spam")
                                .primary()
                                .label(crate::i18n::localize("query_playground_spam_fetch"))
                                .disabled(ignore_loading)
                                .on_click(cx.listener(|this, _, _, cx| this.spam_ignore(cx))),
                        ),
                ),
        )
    }

    pub(in super::super) fn render_retry_policy(&mut self, cx: &mut Context<Self>) -> Div {
        let status = query_status(self.retry_query.as_ref(), cx);
        let error = query_error(self.retry_query.as_ref(), cx);
        let loading = status.is_loading();
        // Read the actual policy off the entity so the chips stay truthful.
        let policy = self
            .retry_query
            .as_ref()
            .map_or(RetryPolicy::new(3).with_delay(400), |(e, _)| {
                e.read_with(cx, |r, _| r.retry_policy().clone())
            });

        section_card(
            "retry-policy",
            &crate::i18n::localize("query_playground_retry_policy_title"),
            &crate::i18n::localize("query_playground_retry_policy_description"),
            cx,
        )
        .child(
            h_flex().gap_2().flex_wrap().px_4().py_3().child(
                Button::new("pg-retry-trigger")
                    .primary()
                    .label(if loading {
                        crate::i18n::localize("query_playground_retrying")
                    } else {
                        crate::i18n::localize("query_playground_trigger_failing_fetch")
                    })
                    .disabled(loading)
                    .on_click(cx.listener(|this, _, _, cx| this.trigger_failing_fetch(cx))),
            ),
        )
        .child(
            h_flex()
                .gap_3()
                .items_center()
                .px_4()
                .pb_3()
                .child(status_badge("retry", status, cx))
                .child(chip(
                    "retry-max",
                    &format!(
                        "{}: {}",
                        crate::i18n::localize("query_playground_max_retries"),
                        policy.max_retries
                    ),
                    cx.theme().background,
                    cx,
                ))
                .child(chip(
                    "retry-backoff",
                    &format!(
                        "{}: {}ms",
                        crate::i18n::localize("query_playground_backoff"),
                        policy.retry_delay_ms
                    ),
                    cx.theme().background,
                    cx,
                ))
                .when_some(error, |el, _| {
                    let gave_up = format!(
                        "{} {} {}",
                        crate::i18n::localize("query_playground_gave_up_after"),
                        policy.max_retries,
                        crate::i18n::localize("query_playground_gave_up_retries")
                    );
                    el.child(
                        div()
                            .id("pg-retry-error")
                            .a11y(Role::Paragraph, gave_up.clone())
                            .a11y_live(accesskit::Live::Polite)
                            .text_xs()
                            .text_color(cx.theme().danger)
                            .child(gave_up),
                    )
                }),
        )
    }
}
