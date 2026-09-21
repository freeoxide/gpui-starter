//! Helper functions for building error playground UI elements.

use gpui::{prelude::*, *};
use gpui_component::{ActiveTheme as _, h_flex, v_flex};

use crate::accessibility::A11yExt as _;

/// Build a test card with a colored left border (red = boundary, green = safe).
pub(crate) fn test_card(title: &str, description: &str, boundary: bool, cx: &App) -> Stateful<Div> {
    let theme = cx.theme();
    let accent = if boundary {
        theme.danger
    } else {
        theme.success
    };
    let card_id: ElementId = ElementId::Name(SharedString::from(format!("ep-card-{title}")));
    let title_id: ElementId = ElementId::Name(SharedString::from(format!("ep-card-title-{title}")));
    let description_id: ElementId =
        ElementId::Name(SharedString::from(format!("ep-card-desc-{title}")));
    let title_text = SharedString::from(title.to_string());
    let description_text = SharedString::from(description.to_string());

    // Colored accent stripe on the left, card body on the right.
    h_flex()
        .id(card_id)
        .a11y(Role::Group, title_text.clone())
        .rounded(theme.radius_lg)
        .overflow_hidden()
        .child(div().h_full().w(px(4.)).bg(accent).flex_shrink_0())
        .child(
            div()
                .flex_1()
                .border_1()
                .border_color(theme.border)
                .overflow_hidden()
                .child(
                    div()
                        .px_4()
                        .py_3()
                        .bg(theme.muted)
                        .border_b_1()
                        .border_color(theme.border)
                        .child(
                            v_flex()
                                .gap_1()
                                .child(
                                    div()
                                        .id(title_id)
                                        .a11y(Role::Heading, title_text)
                                        .aria_level(2)
                                        .text_base()
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .child(title.to_string()),
                                )
                                .child(
                                    div()
                                        .id(description_id)
                                        .a11y(Role::Paragraph, description_text)
                                        .text_xs()
                                        .text_color(theme.muted_foreground)
                                        .child(description.to_string()),
                                ),
                        ),
                ),
        )
}

/// Row that holds the trigger button(s) and any inline result.
pub(crate) fn action_row(_cx: &App) -> Div {
    h_flex().gap_2().flex_wrap().items_center().px_4().py_3()
}

/// Inline result text chip. Announced as a live region so async outcomes are
/// spoken when they land.
pub(crate) fn result_inline(text: &str, cx: &App) -> Stateful<Div> {
    let theme = cx.theme();
    let is_error = text.to_lowercase().contains("error")
        || text.to_lowercase().contains("panic")
        || text.to_lowercase().contains("timeout");

    let color = if is_error {
        theme.danger
    } else {
        theme.muted_foreground
    };

    div()
        .id(ElementId::Name(SharedString::from(format!(
            "ep-result-{text}"
        ))))
        .a11y(Role::Status, text.to_string())
        .a11y_live(accesskit::Live::Polite)
        .text_xs()
        .text_color(color)
        .px_2()
        .py_1()
        .rounded(theme.radius)
        .bg(theme.background)
        .border_1()
        .border_color(color)
        .child(text.to_string())
}
