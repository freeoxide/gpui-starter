use gpui_kit::*;

use gpui_query::core::{CachePolicy, QueryError, RequestPolicy, RetryPolicy, SelectTransform};
use gpui_query::hook::{
    InfiniteQueryOptions, QueryOptions, use_infinite_query, use_mutation, use_query,
    use_query_select,
};

use super::super::{HttpFetchKind, PlaygroundPage, PlaygroundUser, QueryPlaygroundPage};
// Wasm reaches run_http only through `spawn_http_local` (inside actions.rs).
#[cfg(not(target_family = "wasm"))]
use super::actions::run_http;

/// Skeleton for the standard `ensure_*` factories: early-return when populated,
/// then call `use_query`; `$exec` is a cloned background executor.
macro_rules! ensure_query {
    (
        $self:ident . $field:ident,
        $cx:expr,
        $options:expr,
        |$exec:ident| $body:expr $(,)?
    ) => {{
        if $self.$field.is_some() {
            return;
        }
        let exec = $cx.background_executor().clone();
        $self.$field = Some(use_query(
            $options,
            move |_signal| {
                let $exec = exec.clone();
                $body
            },
            $cx,
        ));
    }};
}

impl QueryPlaygroundPage {
    pub(super) fn ensure_simple_query(&mut self, cx: &mut Context<Self>) {
        ensure_query!(
            self.simple_query,
            cx,
            QueryOptions::new("playground::simple")
                .cache_policy(CachePolicy::NoCache)
                .request_policy(RequestPolicy::LatestWins)
                .retry_policy(RetryPolicy::no_retries()),
            |exec| async move {
                exec.timer(std::time::Duration::from_secs(1)).await;
                Ok(PlaygroundUser {
                    id: 1,
                    name: "Alice".into(),
                    email: "alice@test.com".into(),
                })
            }
        );
    }

    pub(super) fn ensure_nocache_query(&mut self, cx: &mut Context<Self>) {
        ensure_query!(
            self.nocache_query,
            cx,
            QueryOptions::new("playground::nocache")
                .cache_policy(CachePolicy::NoCache)
                .request_policy(RequestPolicy::LatestWins)
                .retry_policy(RetryPolicy::no_retries()),
            |exec| async move {
                exec.timer(std::time::Duration::from_millis(500)).await;
                Ok(PlaygroundUser {
                    id: 2,
                    name: "NoCache Bob".into(),
                    email: "bob@test.com".into(),
                })
            }
        );
    }

    pub(super) fn ensure_ttl_query(&mut self, cx: &mut Context<Self>) {
        ensure_query!(
            self.ttl_query,
            cx,
            QueryOptions::new("playground::ttl")
                .cache_policy(CachePolicy::Ttl { ttl_ms: 5_000 })
                .request_policy(RequestPolicy::LatestWins)
                .retry_policy(RetryPolicy::no_retries()),
            |exec| async move {
                exec.timer(std::time::Duration::from_millis(500)).await;
                Ok(PlaygroundUser {
                    id: 3,
                    name: "TTL Carol".into(),
                    email: "carol@test.com".into(),
                })
            }
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
                .request_policy(RequestPolicy::LatestWins)
                .retry_policy(RetryPolicy::no_retries()),
            |exec| async move {
                exec.timer(std::time::Duration::from_millis(500)).await;
                Ok(PlaygroundUser {
                    id: 4,
                    name: "SWR Dave".into(),
                    email: "dave@test.com".into(),
                })
            }
        );
    }

    pub(super) fn ensure_latest_wins_query(&mut self, cx: &mut Context<Self>) {
        ensure_query!(
            self.latest_wins_query,
            cx,
            QueryOptions::new("playground::latest_wins")
                .cache_policy(CachePolicy::NoCache)
                .request_policy(RequestPolicy::LatestWins)
                .retry_policy(RetryPolicy::no_retries()),
            |exec| async move {
                exec.timer(std::time::Duration::from_secs(2)).await;
                Ok("latest-wins result".into())
            }
        );
    }

    pub(super) fn ensure_ignore_query(&mut self, cx: &mut Context<Self>) {
        ensure_query!(
            self.ignore_query,
            cx,
            QueryOptions::new("playground::ignore")
                .cache_policy(CachePolicy::NoCache)
                .request_policy(RequestPolicy::IgnoreWhileLoading)
                .retry_policy(RetryPolicy::no_retries()),
            |exec| async move {
                exec.timer(std::time::Duration::from_secs(2)).await;
                Ok("ignore result".into())
            }
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
                .request_policy(RequestPolicy::LatestWins)
                .retry_policy(RetryPolicy::new(3).with_delay(400)),
            |exec| async move {
                exec.timer(std::time::Duration::from_millis(300)).await;
                Err(QueryError::response("simulated failure"))
            }
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
                    exec.timer(std::time::Duration::from_millis(600)).await;
                    let items: Vec<String> = (0..5)
                        .map(|i| format!("page {} item {}", page_num, i))
                        .collect();
                    Ok((
                        PlaygroundPage {
                            items,
                            page_number: page_num,
                        },
                        page_num < 10,
                    ))
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
                async move {
                    exec.timer(std::time::Duration::from_secs(1)).await;
                    Ok(vec![
                        PlaygroundUser {
                            id: 10,
                            name: "Eve".into(),
                            email: "eve@test.com".into(),
                        },
                        PlaygroundUser {
                            id: 11,
                            name: "Frank".into(),
                            email: "frank@test.com".into(),
                        },
                        PlaygroundUser {
                            id: 12,
                            name: "Grace".into(),
                            email: "grace@test.com".into(),
                        },
                    ])
                }
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
                .request_policy(RequestPolicy::LatestWins)
                .retry_policy(RetryPolicy::no_retries()),
            |exec| async move {
                exec.timer(std::time::Duration::from_secs(2)).await;
                Ok("imperative result".into())
            }
        );
    }

    /// Real HTTP query (reqwest over the tokio runtime), created lazily on the
    /// first HTTP click. Initial fetch is GET JSON; `fetch_http` re-fetches.
    pub(super) fn ensure_http_query(&mut self, cx: &mut Context<Self>) {
        if self.http_query.is_some() {
            return;
        }
        // Clone the shared client + runtime out of the global before entering
        // the fetch closure, releasing the cx borrow.
        let (client, runtime) = match crate::services::tokio_runtime::runtime_and_client(cx) {
            Some((runtime, client)) => (client, runtime),
            None => return, // fetch_http also guards and logs this
        };

        // Wasm: reqwest futures are `!Send` but `use_query` demands `Fn + Send`,
        // so pre-spawn the single initial fetch via `spawn_http_local`.
        #[cfg(target_family = "wasm")]
        let http_task = std::sync::Arc::new(std::sync::Mutex::new(Some(
            super::actions::spawn_http_local(cx, &client, &runtime, HttpFetchKind::GetJson),
        )));

        let (entity, sub) = use_query(
            QueryOptions::new("playground::http")
                .cache_policy(CachePolicy::NoCache)
                .request_policy(RequestPolicy::LatestWins)
                .retry_policy(RetryPolicy::no_retries()),
            move |signal| {
                #[cfg(not(target_family = "wasm"))]
                {
                    let client = client.clone();
                    let runtime = runtime.clone();
                    async move {
                        if signal.is_cancelled() {
                            return Err(QueryError::cancelled("cancelled before send"));
                        }
                        let result = run_http(&client, &runtime, HttpFetchKind::GetJson).await?;
                        Ok(result)
                    }
                }
                #[cfg(target_family = "wasm")]
                {
                    // The fetcher is `Fn` (re-callable), so share the cell via an
                    // Arc clone instead of moving it into the future.
                    let http_task = http_task.clone();
                    async move {
                        if signal.is_cancelled() {
                            return Err(QueryError::cancelled("cancelled before send"));
                        }
                        // Take the handle out before awaiting: MutexGuard is !Send
                        // and must not live across the await point. The only
                        // lock site never panics, so recover the guard on poison.
                        let task = http_task
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner)
                            .take();
                        match task {
                            Some(task) => task.await,
                            None => Err(QueryError::response(
                                "http query refetched; wasm bridge is single-shot",
                            )),
                        }
                    }
                }
            },
            cx,
        );
        self.http_query = Some((entity, sub));
    }
}
