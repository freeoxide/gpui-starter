use gpui::{prelude::*, *};
use gpui_component::{
    ActiveTheme as _, Disableable, Icon, IconName, button::Button, h_flex, v_flex,
};

use gpui_query::client::QueryClient;
use gpui_query::core::QueryKeyFilter;

use crate::accessibility::A11yExt as _;

use super::helpers::QuerySort;
use super::mutations::render_mutations_table;
use super::registry::render_query_registry;

pub struct QueryDevToolsV2Page {
    _subscriptions: Vec<Subscription>,
    pub(super) expanded_key: Option<String>,
    pub(super) sort_by: QuerySort,
    /// Status filter: `None` shows all; `Some(String)` must be a valid
    /// `QueryStatus` variant name (e.g. "Idle", "Success").
    pub(super) status_filter: Option<String>,
    /// Scroll handle for the virtualized query registry list.
    pub(super) scroll_handle: gpui_component::VirtualListScrollHandle,
}

impl QueryDevToolsV2Page {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut subscriptions = Vec::new();
        // Refreshes only on QueryClient global mutations; gpui-query writes
        // completions straight to resource entities, so statuses go stale here.
        subscriptions.push(cx.observe_global_in::<QueryClient>(window, |_, _, cx| {
            cx.notify();
        }));
        Self {
            _subscriptions: subscriptions,
            expanded_key: None,
            sort_by: QuerySort::Key,
            status_filter: None,
            scroll_handle: gpui_component::VirtualListScrollHandle::new(),
        }
    }
}

impl Render for QueryDevToolsV2Page {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let diagnostic = cx.try_global::<QueryClient>().map(|c| c.diagnostics(cx));

        let scroll_handle = self.scroll_handle.clone();
        let content = if diagnostic.is_some() {
            render_dashboard(
                &diagnostic,
                &self.expanded_key,
                self.sort_by,
                &self.status_filter,
                &scroll_handle,
                cx,
            )
        } else {
            render_empty_state(cx)
        };

        v_flex().min_h_full().p_6().gap_5().child(content)
    }
}

fn render_empty_state(cx: &mut Context<QueryDevToolsV2Page>) -> Div {
    let theme = cx.theme();
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap_3()
        .py_12()
        .child(
            Icon::new(IconName::Inbox)
                .size_10()
                .text_color(theme.muted_foreground),
        )
        .child(
            div()
                .id("v2-empty-title")
                .a11y(Role::Heading, "No V2 Query Resources")
                .aria_level(1)
                .text_xl()
                .font_weight(FontWeight::BOLD)
                .child("No V2 Query Resources"),
        )
        .child(
            div()
                .id("v2-empty-hint")
                .a11y(
                    Role::Paragraph,
                    "Navigate to the Query Playground page to create queries, then return here.",
                )
                .text_sm()
                .text_color(theme.muted_foreground)
                .child(
                    "Navigate to the Query Playground page to create queries, then return here.",
                ),
        )
}

fn render_dashboard(
    diagnostic: &Option<gpui_query::client::ClientDiagnostic>,
    expanded_key: &Option<String>,
    sort_by: QuerySort,
    status_filter: &Option<String>,
    scroll_handle: &gpui_component::VirtualListScrollHandle,
    cx: &mut Context<QueryDevToolsV2Page>,
) -> Div {
    let theme = cx.theme();
    let radius_lg = theme.radius_lg;
    let border = theme.border;
    let muted = theme.muted;
    let muted_foreground = theme.muted_foreground;

    // Actual collected lengths, not bucket.count() which includes stale weak refs.
    let query_count = diagnostic.as_ref().map(|d| d.queries.len()).unwrap_or(0);
    let mutation_count = diagnostic.as_ref().map(|d| d.mutations.len()).unwrap_or(0);
    let cache_entries = diagnostic
        .as_ref()
        .map(|d| {
            d.queries
                .iter()
                .filter(|q| q.status == gpui_query::core::QueryStatus::Success)
                .count()
        })
        .unwrap_or(0);
    let failed_queries = diagnostic
        .as_ref()
        .map(|d| {
            d.queries
                .iter()
                .filter(|q| q.status == gpui_query::core::QueryStatus::Failure)
                .count()
        })
        .unwrap_or(0);

    let hero = div()
        .rounded(radius_lg)
        .border_1()
        .border_color(border)
        .bg(muted)
        .p_4()
        .child(
            div()
                .id("v2-devtools-title")
                .a11y(Role::Heading, "Query V2 DevTools")
                .aria_level(1)
                .text_xl()
                .font_weight(FontWeight::BOLD)
                .child("Query V2 DevTools"),
        )
        .child(
            div()
                .id("v2-devtools-subtitle")
                .a11y(
                    Role::Paragraph,
                    "Live diagnostics dashboard for gpui-query-v2's QueryClient.",
                )
                .text_sm()
                .text_color(muted_foreground)
                .child("Live diagnostics dashboard for gpui-query-v2's QueryClient."),
        );

    let overview = h_flex().gap_4().children(vec![
        stat_card(
            "Total Queries",
            query_count.to_string(),
            radius_lg,
            border,
            muted,
            muted_foreground,
        ),
        stat_card(
            "Total Mutations",
            mutation_count.to_string(),
            radius_lg,
            border,
            muted,
            muted_foreground,
        ),
        stat_card(
            "Cache Entries",
            cache_entries.to_string(),
            radius_lg,
            border,
            muted,
            muted_foreground,
        ),
        stat_card(
            "Failed Queries",
            failed_queries.to_string(),
            radius_lg,
            border,
            muted,
            muted_foreground,
        ),
    ]);

    let actions = render_action_bar(cx);

    let registry = render_query_registry(
        diagnostic,
        expanded_key,
        sort_by,
        status_filter,
        scroll_handle,
        cx,
    );

    let mutations = render_mutations_table(diagnostic, cx);

    div()
        .gap_5()
        .flex()
        .flex_col()
        .child(hero)
        .child(overview)
        .child(actions)
        .child(registry)
        .child(mutations)
}

fn stat_card(
    label: &str,
    value: String,
    radius_lg: Pixels,
    border: Hsla,
    muted: Hsla,
    muted_foreground: Hsla,
) -> Stateful<Div> {
    div()
        .id(ElementId::Name(SharedString::from(format!(
            "v2-stat-{label}"
        ))))
        .a11y(Role::Paragraph, format!("{label}: {value}"))
        .flex_1()
        .rounded(radius_lg)
        .border_1()
        .border_color(border)
        .bg(muted)
        .p_4()
        .child(div().text_2xl().font_weight(FontWeight::BOLD).child(value))
        .child(
            div()
                .text_sm()
                .text_color(muted_foreground)
                .child(label.to_string()),
        )
}

fn render_action_bar(cx: &mut Context<QueryDevToolsV2Page>) -> Div {
    let theme = cx.theme();
    let radius_lg = theme.radius_lg;
    let border = theme.border;
    let muted = theme.muted;

    let has_client = cx.has_global::<QueryClient>();

    let invalidate = Button::new("v2-devtools-invalidate-all")
        .outline()
        .label("Invalidate All")
        .disabled(!has_client)
        .on_click(cx.listener(|_, _, _, cx| {
            if cx.has_global::<QueryClient>() {
                cx.update_global::<QueryClient, _>(|client, cx| {
                    client.invalidate_queries(&QueryKeyFilter::All, cx);
                });
                cx.notify();
            }
        }));

    let reset = Button::new("v2-devtools-reset-all")
        .outline()
        .label("Reset All")
        .disabled(!has_client)
        .on_click(cx.listener(|_, _, _, cx| {
            if cx.has_global::<QueryClient>() {
                cx.update_global::<QueryClient, _>(|client, cx| {
                    client.reset_queries(&QueryKeyFilter::All, cx);
                });
                cx.notify();
            }
        }));

    let gc = Button::new("v2-devtools-gc")
        .outline()
        .label("GC")
        .disabled(!has_client)
        .on_click(cx.listener(|_, _, _, cx| {
            if cx.has_global::<QueryClient>() {
                cx.update_global::<QueryClient, _>(|client, cx| {
                    client.gc(cx);
                });
                cx.notify();
            }
        }));

    let cancel = Button::new("v2-devtools-cancel-all")
        .outline()
        .label("Cancel All")
        .disabled(!has_client)
        .on_click(cx.listener(|_, _, _, cx| {
            if cx.has_global::<QueryClient>() {
                cx.update_global::<QueryClient, _>(|client, cx| {
                    client.cancel_queries(&QueryKeyFilter::All, cx);
                });
                cx.notify();
            }
        }));

    let remove = Button::new("v2-devtools-remove-all")
        .outline()
        .label("Remove All")
        .disabled(!has_client)
        .on_click(cx.listener(|_, _, _, cx| {
            if cx.has_global::<QueryClient>() {
                cx.update_global::<QueryClient, _>(|client, _cx| {
                    client.remove_queries(&QueryKeyFilter::All);
                });
                cx.notify();
            }
        }));

    div()
        .rounded(radius_lg)
        .border_1()
        .border_color(border)
        .bg(muted)
        .p_4()
        .child(
            div()
                .id("v2-actions-title")
                .a11y(Role::Heading, "Actions")
                .aria_level(2)
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .mb_2()
                .child("Actions"),
        )
        .child(
            h_flex()
                .gap_2()
                .flex_wrap()
                .children(vec![invalidate, reset, gc, cancel, remove]),
        )
}
