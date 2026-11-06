use gpui_kit::component::{ActiveTheme as _, label::Label, v_flex};
use gpui_kit::{Context, Div, ElementId, Role, SharedString, Stateful, div, prelude::*};

use crate::accessibility::A11yExt as _;

use super::SettingsPage;

/// Shared base layout for every settings card: a vertical flex with the
/// card chrome (padding, radius, border).
pub(crate) fn settings_card_base(cx: &Context<SettingsPage>) -> Div {
    v_flex()
        .gap_3()
        .p_4()
        .rounded(cx.theme().radius)
        .border_1()
        .border_color(cx.theme().border)
}

/// Card section title: a level-2 heading node, since `Label` text alone
/// produces no accessibility node.
pub(crate) fn section_heading(id: &'static str, title: impl Into<SharedString>) -> Stateful<Div> {
    let title = title.into();
    div()
        .id(id)
        .a11y(Role::Heading, title.clone())
        .aria_level(2)
        .child(Label::new(title))
}

/// Label/value read-out row. `scope` + `id` key the element id; the label is
/// localized and would re-key the row on every locale switch. The combined
/// "label: value" text is the accessible label.
pub(crate) fn status_row(
    scope: &'static str,
    id: &'static str,
    label: impl Into<SharedString>,
    value: impl Into<SharedString>,
) -> impl IntoElement {
    let label = label.into();
    let value = value.into();
    let text = format!("{label}: {value}");
    div()
        .id(ElementId::Name(SharedString::from(format!("{scope}-{id}"))))
        .a11y(Role::Paragraph, text)
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .child(Label::new(label))
        .child(div().text_sm().child(value))
}
