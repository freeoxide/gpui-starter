use gpui_kit::prelude::*;
use gpui_kit::{AnyView, TestAppContext, Window, div, px, size};

struct Blank;

impl Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

struct SectionHost(AnyView);

impl Render for SectionHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.0.clone())
    }
}

/// A first frame per entity is the cheapest guard that the family's
/// construction and render stay panic-free.
#[gpui_kit::test]
fn every_section_renders_a_frame(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let window = cx.open_window(size(px(800.), px(600.)), |_, _| Blank);
    let sections = window
        .update(cx, |_, window, cx| {
            let mut sections = vec![];
            super::register(&mut sections, window, cx);
            sections
        })
        .expect("family registers its sections");
    assert!(!sections.is_empty());

    for section in &sections {
        let host = cx.open_window(size(px(900.), px(700.)), |_, _| SectionHost(section.view()));
        host.update(cx, |_, window, cx| window.draw(cx).clear(cx))
            .expect("a section must survive its first frame");
    }
}
