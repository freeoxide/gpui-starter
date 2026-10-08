use gpui_kit::component::{ActiveTheme as _, h_flex, v_flex};
use gpui_kit::{prelude::*, *};

use crate::accessibility::A11yExt as _;

/// Inline outcome of a playground action: the displayed text plus whether it
/// should read as an error (the text itself is locale-dependent).
#[derive(Clone, PartialEq)]
pub(crate) struct ResultMsg {
    pub(crate) text: String,
    pub(crate) is_error: bool,
}

impl ResultMsg {
    pub(crate) fn error(text: String) -> Self {
        Self {
            text,
            is_error: true,
        }
    }

    pub(crate) fn info(text: String) -> Self {
        Self {
            text,
            is_error: false,
        }
    }
}

/// `id` keys the card elements; `title`/`description` are localized and would
/// re-key them on every locale switch. Vertical card: action rows flow under
/// the header so text never competes with fixed-width buttons for row width.
pub(crate) fn test_card(
    id: &str,
    title: &str,
    description: &str,
    boundary: bool,
    cx: &App,
) -> Stateful<Div> {
    let theme = cx.theme();
    let accent = if boundary {
        theme.danger
    } else {
        theme.success
    };
    let card_id: ElementId = ElementId::Name(SharedString::from(format!("ep-card-{id}")));
    let title_id: ElementId = ElementId::Name(SharedString::from(format!("ep-card-title-{id}")));
    let description_id: ElementId =
        ElementId::Name(SharedString::from(format!("ep-card-desc-{id}")));
    let title_text = SharedString::from(title.to_string());
    let description_text = SharedString::from(description.to_string());

    v_flex()
        .id(card_id)
        .a11y(Role::Group, title_text.clone())
        .relative()
        .rounded(theme.radius_lg)
        .overflow_hidden()
        .border_1()
        .border_color(theme.border)
        .child(
            // Content masks clip square, so a full-height rail would poke past
            // the rounded corners; insetting by the radius keeps it inside.
            div()
                .absolute()
                .top(theme.radius_lg)
                .bottom(theme.radius_lg)
                .left_0()
                .w_1()
                .bg(accent),
        )
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
        )
}

pub(crate) fn action_row(_cx: &App) -> Div {
    h_flex().gap_2().flex_wrap().items_center().px_4().py_3()
}

/// Inline result text chip, keyed by `id` (the owning card's call-site key:
/// the result text changes as an operation progresses and would re-key the
/// chip mid-flight). Announced as a live region so async outcomes are spoken
/// when they land.
pub(crate) fn result_inline(id: &str, msg: &ResultMsg, cx: &App) -> Stateful<Div> {
    let theme = cx.theme();
    let color = if msg.is_error {
        theme.danger
    } else {
        theme.muted_foreground
    };

    div()
        .id(ElementId::Name(SharedString::from(format!(
            "ep-result-{id}"
        ))))
        .a11y(Role::Status, msg.text.clone())
        .a11y_live(accesskit::Live::Polite)
        .text_xs()
        .text_color(color)
        .px_2()
        .py_1()
        .rounded(theme.radius)
        .bg(theme.background)
        .border_1()
        .border_color(color)
        .child(msg.text.clone())
}
