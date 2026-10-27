use std::cell::RefCell;
use std::rc::Rc;

use gpui_kit::component::h_flex;
use gpui_kit::prelude::*;
use gpui_kit::{
    Bounds, Entity, Subscription, TestAppContext, VisualTestContext, Window, div, point, px, size,
};

use crate::ui::canvas::{CanvasEvent, CanvasState};

use super::InfiniteCanvasPage;
use super::content::{NoteField, build_field};
use super::explore_tool::{PAGE_MAX_ZOOM, PAGE_MIN_ZOOM, page_factor};
use super::perf;
use super::viewport::local_viewport;

struct CanvasHost {
    page: Entity<InfiniteCanvasPage>,
}

impl Render for CanvasHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .size_full()
            .child(div().id("lead").w_16())
            .child(self.page.clone())
    }
}

fn open_page_root(cx: &mut TestAppContext) -> Entity<InfiniteCanvasPage> {
    cx.update(gpui_kit::component::theme::init);
    let handle = cx.open_window(size(px(800.), px(600.)), |window, cx| {
        InfiniteCanvasPage::new(window, cx)
    });
    handle.root(cx).unwrap()
}

fn event_counter(
    cx: &TestAppContext,
    state: &Entity<CanvasState>,
) -> (Rc<RefCell<Vec<CanvasEvent>>>, Subscription) {
    let events = Rc::new(RefCell::new(Vec::new()));
    let sink = events.clone();
    let subscription = cx.update(|cx| {
        cx.subscribe(state, move |_, event: &CanvasEvent, _| {
            sink.borrow_mut().push(event.clone())
        })
    });
    (events, subscription)
}

#[test]
fn local_viewport_strips_the_origin_and_keeps_the_size() {
    let viewport = Bounds {
        origin: point(px(64.), px(128.)),
        size: size(px(800.), px(600.)),
    };
    let local = local_viewport(viewport);
    assert_eq!(local.origin, point(px(0.), px(0.)));
    assert_eq!(local.size, viewport.size);
}

#[gpui_kit::test]
fn pinch_factor_path_clamps_at_the_ceiling_with_a_stable_anchor(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let state = cx.new(|_| CanvasState::new());
    state.update(cx, |state, cx| state.set_zoom(7., cx));
    let anchor = point(px(300.), px(200.));
    let before = cx.read(|cx| state.read(cx).transform().to_doc(anchor));
    let factor = page_factor(cx.read(|cx| state.read(cx).zoom()), 1.0 + 0.5);
    assert!((factor - 8. / 7.).abs() < 1e-6);
    state.update(cx, |state, cx| state.zoom_at(anchor, factor, cx));
    let after_zoom = cx.read(|cx| state.read(cx).zoom());
    assert!((after_zoom - PAGE_MAX_ZOOM).abs() < 0.01);
    let after = cx.read(|cx| state.read(cx).transform().to_doc(anchor));
    assert!((after.x - before.x).as_f32().abs() < 0.01);
    assert!((after.y - before.y).as_f32().abs() < 0.01);
}

#[gpui_kit::test]
fn space_pan_swaps_the_tool_once_per_transition(cx: &mut TestAppContext) {
    let page = open_page_root(cx);
    let state = cx.read(|cx| page.read(cx).state.clone());
    let (events, _subscription) = event_counter(cx, &state);
    page.update(cx, |page, cx| page.set_space_pan(true, cx));
    assert!(cx.read(|cx| state.read(cx).active_tool_id() == "pan"));
    assert_eq!(&*events.borrow(), &[CanvasEvent::ToolChanged]);
    events.borrow_mut().clear();
    page.update(cx, |page, cx| page.set_space_pan(true, cx));
    assert!(events.borrow().is_empty());
    page.update(cx, |page, cx| page.set_space_pan(false, cx));
    assert!(cx.read(|cx| state.read(cx).active_tool_id() == "explore"));
    assert_eq!(&*events.borrow(), &[CanvasEvent::ToolChanged]);
}

#[gpui_kit::test]
fn reconcile_lod_hides_labels_below_the_threshold_without_repeat_mutations(
    cx: &mut TestAppContext,
) {
    let page = open_page_root(cx);
    let state = cx.read(|cx| page.read(cx).state.clone());
    let labels = cx.read(|cx| page.read(cx).layers.1);
    let (events, _subscription) = event_counter(cx, &state);
    page.update(cx, |page, cx| {
        page.state.update(cx, |state, cx| state.set_zoom(0.4, cx))
    });
    assert!(!cx.read(|cx| {
        state
            .read(cx)
            .document()
            .layer(labels)
            .unwrap()
            .is_visible()
    }));
    let mutations: usize = events
        .borrow()
        .iter()
        .filter(|event| **event == CanvasEvent::DocumentChanged)
        .count();
    assert_eq!(mutations, 1);
    page.update(cx, |page, cx| page.reconcile_lod(cx));
    let mutations: usize = events
        .borrow()
        .iter()
        .filter(|event| **event == CanvasEvent::DocumentChanged)
        .count();
    assert_eq!(mutations, 1);
    page.update(cx, |page, cx| {
        page.state.update(cx, |state, cx| state.set_zoom(1.0, cx))
    });
    assert!(cx.read(|cx| {
        state
            .read(cx)
            .document()
            .layer(labels)
            .unwrap()
            .is_visible()
    }));
}

#[gpui_kit::test]
fn store_viewport_with_equal_bounds_leaves_the_field_untouched(cx: &mut TestAppContext) {
    let page = open_page_root(cx);
    let bounds = Bounds {
        origin: point(px(10.), px(20.)),
        size: size(px(100.), px(50.)),
    };
    page.update(cx, |page, _| page.store_viewport(bounds));
    let stored = cx.read(|cx| page.read(cx).viewport);
    assert_eq!(stored, bounds);
    page.update(cx, |page, _| page.store_viewport(bounds));
    assert_eq!(cx.read(|cx| page.read(cx).viewport), bounds);
}

#[gpui_kit::test]
fn viewport_tracks_the_canvas_area_across_resize(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let handle = cx.open_window(size(px(800.), px(600.)), |window, cx| {
        let page = cx.new(|cx| InfiniteCanvasPage::new(window, cx));
        CanvasHost { page }
    });
    let host = handle.root(cx).unwrap();
    let page = cx.read(|cx| host.read(cx).page.clone());
    let mut cx = VisualTestContext::from_window(handle.into(), &cx);
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let viewport = cx.read(|cx| page.read(cx).viewport);
    assert!(viewport.size.width > px(0.));
    assert!(viewport.size.height > px(0.));
    assert!(
        viewport.origin.x > px(0.),
        "the lead strip must offset the area"
    );
    assert!(
        viewport.origin.y > px(0.),
        "the HUD rule must offset the area"
    );
    cx.simulate_window_resize(handle.into(), size(px(900.), px(700.)));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let resized = cx.read(|cx| page.read(cx).viewport);
    assert!(resized.size.width > viewport.size.width);
}

fn viewport_800x600() -> Bounds<gpui_kit::Pixels> {
    Bounds {
        origin: point(px(0.), px(0.)),
        size: size(px(800.), px(600.)),
    }
}

fn drawn_page(cx: &mut TestAppContext) -> (Entity<InfiniteCanvasPage>, VisualTestContext) {
    cx.update(gpui_kit::component::theme::init);
    let handle = cx.open_window(size(px(800.), px(600.)), |window, cx| {
        InfiniteCanvasPage::new(window, cx)
    });
    let page = handle.root(cx).unwrap();
    let mut vcx = VisualTestContext::from_window(handle.into(), cx);
    vcx.update(|window, cx| window.draw(cx).clear(cx));
    (page, vcx)
}

fn assert_centered_on(
    cx: &VisualTestContext,
    page: &Entity<InfiniteCanvasPage>,
    expected: gpui_kit::Point<gpui_kit::Pixels>,
) {
    let (state, viewport) = cx.read(|cx| {
        let page = page.read(cx);
        (page.state.clone(), page.viewport)
    });
    let local_center = point(viewport.size.width * 0.5, viewport.size.height * 0.5);
    let doc = cx.read(|cx| state.read(cx).transform().to_doc(local_center));
    assert!(
        (doc.x - expected.x).as_f32().abs() < 1.,
        "doc.x was {doc:?}, expected ~{expected:?}"
    );
    assert!(
        (doc.y - expected.y).as_f32().abs() < 1.,
        "doc.y was {doc:?}, expected ~{expected:?}"
    );
}

#[gpui_kit::test]
fn zoom_view_at_the_ceiling_is_a_no_op(cx: &mut TestAppContext) {
    let page = open_page_root(cx);
    page.update(cx, |page, cx| {
        page.store_viewport(viewport_800x600());
        page.state
            .update(cx, |state, cx| state.set_zoom(PAGE_MAX_ZOOM, cx));
        page.zoom_view(1.25, cx);
    });
    let zoom = cx.read(|cx| page.read(cx).state.read(cx).zoom());
    assert!((zoom - PAGE_MAX_ZOOM).abs() < 0.01);
}

#[gpui_kit::test]
fn fit_view_frames_a_small_field_unclamped_and_centered(cx: &mut TestAppContext) {
    let (page, mut cx) = drawn_page(cx);
    page.update(&mut cx, |page, cx| {
        let field = NoteField { seed: 1, count: 4 };
        page.layers = page
            .state
            .update(cx, |state, cx| build_field(state, &field, cx));
        page.field = field;
        page.fit_view(cx);
    });
    let (zoom, short_side) = cx.read(|cx| {
        let page = page.read(cx);
        let viewport = page.viewport;
        (
            page.state.read(cx).zoom(),
            viewport
                .size
                .width
                .as_f32()
                .min(viewport.size.height.as_f32()),
        )
    });
    let expected = 0.9 * short_side / 400.;
    assert!(expected > PAGE_MIN_ZOOM && expected < PAGE_MAX_ZOOM);
    assert!(
        (zoom - expected).abs() < 0.01,
        "fit zoom was {zoom}, expected {expected}"
    );
    let span_center = cx.read(|cx| page.read(cx).field.span().center());
    assert_centered_on(&cx, &page, span_center);
}

#[gpui_kit::test]
fn fit_view_clamps_and_recenters_the_default_field(cx: &mut TestAppContext) {
    let (page, mut cx) = drawn_page(cx);
    page.update(&mut cx, |page, cx| page.fit_view(cx));
    let zoom = cx.read(|cx| page.read(cx).state.read(cx).zoom());
    assert!(
        (zoom - PAGE_MIN_ZOOM).abs() < 0.01,
        "clamped zoom was {zoom}"
    );
    let span_center = cx.read(|cx| page.read(cx).field.span().center());
    assert_centered_on(&cx, &page, span_center);
}

#[gpui_kit::test]
fn regenerate_rebuilds_counts_with_a_new_seed_and_records_timing(cx: &mut TestAppContext) {
    let page = open_page_root(cx);
    let before = cx.read(|cx| page.read(cx).field.spec(0).unwrap());
    page.update(cx, |page, cx| page.regenerate(1_000, cx));
    let (bodies, labels, after, preset_ix, gen_us) = cx.read(|cx| {
        let page = page.read(cx);
        let (layers, state) = (page.layers, page.state.clone());
        let doc = state.read(cx).document();
        let (mut body_count, mut label_count) = (0, 0);
        for (layer, _, _) in doc.shapes() {
            if layer == layers.0 {
                body_count += 1;
            } else if layer == layers.1 {
                label_count += 1;
            }
        }
        (
            body_count,
            label_count,
            page.field.spec(0).unwrap(),
            page.preset_ix,
            perf::last_gen_us(),
        )
    });
    assert_eq!(bodies, 1_000 + 6 * (1_000 / 4));
    assert_eq!(labels, 1_000 - 1_000 / 4);
    assert_ne!(
        after.bounds.origin.x.as_f32().to_bits(),
        before.bounds.origin.x.as_f32().to_bits()
    );
    assert_eq!(preset_ix, 0);
    assert!(gen_us > 0);
}
