// No `use super::*`: gpui's `test` proc-macro glob would make bare `#[test]`
// recurse infinitely. Import only what we need.
use std::cell::RefCell;
use std::rc::Rc;

use gpui_kit::{
    AppContext, Bounds, Entity, Pixels, Point, Size, Subscription, TestAppContext, point, px, size,
};

use crate::ui::canvas::{
    CanvasEvent, CanvasShape, CanvasState, CanvasTransform, LayerId, MAX_ZOOM, MIN_ZOOM, ShapeId,
    ShapeStyle,
};

fn rect_shape(bounds: Bounds<Pixels>) -> CanvasShape {
    CanvasShape::rect(bounds, ShapeStyle::default())
}

fn assert_px_eq(actual: Pixels, expected: Pixels) {
    assert!(
        (actual.as_f32() - expected.as_f32()).abs() < 0.01,
        "expected ~{expected:?}, got {actual:?}"
    );
}

fn collect_events(
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

fn seed_two_overlapping_rects(
    cx: &mut TestAppContext,
    state: &Entity<CanvasState>,
) -> (ShapeId, ShapeId, LayerId, LayerId) {
    state.update(cx, |s, cx| {
        let bottom = s.add_layer(cx);
        let top = s.add_layer(cx);
        let bounds = Bounds {
            origin: point(px(0.), px(0.)),
            size: size(px(100.), px(100.)),
        };
        let bottom_shape = s.add_shape(bottom, rect_shape(bounds), cx).unwrap();
        let top_shape = s.add_shape(top, rect_shape(bounds), cx).unwrap();
        (bottom_shape, top_shape, bottom, top)
    })
}

#[gpui_kit::test]
fn edit_document_bumps_revision_once_and_emits_one_event(cx: &mut TestAppContext) {
    let state = cx.new(|_| CanvasState::new());
    let (events, _subscription) = collect_events(cx, &state);
    state.update(cx, |s, cx| {
        s.edit_document(cx, |doc| doc.add_layer());
    });
    assert_eq!(cx.read(|cx| state.read(cx).document_revision()), 1);
    assert_eq!(&*events.borrow(), &[CanvasEvent::DocumentChanged]);
}

#[gpui_kit::test]
fn add_shape_emits_exactly_one_document_changed(cx: &mut TestAppContext) {
    let state = cx.new(|_| CanvasState::new());
    let layer = state.update(cx, |s, cx| s.add_layer(cx));
    let (events, _subscription) = collect_events(cx, &state);
    state.update(cx, |s, cx| {
        s.add_shape(layer, rect_shape(Bounds::default()), cx);
    });
    assert_eq!(&*events.borrow(), &[CanvasEvent::DocumentChanged]);
}

#[gpui_kit::test]
fn pan_by_emits_view_transformed_and_moves_the_pan(cx: &mut TestAppContext) {
    let state = cx.new(|_| CanvasState::new());
    let (events, _subscription) = collect_events(cx, &state);
    state.update(cx, |s, cx| s.pan_by(point(px(20.), px(10.)), cx));
    assert_px_eq(cx.read(|cx| state.read(cx).pan().x), px(20.));
    assert_eq!(&*events.borrow(), &[CanvasEvent::ViewTransformed]);
}

#[gpui_kit::test]
fn zoom_at_keeps_the_anchor_fixed(cx: &mut TestAppContext) {
    let state = cx.new(|_| CanvasState::new());
    state.update(cx, |s, cx| s.pan_by(point(px(30.), px(-12.)), cx));
    let anchor = point(px(100.), px(60.));
    state.update(cx, |s, cx| s.zoom_at(anchor, 1.7, cx));
    let roundtrip = cx.read(|cx| {
        let transform = state.read(cx).transform();
        transform.to_screen(transform.to_doc(anchor))
    });
    assert_px_eq(roundtrip.x, anchor.x);
    assert_px_eq(roundtrip.y, anchor.y);
}

#[gpui_kit::test]
fn zoom_at_clamps_at_both_ends(cx: &mut TestAppContext) {
    let state = cx.new(|_| CanvasState::new());
    state.update(cx, |s, cx| s.set_zoom(MAX_ZOOM, cx));
    state.update(cx, |s, cx| s.zoom_at(Point::default(), 4., cx));
    assert_eq!(cx.read(|cx| state.read(cx).zoom()), MAX_ZOOM);
    state.update(cx, |s, cx| s.set_zoom(MIN_ZOOM, cx));
    state.update(cx, |s, cx| s.zoom_at(Point::default(), 0.25, cx));
    assert_eq!(cx.read(|cx| state.read(cx).zoom()), MIN_ZOOM);
}

#[gpui_kit::test]
fn set_zoom_keeps_the_pan(cx: &mut TestAppContext) {
    let state = cx.new(|_| CanvasState::new());
    state.update(cx, |s, cx| s.pan_by(point(px(7.), px(9.)), cx));
    state.update(cx, |s, cx| s.set_zoom(2.5, cx));
    cx.read(|cx| {
        let state = state.read(cx);
        assert_eq!(state.zoom(), 2.5);
        assert_px_eq(state.pan().y, px(9.));
    });
}

#[gpui_kit::test]
fn zoom_to_fit_matches_the_formula_and_centers_the_doc(cx: &mut TestAppContext) {
    let state = cx.new(|_| CanvasState::new());
    let doc_bounds = Bounds {
        origin: point(px(100.), px(100.)),
        size: size(px(200.), px(100.)),
    };
    let viewport = size(px(1000.), px(500.));
    state.update(cx, |s, cx| s.zoom_to_fit(doc_bounds, viewport, cx));
    let transform = cx.read(|cx| state.read(cx).transform());
    assert_eq!(transform.zoom, 4.5);
    assert_px_eq(transform.pan.x, px(-400.));
    assert_px_eq(transform.pan.y, px(-425.));
}

#[gpui_kit::test]
fn zoom_to_fit_resets_the_view_on_empty_bounds(cx: &mut TestAppContext) {
    let state = cx.new(|_| CanvasState::new());
    state.update(cx, |s, cx| s.pan_by(point(px(50.), px(50.)), cx));
    state.update(cx, |s, cx| {
        s.zoom_to_fit(Bounds::default(), Size::default(), cx)
    });
    assert_eq!(
        cx.read(|cx| state.read(cx).transform()),
        CanvasTransform::IDENTITY
    );
}

#[gpui_kit::test]
fn hit_test_prefers_the_top_layer_and_skips_hidden_layers(cx: &mut TestAppContext) {
    let state = cx.new(|_| CanvasState::new());
    let (bottom_shape, top_shape, bottom, top) = seed_two_overlapping_rects(cx, &state);
    let center = point(px(50.), px(50.));
    assert_eq!(
        cx.read(|cx| state.read(cx).hit_test(center, px(1.))),
        Some(top_shape)
    );
    state.update(cx, |s, cx| s.set_layer_visible(top, false, cx));
    assert_eq!(
        cx.read(|cx| state.read(cx).hit_test(center, px(1.))),
        Some(bottom_shape)
    );
    state.update(cx, |s, cx| s.set_layer_visible(bottom, false, cx));
    assert_eq!(cx.read(|cx| state.read(cx).hit_test(center, px(1.))), None);
}

#[gpui_kit::test]
fn select_shape_replaces_adds_and_dedupes(cx: &mut TestAppContext) {
    let state = cx.new(|_| CanvasState::new());
    let (bottom_shape, top_shape, _, _) = seed_two_overlapping_rects(cx, &state);
    state.update(cx, |s, cx| s.select_shape(bottom_shape, false, cx));
    assert!(cx.read(|cx| state.read(cx).selected_shape_ids() == &[bottom_shape]));
    state.update(cx, |s, cx| s.select_shape(top_shape, true, cx));
    assert!(cx.read(|cx| state.read(cx).selected_shape_ids() == &[bottom_shape, top_shape]));
    state.update(cx, |s, cx| s.select_shape(top_shape, true, cx));
    assert!(cx.read(|cx| state.read(cx).selected_shape_ids() == &[bottom_shape, top_shape]));
    state.update(cx, |s, cx| s.select_shape(top_shape, false, cx));
    assert!(cx.read(|cx| state.read(cx).selected_shape_ids() == &[top_shape]));
}

#[gpui_kit::test]
fn selection_no_op_emits_nothing(cx: &mut TestAppContext) {
    let state = cx.new(|_| CanvasState::new());
    let (bottom_shape, _, _, _) = seed_two_overlapping_rects(cx, &state);
    let (events, _subscription) = collect_events(cx, &state);
    state.update(cx, |s, cx| s.select_shape(bottom_shape, false, cx));
    events.borrow_mut().clear();
    state.update(cx, |s, cx| s.select_shape(bottom_shape, false, cx));
    assert!(events.borrow().is_empty());
    state.update(cx, |s, cx| s.clear_selection(cx));
    events.borrow_mut().clear();
    state.update(cx, |s, cx| s.clear_selection(cx));
    assert!(events.borrow().is_empty());
}
