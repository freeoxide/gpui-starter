use gpui::{prelude::*, *};
use gpui_component::{ActiveTheme as _, v_flex};

use crate::accessibility::A11yExt as _;

pub struct AboutPage;

impl AboutPage {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AboutPage {
    fn default() -> Self {
        Self::new()
    }
}

impl Render for AboutPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title = crate::i18n::localize("about_title", None);
        let version = crate::i18n::localize("about_version", None);

        v_flex()
            .min_h_full()
            .items_center()
            .justify_center()
            .gap_4()
            .child(
                div()
                    .id("about-title")
                    .a11y(Role::Heading, title.clone())
                    .aria_level(1)
                    .text_2xl()
                    .child(title),
            )
            .child(
                div()
                    .id("about-version")
                    .a11y(Role::Paragraph, version.clone())
                    .text_color(cx.theme().muted_foreground)
                    .child(version),
            )
    }
}
