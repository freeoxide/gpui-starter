//! Kbd section, ported from the upstream `KbdStory`.

use gpui_kit::component::{h_flex, kbd::Kbd, v_flex};
use gpui_kit::{
    App, AppContext, Context, Entity, IntoElement, Keystroke, ParentElement, Render, Styled,
    Window, rems,
};

use crate::gallery::registry::GallerySection;

use super::demo::section;

pub struct KbdSection;

impl KbdSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self)
    }
}

impl Render for KbdSection {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .p_4()
            .child(
                section("kbd-default", "Default")
                    .description("Displays single keys and multi-key shortcuts.")
                    .w(rems(35.))
                    .child(
                        h_flex()
                            .w_full()
                            .justify_center()
                            .gap_2()
                            .flex_wrap()
                            .child(Kbd::new(Keystroke::parse("cmd-shift-p").unwrap()))
                            .child(Kbd::new(Keystroke::parse("cmd-ctrl-t").unwrap()))
                            .child(Kbd::new(Keystroke::parse("cmd--").unwrap()))
                            .child(Kbd::new(Keystroke::parse("cmd-+").unwrap()))
                            .child(Kbd::new(Keystroke::parse("escape").unwrap()))
                            .child(Kbd::new(Keystroke::parse("backspace").unwrap()))
                            .child(Kbd::new(Keystroke::parse("/").unwrap()))
                            .child(Kbd::new(Keystroke::parse("enter").unwrap())),
                    ),
            )
            .child(
                section("kbd-outlined", "Outlined")
                    .description("An outlined treatment adds emphasis on dense surfaces.")
                    .w(rems(35.))
                    .child(
                        h_flex()
                            .w_full()
                            .justify_center()
                            .gap_2()
                            .flex_wrap()
                            .child(Kbd::new(Keystroke::parse("cmd-shift-p").unwrap()).outline())
                            .child(Kbd::new(Keystroke::parse("cmd-ctrl-t").unwrap()).outline())
                            .child(Kbd::new(Keystroke::parse("enter").unwrap()).outline()),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "kbd",
        "Kbd",
        "A tag style to display keyboard shortcuts.",
        KbdSection::view(window, cx),
    ));
}
