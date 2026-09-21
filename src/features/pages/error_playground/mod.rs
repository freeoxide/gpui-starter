//! Error boundary playground. Render panics are process-fatal in GPUI, so
//! boundary cards simulate via `TriggerRenderError`; safe cards handle inline.

mod helpers;
mod sections;

use gpui::{prelude::*, *};
use gpui_component::{ActiveTheme as _, v_flex};

use crate::accessibility::A11yExt as _;

pub struct ErrorPlaygroundPage {
    // Inline results for safe tests.
    http_result: Option<String>,
    fs_result: Option<String>,
    async_result: Option<String>,
    background_panic_result: Option<String>,
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
        let intro = "Test different failure modes. Red-bordered cards activate the \
                     error boundary via action dispatch (a real render panic is \
                     process-fatal in GPUI, so the recovery flow is simulated \
                     without crashing). Green-bordered cards handle errors \
                     gracefully inline.";

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
                        v_flex().gap_3().child(
                            div().id("error-playground-title").a11y(Role::Heading, "Error Boundary Playground").aria_level(1)
                                .text_2xl().font_weight(FontWeight::BOLD)
                                .child("Error Boundary Playground"),
                        ).child(
                            div()
                                .id("error-playground-intro")
                                .a11y(Role::Paragraph, intro)
                                .max_w(px(800.))
                                .text_sm()
                                .text_color(muted_foreground)
                                .child(intro),
                        ),
                    ),
            )
            .child(self.render_boundary_trigger(
                "Simulated Render Error",
                "Dispatches TriggerRenderError to activate the error boundary directly. \
                 The fallback page appears with a summary and Reload button.",
                "Trigger Render Error",
                "error playground: simulated render panic",
                cx,
            ))
            .child(self.render_boundary_trigger(
                "Simulated Division by Zero",
                "Activates the error boundary as if a division-by-zero occurred during render.",
                "Trigger Div Zero Error",
                "error playground: simulated division by zero",
                cx,
            ))
            .child(self.render_boundary_trigger(
                "Simulated Index Out of Bounds",
                "Activates the error boundary as if an out-of-bounds access occurred during render.",
                "Trigger OOB Error",
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
