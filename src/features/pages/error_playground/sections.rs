use gpui_kit::component::{
    button::{Button, ButtonVariants as _},
    v_flex,
};
use gpui_kit::{prelude::*, *};

use super::super::render_error::TriggerRenderError;
use super::ErrorPlaygroundPage;
use super::helpers::{ResultMsg, action_row, result_inline, test_card};

/// The fs card's read target; guaranteed absent, so the read must fail.
pub(super) const FS_CHECK_PATH: &str = "/tmp/gpui-error-playground-nonexistent.txt";

/// Parameters for `render_error_block`; `*_key` fields are catalog keys. Every
/// field is `Copy`, so one value can be captured by the click closure.
#[derive(Clone, Copy)]
struct ErrorBlockCtx {
    id: &'static str,
    title_key: &'static str,
    description_key: &'static str,
    button_label_key: &'static str,
    initial_msg_key: &'static str,
    url: &'static str,
    timeout: std::time::Duration,
    error_prefix_key: &'static str,
}

impl ErrorPlaygroundPage {
    pub(super) fn render_boundary_trigger(
        &self,
        id: &str,
        title: String,
        description: String,
        button_label: String,
        error_message: &'static str,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let error_msg = error_message.to_string();
        let button_id = SharedString::from(format!("ep-{id}"));
        test_card(id, &title, &description, true, cx).child(
            action_row().child(
                Button::new(button_id)
                    .primary()
                    .label(button_label)
                    .on_click(move |_, window, cx| {
                        // AppRoot swaps in RenderErrorPage. Window-scoped
                        // dispatch: the App-level one fails on Windows.
                        window.dispatch_action(
                            Box::new(TriggerRenderError {
                                message: error_msg.clone(),
                            }),
                            cx,
                        );
                    }),
            ),
        )
    }

    pub(super) fn render_background_panic(&mut self, cx: &mut Context<Self>) -> Stateful<Div> {
        let result_text = self.background_panic_result.clone();
        let card = test_card(
            "bg-panic",
            &crate::i18n::localize("error_playground_bg_panic_title"),
            &crate::i18n::localize("error_playground_bg_panic_description"),
            false,
            cx,
        );

        card.child(
            v_flex().gap_2().px_4().pb_3().child(
                action_row()
                    .child(
                        Button::new("ep-bg-panic")
                            .primary()
                            .label(crate::i18n::localize("error_playground_bg_panic_button"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.background_panic_result = Some(ResultMsg::error(
                                    crate::i18n::localize("error_playground_panic_spawned"),
                                ));
                                cx.notify();

                                let Some(rt) = crate::services::tokio_runtime::handle(cx) else {
                                    this.background_panic_result = Some(ResultMsg::error(
                                        crate::i18n::localize("error_playground_runtime_missing"),
                                    ));
                                    cx.notify();
                                    return;
                                };
                                rt.spawn(async move {
                                    panic!("error playground: background panic");
                                });
                            })),
                    )
                    .when_some(result_text, |el, text| {
                        el.child(result_inline("bg-panic", &text, cx))
                    }),
            ),
        )
    }

    pub(super) fn render_http_error(&mut self, cx: &mut Context<Self>) -> Stateful<Div> {
        let result_text = self.http_result.clone();
        Self::render_error_block(
            ErrorBlockCtx {
                id: "http-error",
                title_key: "error_playground_http_title",
                description_key: "error_playground_http_description",
                button_label_key: "error_playground_http_button",
                initial_msg_key: "error_playground_requesting",
                url: "http://127.0.0.1:1/fail",
                timeout: std::time::Duration::from_secs(5),
                error_prefix_key: "error_playground_http_error_prefix",
            },
            |this, v| this.http_result = v,
            result_text,
            cx,
        )
    }

    pub(super) fn render_fs_error(&mut self, cx: &mut Context<Self>) -> Stateful<Div> {
        let result_text = self.fs_result.clone();
        let card = test_card(
            "fs",
            &crate::i18n::localize("error_playground_fs_title"),
            &crate::i18n::localize("error_playground_fs_description"),
            false,
            cx,
        );

        card.child(
            v_flex().gap_2().px_4().pb_3().child(
                action_row()
                    .child(
                        Button::new("ep-fs-error")
                            .primary()
                            .label(crate::i18n::localize("error_playground_fs_button"))
                            .on_click(cx.listener(|this, _, _, cx| this.run_fs_check(cx))),
                    )
                    .when_some(result_text, |el, text| {
                        el.child(result_inline("fs", &text, cx))
                    }),
            ),
        )
    }

    /// The read target is world-writable /tmp, so a planted large file must
    /// never block paint: the read runs off the UI thread.
    pub(super) fn run_fs_check(&mut self, cx: &mut Context<Self>) {
        cx.spawn(async move |this, cx| {
            let result = cx
                .background_spawn(async move { std::fs::read_to_string(FS_CHECK_PATH) })
                .await;
            this.update(cx, |this, cx| {
                this.fs_result = Some(match result {
                    Ok(_contents) => ResultMsg::info(format!(
                        "{} {FS_CHECK_PATH}",
                        crate::i18n::localize("error_playground_fs_unexpected_success")
                    )),
                    Err(e) => ResultMsg::error(format!(
                        "{} {FS_CHECK_PATH}: {e}",
                        crate::i18n::localize("error_playground_fs_error_prefix")
                    )),
                });
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    pub(super) fn render_async_timeout(&mut self, cx: &mut Context<Self>) -> Stateful<Div> {
        let result_text = self.async_result.clone();
        Self::render_error_block(
            ErrorBlockCtx {
                id: "async-timeout",
                title_key: "error_playground_timeout_title",
                description_key: "error_playground_timeout_description",
                button_label_key: "error_playground_timeout_button",
                initial_msg_key: "error_playground_requesting_timeout",
                url: "http://httpbin.org/delay/5",
                timeout: std::time::Duration::from_millis(1),
                error_prefix_key: "error_playground_timeout_error_prefix",
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
        set_result: impl Fn(&mut Self, Option<ResultMsg>) + Copy + Send + 'static,
        result_text: Option<ResultMsg>,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let card = test_card(
            ctx.id,
            &crate::i18n::localize(ctx.title_key),
            &crate::i18n::localize(ctx.description_key),
            false,
            cx,
        );

        card.child(
            v_flex().gap_2().px_4().pb_3().child(
                action_row()
                    .child(
                        Button::new(SharedString::from(format!("ep-{}", ctx.id)))
                            .primary()
                            .label(crate::i18n::localize(ctx.button_label_key))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                set_result(
                                    this,
                                    Some(ResultMsg::info(crate::i18n::localize(
                                        ctx.initial_msg_key,
                                    ))),
                                );
                                cx.notify();

                                let Some((runtime, client)) =
                                    crate::services::tokio_runtime::runtime_and_client(cx)
                                else {
                                    set_result(
                                        this,
                                        Some(ResultMsg::error(crate::i18n::localize(
                                            "error_playground_runtime_missing",
                                        ))),
                                    );
                                    cx.notify();
                                    return;
                                };
                                // The wasm tokio runtime is an undriven shim; only the client is used there.
                                #[cfg(not(target_family = "wasm"))]
                                let rt = runtime;
                                #[cfg(target_family = "wasm")]
                                let _ = runtime;

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
                                            Ok(Ok(resp)) => ResultMsg::info(format!(
                                                "{} {}",
                                                crate::i18n::localize(
                                                    "error_playground_unexpected_status"
                                                ),
                                                resp.status()
                                            )),
                                            Ok(Err(e)) => ResultMsg::error(format!(
                                                "{}: {e}",
                                                crate::i18n::localize(ctx.error_prefix_key)
                                            )),
                                            Err(e) => ResultMsg::error(format!(
                                                "{}: {e}",
                                                crate::i18n::localize(
                                                    "error_playground_task_panicked"
                                                )
                                            )),
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
                                                ResultMsg::info(format!(
                                                    "{} {}",
                                                    crate::i18n::localize(
                                                        "error_playground_unexpected_status"
                                                    ),
                                                    resp.status()
                                                ))
                                            }
                                            futures_util::future::Either::Left((Err(e), _)) => {
                                                ResultMsg::error(format!(
                                                    "{}: {e}",
                                                    crate::i18n::localize(ctx.error_prefix_key)
                                                ))
                                            }
                                            futures_util::future::Either::Right(_) => {
                                                ResultMsg::error(format!(
                                                    "{}: {} ({}ms)",
                                                    crate::i18n::localize(ctx.error_prefix_key),
                                                    crate::i18n::localize(
                                                        "error_playground_request_timed_out"
                                                    ),
                                                    timeout.as_millis()
                                                ))
                                            }
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
                    .when_some(result_text, |el, text| {
                        el.child(result_inline(ctx.id, &text, cx))
                    }),
            ),
        )
    }

    pub(super) fn render_clear_results(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let card = test_card(
            "clear",
            &crate::i18n::localize("error_playground_clear_title"),
            &crate::i18n::localize("error_playground_clear_description"),
            false,
            cx,
        );

        card.child(
            action_row().child(
                Button::new("ep-clear")
                    .outline()
                    .label(crate::i18n::localize("error_playground_clear_button"))
                    .on_click(cx.listener(|this, _, _, cx| this.clear_results(cx))),
            ),
        )
    }

    pub(super) fn clear_results(&mut self, cx: &mut Context<Self>) {
        self.http_result = None;
        self.fs_result = None;
        self.async_result = None;
        self.background_panic_result = None;
        cx.notify();
    }
}
