//! Error boundary playground. Render panics are process-fatal in GPUI, so
//! boundary cards simulate via `TriggerRenderError`; safe cards handle inline.

mod helpers;
mod sections;

use gpui_kit::component::{ActiveTheme as _, v_flex};
use gpui_kit::{prelude::*, *};

use crate::accessibility::A11yExt as _;

use helpers::ResultMsg;

pub struct ErrorPlaygroundPage {
    http_result: Option<ResultMsg>,
    fs_result: Option<ResultMsg>,
    async_result: Option<ResultMsg>,
    background_panic_result: Option<ResultMsg>,
}

impl ErrorPlaygroundPage {
    pub fn new() -> Self {
        Self {
            http_result: None,
            fs_result: None,
            async_result: None,
            background_panic_result: None,
        }
    }
}

impl Default for ErrorPlaygroundPage {
    fn default() -> Self {
        Self::new()
    }
}

impl Render for ErrorPlaygroundPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let radius_lg = theme.radius_lg;
        let border = theme.border;
        let muted = theme.muted;
        let muted_foreground = theme.muted_foreground;
        let title = crate::i18n::localize("error_playground_title");
        let intro = crate::i18n::localize("error_playground_intro");

        v_flex()
            .id("error-playground-page")
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
                                    .id("error-playground-title")
                                    .a11y(Role::Heading, title.clone())
                                    .aria_level(1)
                                    .text_2xl()
                                    .font_weight(FontWeight::BOLD)
                                    .child(title),
                            )
                            .child(
                                div()
                                    .id("error-playground-intro")
                                    .a11y(Role::Paragraph, intro.clone())
                                    .max_w(px(800.))
                                    .text_sm()
                                    .text_color(muted_foreground)
                                    .child(intro),
                            ),
                    ),
            )
            .child(self.render_boundary_trigger(
                "render-error",
                crate::i18n::localize("error_playground_render_error_title"),
                crate::i18n::localize("error_playground_render_error_description"),
                crate::i18n::localize("error_playground_render_error_button"),
                "error playground: simulated render panic",
                cx,
            ))
            .child(self.render_boundary_trigger(
                "div-zero",
                crate::i18n::localize("error_playground_div_zero_title"),
                crate::i18n::localize("error_playground_div_zero_description"),
                crate::i18n::localize("error_playground_div_zero_button"),
                "error playground: simulated division by zero",
                cx,
            ))
            .child(self.render_boundary_trigger(
                "oob",
                crate::i18n::localize("error_playground_oob_title"),
                crate::i18n::localize("error_playground_oob_description"),
                crate::i18n::localize("error_playground_oob_button"),
                "error playground: simulated index out of bounds",
                cx,
            ))
            .child(self.render_background_panic(cx))
            .child(self.render_http_error(cx))
            .child(self.render_fs_error(cx))
            .child(self.render_async_timeout(cx))
            .child(self.render_clear_results(cx))
    }
}

#[cfg(test)]
#[path = "../error_playground.test.rs"]
mod error_playground_test;
