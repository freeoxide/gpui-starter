use gpui::{prelude::*, *};
use gpui_component::{ActiveTheme as _, button::Button, v_flex};

use crate::accessibility::A11yExt as _;

/// Clear the error boundary and retry rendering the active page.
#[derive(Action, Clone, PartialEq, Eq, serde::Deserialize)]
#[action(namespace = app, no_json)]
pub struct ReloadCurrentPage;

/// Activate the error boundary with a custom message. Render panics are
/// process-fatal in GPUI, so tests exercise the boundary via this action.
#[derive(Action, Clone, PartialEq, Eq, serde::Deserialize)]
#[action(namespace = app, no_json)]
pub struct TriggerRenderError {
    pub message: String,
}

/// Fallback view shown by the error boundary when a page render panics.
pub struct RenderErrorPage {
    summary: String,
}

impl RenderErrorPage {
    pub fn new(summary: String) -> Self {
        Self { summary }
    }
}

impl Render for RenderErrorPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let summary = self.summary.clone();

        v_flex()
            .min_h_full()
            .items_center()
            .justify_center()
            .gap_4()
            .p_8()
            .child(
                v_flex()
                    .id("render-error-alert")
                    .a11y(Role::Alert, "Render Error")
                    .a11y_live(accesskit::Live::Polite)
                    .items_center()
                    .gap_3()
                    .max_w(px(480.))
                    .child(
                        div()
                            .id("render-error-title")
                            .a11y(Role::Heading, "Render Error")
                            .aria_level(1)
                            .text_xl()
                            .font_weight(FontWeight::BOLD)
                            .text_color(cx.theme().danger)
                            .child("Render Error"),
                    )
                    .child(
                        div()
                            .id("render-error-summary")
                            .a11y(Role::Paragraph, summary.clone())
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(SharedString::from(summary)),
                    )
                    .child(
                        Button::new("reload-current-page")
                            .label("Reload Page")
                            .on_click(|_, _, cx| {
                                cx.dispatch_action(&ReloadCurrentPage);
                            }),
                    ),
            )
    }
}
