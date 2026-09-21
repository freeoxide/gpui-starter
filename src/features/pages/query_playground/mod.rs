//! Query V2 Playground — interactive demo of gpui-query-v2: queries, cache and
//! request policies, retry, mutations, infinite queries, select transforms.

mod queries;
mod render_sections;
mod ui_helpers;

use std::sync::Arc;

use gpui::prelude::*;
use gpui::*;

use gpui_component::{
    ActiveTheme as _, VirtualListScrollHandle,
    input::{InputEvent, InputState},
    v_flex,
};
use serde::{Deserialize, Serialize};

use gpui_query::client::QueryClient;
use gpui_query::core::{
    InfiniteQueryResource, MappedQueryResource, MutationResource, QueryError, QueryResource,
};

use crate::accessibility::A11yExt as _;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlaygroundUser {
    pub id: u32,
    pub name: String,
    pub email: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlaygroundPage {
    pub items: Vec<String>,
    pub page_number: u32,
}

/// A real httpbin response captured by the HTTP Fetching section, used to demo
/// gpui-query managing live network requests (reqwest over the tokio runtime).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HttpFetchResult {
    pub method: String,
    pub label: String,
    pub url: String,
    pub status: u16,
    pub content_type: String,
    pub body: String,
    pub elapsed_ms: u64,
}

/// Which httpbin request the HTTP Fetching section should perform.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum HttpFetchKind {
    GetJson,
    GetXml,
    GetText,
    PostJson,
    GetFail,
}

impl HttpFetchKind {
    pub(super) fn method(self) -> &'static str {
        match self {
            Self::GetJson | Self::GetXml | Self::GetText | Self::GetFail => "GET",
            Self::PostJson => "POST",
        }
    }

    pub(super) fn label(self) -> &'static str {
        match self {
            Self::GetJson => "GET JSON",
            Self::GetXml => "GET XML",
            Self::GetText => "GET text",
            Self::PostJson => "POST JSON",
            Self::GetFail => "GET fail",
        }
    }

    pub(super) fn url(self) -> &'static str {
        match self {
            Self::GetJson => "https://httpbin.org/json",
            Self::GetXml => "https://httpbin.org/xml",
            Self::GetText => "https://httpbin.org/encoding/utf8",
            Self::PostJson => "https://httpbin.org/post",
            Self::GetFail => "https://httpbin.org/status/500",
        }
    }

    pub(super) fn accept_header(self) -> &'static str {
        match self {
            Self::GetJson => "application/json",
            Self::GetXml => "application/xml",
            Self::GetText => "text/plain",
            Self::PostJson => "application/json",
            Self::GetFail => "*/*",
        }
    }
}

// Page state: one lazily created entity + subscription per demo.
pub struct QueryPlaygroundPage {
    pub(super) _subscriptions: Vec<Subscription>,
    // Simple query
    pub(super) simple_query: Option<(
        Entity<QueryResource<PlaygroundUser, QueryError>>,
        Subscription,
    )>,
    // Cache policy demos
    pub(super) nocache_query: Option<(
        Entity<QueryResource<PlaygroundUser, QueryError>>,
        Subscription,
    )>,
    pub(super) ttl_query: Option<(
        Entity<QueryResource<PlaygroundUser, QueryError>>,
        Subscription,
    )>,
    pub(super) swr_query: Option<(
        Entity<QueryResource<PlaygroundUser, QueryError>>,
        Subscription,
    )>,
    // Request policy demos
    pub(super) latest_wins_query: Option<(Entity<QueryResource<String, QueryError>>, Subscription)>,
    pub(super) ignore_query: Option<(Entity<QueryResource<String, QueryError>>, Subscription)>,
    // Retry demo
    pub(super) retry_query: Option<(Entity<QueryResource<String, QueryError>>, Subscription)>,
    // Mutation demo
    pub(super) mutation_entity: Option<(
        Entity<MutationResource<String, String, QueryError>>,
        Subscription,
    )>,
    // Mutation input state
    pub(super) mutation_input_state: Entity<InputState>,
    // Infinite query
    pub(super) infinite_entity: Option<(
        Entity<InfiniteQueryResource<PlaygroundPage, QueryError>>,
        Subscription,
    )>,
    // Select transform
    pub(super) select_source: Option<Entity<QueryResource<Vec<PlaygroundUser>, QueryError>>>,
    pub(super) select_mapped:
        Option<Entity<MappedQueryResource<Vec<PlaygroundUser>, Vec<String>, QueryError>>>,
    pub(super) _select_subs: Option<(Subscription, Subscription)>,
    // Imperative fetch
    pub(super) imperative_query: Option<(Entity<QueryResource<String, QueryError>>, Subscription)>,
    // Real HTTP fetch (reqwest via the tokio runtime)
    pub(super) http_query: Option<(
        Entity<QueryResource<HttpFetchResult, QueryError>>,
        Subscription,
    )>,
    // UI state
    pub(super) activity_log: Vec<String>,
    pub(super) log_scroll_handle: VirtualListScrollHandle,
    // Mutation callbacks write here so their log survives past the click handler.
    pub(super) _callback_log: Arc<std::sync::Mutex<Vec<String>>>,
}

impl QueryPlaygroundPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let mut subs = Vec::new();
        subs.push(cx.observe_global_in::<QueryClient>(window, |_, _, cx| {
            cx.notify();
        }));

        let mutation_input_state =
            cx.new(|cx| InputState::new(window, cx).placeholder("Enter mutation variables..."));

        subs.push(cx.subscribe(
            &mutation_input_state,
            |_this: &mut Self, _state: Entity<InputState>, ev: &InputEvent, cx| {
                if let InputEvent::Change = ev {
                    cx.notify();
                }
            },
        ));

        let callback_log = Arc::new(std::sync::Mutex::new(Vec::<String>::new()));

        Self {
            _subscriptions: subs,
            simple_query: None,
            nocache_query: None,
            ttl_query: None,
            swr_query: None,
            latest_wins_query: None,
            ignore_query: None,
            retry_query: None,
            mutation_entity: None,
            mutation_input_state,
            infinite_entity: None,
            select_source: None,
            select_mapped: None,
            _select_subs: None,
            imperative_query: None,
            http_query: None,
            activity_log: Vec::new(),
            log_scroll_handle: VirtualListScrollHandle::new(),
            _callback_log: callback_log,
        }
    }

    /// Read the current mutation input text from the InputState entity.
    pub(super) fn mutation_input_value(&self, cx: &App) -> String {
        self.mutation_input_state.read(cx).value().to_string()
    }

    pub(super) fn log(&mut self, msg: impl Into<String>) {
        self.activity_log.push(msg.into());
        // Cap at 30 entries to limit DOM overhead.
        if self.activity_log.len() > 30 {
            self.activity_log.drain(0..self.activity_log.len() - 30);
        }
    }
}

impl Render for QueryPlaygroundPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let radius_lg = theme.radius_lg;
        let border = theme.border;
        let muted = theme.muted;
        let muted_foreground = theme.muted_foreground;

        let page = v_flex()
            .id("query-playground-page")
            .min_h_full()
            .p_6()
            .gap_5()
            .overflow_y_scroll()
            .child(
                div()
                    .p_5()
                    .rounded(radius_lg)
                    .border_1()
                    .border_color(border)
                    .bg(muted)
                    .child(
                        v_flex()
                            .gap_3()
                            .child(
                                div()
                                    .id("query-playground-title")
                                    .a11y(Role::Heading, "Query V2 Playground")
                                    .aria_level(1)
                                    .text_2xl()
                                    .font_weight(FontWeight::BOLD)
                                    .child("Query V2 Playground"),
                            )
                            .child(
                                div()
                                    .id("query-playground-intro")
                                    .a11y(
                                        Role::Paragraph,
                                        "Interactive demo of every gpui-query-v2 feature: queries, \
                                         cache policies, request policies, retry, mutations, \
                                         infinite queries, select transforms, and imperative fetch \
                                         with signal cancellation.",
                                    )
                                    .max_w(px(800.))
                                    .text_sm()
                                    .text_color(muted_foreground)
                                    .child(
                                        "Interactive demo of every gpui-query-v2 feature: queries, \
                                         cache policies, request policies, retry, mutations, \
                                         infinite queries, select transforms, and imperative fetch \
                                         with signal cancellation.",
                                    ),
                            ),
                    ),
            )
            .child(self.render_simple_query(cx))
            .child(self.render_cache_policies(cx))
            .child(self.render_request_policies(cx))
            .child(self.render_retry_policy(cx))
            .child(self.render_mutation(cx))
            .child(self.render_infinite_query(cx))
            .child(self.render_select_transform(cx))
            .child(self.render_imperative_fetch(cx))
            .child(self.render_http_fetching(cx))
            .child(self.render_activity_log(cx));

        page
    }
}
