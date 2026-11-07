use gpui_kit::*;

use gpui_query::core::{CachePolicy, RequestPolicy, RetryPolicy, SelectTransform};
use gpui_query::hook::{
    InfiniteQueryOptions, QueryOptions, use_infinite_query, use_mutation, use_query_manual_opts,
    use_query_select,
};

use super::super::{PlaygroundPage, PlaygroundUser, QueryPlaygroundPage};
use super::actions::{fabricate_page, fetch_select_roster};

/// Shared `ensure_*` body: early-return when populated, else create the entity
/// unfetched (every section fetches on click) and arm its retry policy.
macro_rules! ensure_query {
    (
        $self:ident . $field:ident,
        $cx:expr,
        $options:expr,
        $retry:expr $(,)?
    ) => {{
        if $self.$field.is_some() {
            return;
        }
        let (entity, subscription) = use_query_manual_opts($options, $cx);
        entity.update($cx, |r, _| r.set_retry_policy($retry));
        $self.$field = Some((entity, subscription));
    }};
}

impl QueryPlaygroundPage {
    pub(super) fn ensure_simple_query(&mut self, cx: &mut Context<Self>) {
        ensure_query!(
            self.simple_query,
            cx,
            QueryOptions::new("playground::simple")
                .cache_policy(CachePolicy::NoCache)
                .request_policy(RequestPolicy::LatestWins),
            RetryPolicy::no_retries(),
        );
    }

    pub(super) fn ensure_nocache_query(&mut self, cx: &mut Context<Self>) {
        ensure_query!(
            self.nocache_query,
            cx,
            QueryOptions::new("playground::nocache")
                .cache_policy(CachePolicy::NoCache)
                .request_policy(RequestPolicy::LatestWins),
            RetryPolicy::no_retries(),
        );
    }

    pub(super) fn ensure_ttl_query(&mut self, cx: &mut Context<Self>) {
        ensure_query!(
            self.ttl_query,
            cx,
            QueryOptions::new("playground::ttl")
                .cache_policy(CachePolicy::Ttl { ttl_ms: 5_000 })
                .request_policy(RequestPolicy::LatestWins),
            RetryPolicy::no_retries(),
        );
    }

    pub(super) fn ensure_swr_query(&mut self, cx: &mut Context<Self>) {
        ensure_query!(
            self.swr_query,
            cx,
            QueryOptions::new("playground::swr")
                .cache_policy(CachePolicy::StaleWhileRevalidate {
                    ttl_ms: 3_000,
                    stale_ms: 7_000,
                })
                .request_policy(RequestPolicy::LatestWins),
            RetryPolicy::no_retries(),
        );
    }

    pub(super) fn ensure_latest_wins_query(&mut self, cx: &mut Context<Self>) {
        ensure_query!(
            self.latest_wins_query,
            cx,
            QueryOptions::new("playground::latest_wins")
                .cache_policy(CachePolicy::NoCache)
                .request_policy(RequestPolicy::LatestWins),
            RetryPolicy::no_retries(),
        );
    }

    pub(super) fn ensure_ignore_query(&mut self, cx: &mut Context<Self>) {
        ensure_query!(
            self.ignore_query,
            cx,
            QueryOptions::new("playground::ignore")
                .cache_policy(CachePolicy::NoCache)
                .request_policy(RequestPolicy::IgnoreWhileLoading),
            RetryPolicy::no_retries(),
        );
    }

    pub(super) fn ensure_retry_query(&mut self, cx: &mut Context<Self>) {
        // Retry attempts don't change query status (stays Loading), so the
        // status-deduped observer hides the climbing retry count during the loop.
        ensure_query!(
            self.retry_query,
            cx,
            QueryOptions::new("playground::retry")
                .cache_policy(CachePolicy::NoCache)
                .request_policy(RequestPolicy::LatestWins),
            RetryPolicy::new(3).with_delay(400),
        );
    }

    pub(super) fn ensure_mutation(&mut self, cx: &mut Context<Self>) {
        if self.mutation_entity.is_some() {
            return;
        }
        let (entity, sub) = use_mutation((), cx);
        self.mutation_entity = Some((entity, sub));
    }

    pub(super) fn ensure_infinite(&mut self, cx: &mut Context<Self>) {
        if self.infinite_entity.is_some() {
            return;
        }
        let exec = cx.background_executor().clone();
        let (entity, sub) = use_infinite_query(
            InfiniteQueryOptions::new("playground::infinite")
                .max_pages(3)
                .cache_policy(CachePolicy::NoCache)
                .retry_policy(RetryPolicy::no_retries()),
            move |last_page: Option<&PlaygroundPage>| {
                let exec = exec.clone();
                let page_num = last_page.map(|p| p.page_number + 1).unwrap_or(5);
                async move {
                    let page = fabricate_page(exec, page_num).await;
                    Ok((page, page_num < 10))
                }
            },
            cx,
        );
        self.infinite_entity = Some((entity, sub));
    }

    pub(super) fn ensure_select(&mut self, cx: &mut Context<Self>) {
        if self.select_source.is_some() {
            return;
        }
        let transform = SelectTransform::new(|users: &Vec<PlaygroundUser>| {
            users.iter().map(|u| u.name.clone()).collect()
        });
        let exec = cx.background_executor().clone();
        let (mapped, source, subs) = use_query_select(
            QueryOptions::new("playground::select")
                .cache_policy(CachePolicy::NoCache)
                .retry_policy(RetryPolicy::no_retries()),
            transform,
            move |_signal| {
                let exec = exec.clone();
                async move { fetch_select_roster(exec).await }
            },
            cx,
        );
        self.select_source = Some(source);
        self.select_mapped = Some(mapped);
        self._select_subs = Some(subs);
    }

    pub(super) fn ensure_imperative(&mut self, cx: &mut Context<Self>) {
        ensure_query!(
            self.imperative_query,
            cx,
            QueryOptions::new("playground::imperative")
                .cache_policy(CachePolicy::NoCache)
                .request_policy(RequestPolicy::LatestWins),
            RetryPolicy::no_retries(),
        );
    }

    /// Real HTTP query (reqwest over the tokio runtime), created lazily and
    /// unfetched on the first HTTP click; every click fetches via `fetch_http`.
    pub(super) fn ensure_http_query(&mut self, cx: &mut Context<Self>) {
        ensure_query!(
            self.http_query,
            cx,
            QueryOptions::new("playground::http")
                .cache_policy(CachePolicy::NoCache)
                .request_policy(RequestPolicy::LatestWins),
            RetryPolicy::no_retries(),
        );
    }
}
