// No glob `use gpui_kit::*` here: it would import gpui's `test` proc-macro and
// shadow this file's `#[gpui_kit::test]` attribute.
use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::prelude::*;
use gpui_kit::{Bounds, Entity, TestAppContext, Window, div, point, px, size};

use super::{TourController, publish, resolve, target};

fn square(x: f32, y: f32, w: f32, h: f32) -> Bounds<gpui_kit::Pixels> {
    Bounds {
        origin: point(px(x), px(y)),
        size: size(px(w), px(h)),
    }
}

struct ControllerHost {
    tour: Entity<TourController>,
}

impl Render for ControllerHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.tour.clone())
    }
}

#[gpui_kit::test]
fn publish_notifies_the_subscribed_controller_only_on_change(cx: &mut TestAppContext) {
    let window = cx.open_window(size(px(600.), px(400.)), |window, cx| ControllerHost {
        tour: cx.new(|cx| TourController::new(window, cx)),
    });
    let host = window.root(cx).unwrap();
    let tour = cx.read(|cx| host.read(cx).tour.clone());
    cx.run_until_parked();

    let notifies = Rc::new(Cell::new(0usize));
    let counter = notifies.clone();
    cx.update(|cx| {
        cx.observe(&tour, move |_, _| counter.set(counter.get() + 1))
            .detach();
    });
    cx.run_until_parked();
    let before = notifies.get();

    let first = square(0., 0., 10., 10.);
    let moved = square(5., 0., 10., 10.);
    cx.update(|cx| {
        assert!(publish("probe-notify", first, cx));
        assert!(!publish("probe-notify", first, cx));
    });
    cx.run_until_parked();
    assert_eq!(
        notifies.get(),
        before + 1,
        "the first publish notifies once even when immediately repeated"
    );

    cx.update(|cx| assert!(!publish("probe-notify", first, cx)));
    cx.run_until_parked();
    assert_eq!(
        notifies.get(),
        before + 1,
        "steady-state frames must schedule no renders"
    );

    cx.update(|cx| assert!(publish("probe-notify", moved, cx)));
    cx.run_until_parked();
    assert_eq!(
        notifies.get(),
        before + 2,
        "a changed value must notify the controller so the overlay follows"
    );
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
