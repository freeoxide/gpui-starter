use gpui_kit::prelude::*;
use gpui_kit::{
    AnyView, AnyWindowHandle, App, Bounds, Entity, TestAppContext, Window, WindowBounds,
    WindowOptions, div, point, px, size,
};

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

// Sections read kit globals and per-window Root plugin state, so every
// window opens through gpui_kit::open_window, never the raw context one.
fn open_sized<V: Render>(
    cx: &mut TestAppContext,
    width: f32,
    height: f32,
    build: impl FnOnce(&mut Window, &mut App) -> Entity<V>,
) -> AnyWindowHandle {
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: size(px(width), px(height)),
        })),
        ..WindowOptions::default()
    };
    let (handle, _) = cx
        .update(|cx| gpui_kit::open_window(options, cx, build))
        .expect("test window must open");
    handle
}

/// A first frame per entity is the cheapest guard that the family's
/// construction and render stay panic-free.
#[gpui_kit::test]
fn every_section_renders_a_frame(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let window = open_sized(cx, 800., 600., |_, cx| cx.new(|_| Blank));
    let sections = window
        .update(cx, |_, window, cx| {
            let mut sections = vec![];
            super::register(&mut sections, window, cx);
            sections
        })
        .expect("family registers its sections");
    assert!(!sections.is_empty());

    for section in &sections {
        // Draw through AnyWindowHandle: a typed handle holds a lease on the
        // root entity, and window.draw leases it again while rendering.
        let host = open_sized(cx, 900., 700., |_, cx| {
            cx.new(|_| SectionHost(section.view()))
        });
        host.update(cx, |_, window, cx| window.draw(cx).clear(cx))
            .expect("a section must survive its first frame");
    }
}
