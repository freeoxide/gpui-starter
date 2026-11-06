use std::sync::Arc;

use gpui_kit::{App, Global};

/// Dedicated tokio runtime for I/O-bound work: GPUI's executor is not a tokio
/// runtime, so tokio-dependent code must run via `runtime.spawn(...)` here.
pub struct TokioRuntime {
    pub runtime: Arc<tokio::runtime::Runtime>,
    pub http_client: reqwest::Client,
}

#[cfg(not(target_family = "wasm"))]
impl TokioRuntime {
    pub fn new() -> Self {
        let runtime = match tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("gpui-io")
            .build()
        {
            Ok(runtime) => Arc::new(runtime),
            Err(err) => {
                tracing::warn!(
                    target: "gpui_starter::tokio_runtime",
                    error = %err,
                    "multi-thread runtime unavailable; falling back to a current-thread runtime"
                );
                // An undriven current-thread runtime would strand every
                // spawned task, so park one named thread on block_on.
                let fallback = Arc::new(
                    tokio::runtime::Builder::new_current_thread()
                        .enable_all()
                        .build()
                        .expect("failed to create tokio runtime"),
                );
                let driver = fallback.clone();
                std::thread::Builder::new()
                    .name("gpui-io".into())
                    .spawn(move || driver.block_on(std::future::pending::<()>()))
                    .expect("failed to spawn tokio driver thread");
                fallback
            }
        };
        tracing::info!(target: "gpui_starter::tokio_runtime", "tokio runtime created");

        // Build the reqwest client inside the runtime context so the connector
        // can resolve DNS via tokio.
        let _guard = runtime.enter();
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .cookie_store(true)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("failed to build reqwest client");

        Self {
            runtime,
            http_client,
        }
    }
}

/// Wasm shim: an undriven current-thread runtime (no `enable_all()` — its clock
/// setup panics on wasm) plus a plain reqwest client. Use GPUI's executor.
#[cfg(target_family = "wasm")]
impl TokioRuntime {
    pub fn new() -> Self {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("failed to create tokio runtime shim");
        tracing::info!(
            target: "gpui_starter::tokio_runtime",
            "tokio current-thread runtime shim created (wasm; not driven)"
        );

        let http_client = reqwest::Client::builder()
            .build()
            .expect("failed to build reqwest client");

        Self {
            runtime: Arc::new(runtime),
            http_client,
        }
    }
}

impl Default for TokioRuntime {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TokioRuntimeGlobal(pub TokioRuntime);

impl Global for TokioRuntimeGlobal {}

/// Callers should degrade gracefully when this returns `None`.
pub fn handle(cx: &App) -> Option<Arc<tokio::runtime::Runtime>> {
    cx.try_global::<TokioRuntimeGlobal>()
        .map(|g| g.0.runtime.clone())
}

/// Callers should degrade gracefully when this returns `None`.
pub fn runtime_and_client(cx: &App) -> Option<(Arc<tokio::runtime::Runtime>, reqwest::Client)> {
    cx.try_global::<TokioRuntimeGlobal>()
        .map(|g| (g.0.runtime.clone(), g.0.http_client.clone()))
}
