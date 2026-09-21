use gpui::{Context, Div, Role, Stateful, div, prelude::*};
use gpui_component::{ActiveTheme as _, label::Label, v_flex};

use crate::accessibility::A11yExt as _;

use super::SettingsPage;

mod app_sections;
mod event_sections;
mod runtime_sections;
mod telemetry_sections;

pub(crate) use app_sections::{
    render_developer_section, render_shortcuts_section, render_storage_section,
};
pub(crate) use event_sections::render_event_emitter_section;
pub(crate) use runtime_sections::{
    render_desktop_actions_section, render_runtime_boundaries_section,
};
pub(crate) use telemetry_sections::{render_telemetry_runtime_section, render_telemetry_section};

/// Shared base layout for every settings card: a vertical flex with the
/// card chrome (padding, radius, border). Callers append `.child(...)` content.
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
pub(crate) fn section_heading(id: &'static str, title: &'static str) -> Stateful<Div> {
    div()
        .id(id)
        .a11y(Role::Heading, title)
        .aria_level(2)
        .child(Label::new(title))
}
