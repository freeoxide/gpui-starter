//! Section renderers for each error playground test card.

use gpui::{prelude::*, *};
use gpui_component::{
    button::{Button, ButtonVariants as _},
    v_flex,
};

use super::super::render_error::TriggerRenderError;
use super::ErrorPlaygroundPage;
use super::helpers::{action_row, result_inline, test_card};

/// Parameter bundle for the unified HTTP/timeout error block renderer.
/// All fields are `Copy`, so one value can be captured by the click closure.
#[derive(Clone, Copy)]
struct ErrorBlockCtx {
    title: &'static str,
    description: &'static str,
    button_key: &'static str,
    button_label: &'static str,
    initial_msg: &'static str,
    url: &'static str,
    timeout: std::time::Duration,
    error_prefix: &'static str,
}

impl ErrorPlaygroundPage {
    pub(super) fn render_boundary_trigger(
        &self,
        title: &str,
        description: &str,
        button_label: &str,
        error_message: &str,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let error_msg = error_message.to_string();
        test_card(title, description, true, cx).child(
            action_row(cx).child(
                Button::new(SharedString::from(format!("ep-{}", title)))
                    .primary()
                    .label(button_label.to_string())
                    .on_click(move |_, _, cx| {
                        // AppRoot listens for this and swaps in RenderErrorPage.
                        cx.dispatch_action(&TriggerRenderError {
                            message: error_msg.clone(),
                        });
                    }),
            ),
        )
    }

    pub(super) fn render_background_panic(&mut self, cx: &mut Context<Self>) -> Stateful<Div> {
        let result_text = self.background_panic_result.clone();
        let card = test_card(
            "Background Task Panic",
            "Spawns an async task on the tokio runtime that panics. Should NOT trigger the \
             error boundary (only render-path panics are caught by the thread-local guard).",
            false,
            cx,
        );

        card.child(
            v_flex().gap_2().px_4().pb_3().child(
                action_row(cx)
                    .child(
                        Button::new("ep-bg-panic")
                            .primary()
                            .label("Spawn Background Panic")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.background_panic_result =
                                    Some("Panic spawned — check logs".to_string());
                                cx.notify();

                                let rt = cx
                                    .global::<crate::services::tokio_runtime::TokioRuntimeGlobal>()
                                    .0
                                    .runtime
                                    .clone();
                                rt.spawn(async move {
                                    panic!("error playground: background panic");
                                });
                            })),
                    )
                    .when_some(result_text, |el, text| el.child(result_inline(&text, cx))),
            ),
        )
    }

    pub(super) fn render_http_error(&mut self, cx: &mut Context<Self>) -> Stateful<Div> {
        let result_text = self.http_result.clone();
        Self::render_error_block(
            ErrorBlockCtx {
                title: "HTTP Error",
                description: "Sends an HTTP request to an invalid URL (127.0.0.1:1). Should NOT \
                    trigger the error boundary — the error is shown inline.",
                button_key: "ep-http-error",
                button_label: "Send HTTP Request",
                initial_msg: "Requesting...",
                url: "http://127.0.0.1:1/fail",
                timeout: std::time::Duration::from_secs(5),
                error_prefix: "HTTP error",
            },
            |this, v| this.http_result = v,
            result_text,
            cx,
        )
    }

    pub(super) fn render_fs_error(&mut self, cx: &mut Context<Self>) -> Stateful<Div> {
        let result_text = self.fs_result.clone();
        let card = test_card(
            "Filesystem Error",
            "Tries to read a file that does not exist. Should NOT trigger the error \
             boundary — the error is shown inline.",
            false,
            cx,
        );

        card.child(
            v_flex().gap_2().px_4().pb_3().child(
                action_row(cx)
                    .child(
                        Button::new("ep-fs-error")
                            .primary()
                            .label("Read Missing File")
                            .on_click(cx.listener(|this, _, _, cx| {
                                let path = "/tmp/gpui-error-playground-nonexistent.txt";
                                match std::fs::read_to_string(path) {
                                    Ok(_contents) => {
                                        this.fs_result =
                                            Some(format!("Unexpected success reading {}", path));
                                    }
                                    Err(e) => {
                                        this.fs_result = Some(format!("FS error: {e}"));
                                    }
                                }
                                cx.notify();
                            })),
                    )
                    .when_some(result_text, |el, text| el.child(result_inline(&text, cx))),
            ),
        )
    }

    pub(super) fn render_async_timeout(&mut self, cx: &mut Context<Self>) -> Stateful<Div> {
        let result_text = self.async_result.clone();
        Self::render_error_block(
            ErrorBlockCtx {
                title: "Async Timeout",
                description: "Sends an HTTP request with a very short timeout (1ms) to a slow \
                    endpoint. Should NOT trigger the error boundary — the timeout error is \
                    shown inline.",
                button_key: "ep-async-timeout",
                button_label: "Send Timeout Request",
                initial_msg: "Requesting (1ms timeout)...",
                url: "http://httpbin.org/delay/5",
                timeout: std::time::Duration::from_millis(1),
                error_prefix: "Timeout error",
            },
            |this, v| this.async_result = v,
            result_text,
            cx,
        )
    }

    /// Unified renderer behind `render_http_error` and `render_async_timeout`;
    /// the two differ only in URL, timeout, copy, and the result field.
    fn render_error_block(
        ctx: ErrorBlockCtx,
        set_result: impl Fn(&mut Self, Option<String>) + Copy + Send + 'static,
        result_text: Option<String>,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let card = test_card(ctx.title, ctx.description, false, cx);

        card.child(
            v_flex().gap_2().px_4().pb_3().child(
                action_row(cx)
                    .child(
                        Button::new(ctx.button_key)
                            .primary()
                            .label(ctx.button_label)
                            .on_click(cx.listener(move |this, _, _, cx| {
                                set_result(this, Some(ctx.initial_msg.to_string()));
                                cx.notify();

                                // One global read covers both the runtime and client handles.
                                let tokio_rt = cx
                                    .global::<crate::services::tokio_runtime::TokioRuntimeGlobal>();
                                // The wasm tokio runtime is an undriven shim; only the client is needed there.
                                #[cfg(not(target_family = "wasm"))]
                                let rt = tokio_rt.0.runtime.clone();
                                let client = tokio_rt.0.http_client.clone();

                                cx.spawn(async move |this, cx| {
                                    // Native: reqwest needs the driven tokio runtime, so spawn
                                    // the request onto it and await the JoinHandle.
                                    #[cfg(not(target_family = "wasm"))]
                                    let msg = {
                                        let result = rt
                                            .spawn(async move {
                                                client
                                                    .get(ctx.url)
                                                    .timeout(ctx.timeout)
                                                    .send()
                                                    .await
                                            })
                                            .await;

                                        match result {
                                            Ok(Ok(resp)) => {
                                                format!(
                                                    "Unexpected success: status {}",
                                                    resp.status()
                                                )
                                            }
                                            Ok(Err(e)) => {
                                                format!("{}: {e}", ctx.error_prefix)
                                            }
                                            Err(e) => format!("Task panicked: {e}"),
                                        }
                                    };

                                    // Wasm: no driven tokio runtime and no `.timeout()` — send from
                                    // this local executor and race the fetch against a GPUI timer.
                                    #[cfg(target_family = "wasm")]
                                    let msg = {
                                        let timeout = ctx.timeout;
                                        let timer = cx.background_executor().timer(timeout);
                                        let fetch = client.get(ctx.url).send();
                                        match futures_util::future::select(
                                            Box::pin(fetch),
                                            Box::pin(timer),
                                        )
                                        .await
                                        {
                                            futures_util::future::Either::Left((Ok(resp), _)) => {
                                                format!(
                                                    "Unexpected success: status {}",
                                                    resp.status()
                                                )
                                            }
                                            futures_util::future::Either::Left((Err(e), _)) => {
                                                format!("{}: {e}", ctx.error_prefix)
                                            }
                                            futures_util::future::Either::Right(_) => format!(
                                                "{}: request timed out after {}ms",
                                                ctx.error_prefix,
                                                timeout.as_millis()
                                            ),
                                        }
                                    };

                                    this.update(cx, |this, cx| {
                                        set_result(this, Some(msg));
                                        cx.notify();
                                    })
                                    .ok();
                                })
                                .detach();
                            })),
                    )
                    .when_some(result_text, |el, text| el.child(result_inline(&text, cx))),
            ),
        )
    }

    pub(super) fn render_clear_results(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let card = test_card(
            "Clear Results",
            "Resets all inline error/success messages.",
            false,
            cx,
        );

        card.child(
            action_row(cx).child(
                Button::new("ep-clear")
                    .outline()
                    .label("Clear All Results")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.http_result = None;
                        this.fs_result = None;
                        this.async_result = None;
                        this.background_panic_result = None;
                        cx.notify();
                    })),
            ),
        )
    }
}
