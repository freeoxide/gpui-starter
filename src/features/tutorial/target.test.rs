// No glob `use gpui_kit::*` here: it would import gpui's `test` proc-macro and
// shadow this file's `#[gpui_kit::test]` attribute.
use gpui_kit::prelude::*;
use gpui_kit::{Bounds, TestAppContext, Window, div, point, px, size};

use super::{publish, resolve, target};

fn square(x: f32, y: f32, w: f32, h: f32) -> Bounds<gpui_kit::Pixels> {
    Bounds {
        origin: point(px(x), px(y)),
        size: size(px(w), px(h)),
    }
}

#[gpui_kit::test]
fn publish_returns_changed_only_for_new_value(cx: &mut TestAppContext) {
    let first = square(0., 0., 10., 10.);
    let moved = square(5., 0., 10., 10.);
    cx.update(|cx| {
        assert!(publish("probe-dedupe", first, cx));
        assert!(!publish("probe-dedupe", first, cx));
        assert!(publish("probe-dedupe", moved, cx));
    });
}

#[gpui_kit::test]
fn resolve_reads_back_published_bounds(cx: &mut TestAppContext) {
    let bounds = square(3., 4., 10., 20.);
    cx.update(|cx| publish("probe-readback", bounds, cx));
    assert_eq!(
        cx.read(|cx| resolve("probe-readback", cx)),
        Some(bounds),
        "resolve must return the last published bounds"
    );
    assert_eq!(
        cx.read(|cx| resolve("never-published", cx)),
        None,
        "an id that never painted must resolve to None"
    );
}

struct ProbeHost;

impl Render for ProbeHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        target("probe", div().id("probe-child").size_full())
    }
}

#[gpui_kit::test]
fn target_publishes_child_bounds_through_a_real_window(cx: &mut TestAppContext) {
    let handle = cx.open_window(size(px(600.), px(400.)), |_, _| ProbeHost);
    cx.update_window(handle.into(), |_, window, cx| {
        window.draw(cx).clear(cx);
    })
    .unwrap();
    assert_eq!(
        cx.read(|cx| resolve("probe", cx)),
        Some(square(0., 0., 600., 400.)),
        "a size_full child of the window root must publish the window bounds"
    );
}
