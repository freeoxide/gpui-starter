use std::cell::RefCell;
use std::rc::Rc;

use gpui_kit::{
    Bounds, Entity, Modifiers, MouseButton, Pixels, Subscription, TestAppContext, point, px, size,
};

use crate::ui::canvas::{
    CanvasEvent, CanvasPointerEvent, CanvasShape, CanvasState, CanvasTransform, ShapeId,
    ToolContext,
};

use super::ShapesDemo;
use super::geometry::{handle_positions, resize_bounds, scale_points};

const SEED_RECT: Bounds<Pixels> = Bounds {
    origin: point(px(80.), px(80.)),
    size: size(px(180.), px(100.)),
};

fn pointer(doc: (f32, f32), button: MouseButton, modifiers: Modifiers) -> CanvasPointerEvent {
    let position = point(px(doc.0), px(doc.1));
    CanvasPointerEvent {
        screen_position: position,
        doc_position: position,
        button,
        click_count: 1,
        modifiers,
    }
}

fn tool_ctx(hit: Option<ShapeId>) -> ToolContext {
    ToolContext {
        viewport: Bounds {
            origin: point(px(0.), px(0.)),
            size: size(px(600.), px(500.)),
        },
        transform: CanvasTransform::IDENTITY,
        hit,
    }
}

fn dispatch_down(
    state: &Entity<CanvasState>,
    cx: &mut TestAppContext,
    event: &CanvasPointerEvent,
    ctx: &ToolContext,
) {
    state.update(cx, |s, cx| {
        s.dispatch_tool_event(cx, |tool, s, cx| tool.pointer_down(event, ctx, s, cx))
    });
}

fn dispatch_move(
    state: &Entity<CanvasState>,
    cx: &mut TestAppContext,
    event: &CanvasPointerEvent,
    ctx: &ToolContext,
) {
    state.update(cx, |s, cx| {
        s.dispatch_tool_event(cx, |tool, s, cx| tool.pointer_move(event, ctx, s, cx))
    });
}

fn dispatch_up(
    state: &Entity<CanvasState>,
    cx: &mut TestAppContext,
    event: &CanvasPointerEvent,
    ctx: &ToolContext,
) {
    state.update(cx, |s, cx| {
        s.dispatch_tool_event(cx, |tool, s, cx| tool.pointer_up(event, ctx, s, cx))
    });
}

fn mounted_demo(cx: &mut TestAppContext) -> Entity<CanvasState> {
    cx.update(gpui_kit::component::theme::init);
    let handle = cx.add_window(|_, cx| ShapesDemo::new(cx));
    let demo = handle.root(cx).expect("demo root entity");
    cx.read(|cx| demo.read(cx).state.clone())
}

fn hit_at(cx: &TestAppContext, state: &Entity<CanvasState>, doc: (f32, f32)) -> Option<ShapeId> {
    cx.read(|cx| state.read(cx).hit_test(point(px(doc.0), px(doc.1)), px(4.)))
}

fn handle_shapes(cx: &TestAppContext, state: &Entity<CanvasState>) -> Vec<(ShapeId, CanvasShape)> {
    cx.read(|cx| state.read(cx).document().layers()[1].shapes().to_vec())
}

fn shape_bounds(cx: &TestAppContext, state: &Entity<CanvasState>, id: ShapeId) -> Bounds<Pixels> {
    cx.read(|cx| {
        state
            .read(cx)
            .document()
            .shape(id)
            .expect("shape still seeded")
            .doc_bounds()
    })
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

fn assert_px_eq(actual: Pixels, expected: Pixels) {
    assert!(
        (actual.as_f32() - expected.as_f32()).abs() < 0.01,
        "expected ~{expected:?}, got {actual:?}"
    );
}

#[gpui_kit::test]
fn seed_builds_two_layers_four_shapes_and_the_shape_edit_tool(cx: &mut TestAppContext) {
    let state = mounted_demo(cx);
    assert!(cx.read(|cx| state.read(cx).active_tool_id() == "shape-edit"));
    assert!(cx.read(|cx| !state.read(cx).grid().visible));
    let (layer_count, content_shapes, handles) = cx.read(|cx| {
        let doc = state.read(cx).document();
        (
            doc.layers().len(),
            doc.layers().first().map(|l| l.shapes().len()).unwrap_or(0),
            doc.layers().get(1).map(|l| l.shapes().len()).unwrap_or(0),
        )
    });
    assert_eq!(layer_count, 2);
    assert_eq!(content_shapes, 4);
    assert_eq!(handles, 0);
}

#[gpui_kit::test]
fn clicking_a_shape_selects_it_and_makes_eight_handles(cx: &mut TestAppContext) {
    let state = mounted_demo(cx);
    let rect_id = hit_at(cx, &state, (100., 100.)).expect("rect under the click");
    dispatch_down(
        &state,
        cx,
        &pointer((100., 100.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(rect_id)),
    );
    assert!(cx.read(|cx| state.read(cx).selected_shape_ids() == &[rect_id]));
    let handles = handle_shapes(cx, &state);
    assert_eq!(handles.len(), 8);
    let expected_origins = [
        (76., 76.),
        (166., 76.),
        (256., 76.),
        (256., 126.),
        (256., 176.),
        (166., 176.),
        (76., 176.),
        (76., 126.),
    ];
    for ((_, shape), (x, y)) in handles.iter().zip(expected_origins) {
        let CanvasShape::Rect { bounds, .. } = shape else {
            panic!("handle must be a rect");
        };
        assert_px_eq(bounds.origin.x, px(x));
        assert_px_eq(bounds.origin.y, px(y));
        assert_px_eq(bounds.size.width, px(8.));
        assert_px_eq(bounds.size.height, px(8.));
    }
}

#[gpui_kit::test]
fn shift_click_grows_the_selection_and_hides_handles(cx: &mut TestAppContext) {
    let state = mounted_demo(cx);
    let rect_id = hit_at(cx, &state, (100., 100.)).expect("rect under the click");
    dispatch_down(
        &state,
        cx,
        &pointer((100., 100.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(rect_id)),
    );
    let ellipse_id = hit_at(cx, &state, (400., 160.)).expect("ellipse under the click");
    dispatch_down(
        &state,
        cx,
        &pointer((400., 160.), MouseButton::Left, Modifiers::shift()),
        &tool_ctx(Some(ellipse_id)),
    );
    let selected = cx.read(|cx| state.read(cx).selected_shape_ids().to_vec());
    assert_eq!(selected.len(), 2);
    assert!(selected.contains(&rect_id));
    assert!(selected.contains(&ellipse_id));
    assert_eq!(handle_shapes(cx, &state).len(), 0);
}

#[gpui_kit::test]
fn move_drag_translates_the_selection_by_the_delta(cx: &mut TestAppContext) {
    let state = mounted_demo(cx);
    let rect_id = hit_at(cx, &state, (100., 100.)).expect("rect under the click");
    dispatch_down(
        &state,
        cx,
        &pointer((100., 100.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(rect_id)),
    );
    dispatch_move(
        &state,
        cx,
        &pointer((130., 110.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(rect_id)),
    );
    let origin = shape_bounds(cx, &state, rect_id).origin;
    assert_px_eq(origin.x, SEED_RECT.origin.x + px(30.));
    assert_px_eq(origin.y, SEED_RECT.origin.y + px(10.));
    dispatch_up(
        &state,
        cx,
        &pointer((130., 110.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(rect_id)),
    );
}

#[gpui_kit::test]
fn each_drag_move_emits_exactly_one_document_change(cx: &mut TestAppContext) {
    let state = mounted_demo(cx);
    let rect_id = hit_at(cx, &state, (100., 100.)).expect("rect under the click");
    dispatch_down(
        &state,
        cx,
        &pointer((100., 100.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(rect_id)),
    );
    let (events, _subscription) = collect_events(cx, &state);
    events.borrow_mut().clear();
    dispatch_move(
        &state,
        cx,
        &pointer((130., 110.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(rect_id)),
    );
    assert_eq!(&*events.borrow(), &[CanvasEvent::DocumentChanged]);
    dispatch_move(
        &state,
        cx,
        &pointer((150., 130.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(rect_id)),
    );
    assert_eq!(events.borrow().len(), 2);
}

#[gpui_kit::test]
fn resize_drag_through_the_top_left_handle_clamps_at_the_minimum(cx: &mut TestAppContext) {
    let state = mounted_demo(cx);
    let rect_id = hit_at(cx, &state, (100., 100.)).expect("rect under the click");
    dispatch_down(
        &state,
        cx,
        &pointer((100., 100.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(rect_id)),
    );
    let handle_id = hit_at(cx, &state, (80., 80.)).expect("top-left handle");
    dispatch_down(
        &state,
        cx,
        &pointer((80., 80.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(handle_id)),
    );
    dispatch_move(
        &state,
        cx,
        &pointer((60., 60.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(handle_id)),
    );
    let bounds = shape_bounds(cx, &state, rect_id);
    assert_px_eq(bounds.origin.x, px(60.));
    assert_px_eq(bounds.origin.y, px(60.));
    assert_px_eq(bounds.size.width, px(200.));
    assert_px_eq(bounds.size.height, px(120.));
    dispatch_move(
        &state,
        cx,
        &pointer((300., 200.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(handle_id)),
    );
    let bounds = shape_bounds(cx, &state, rect_id);
    assert_px_eq(bounds.origin.x, px(252.));
    assert_px_eq(bounds.origin.y, px(172.));
    assert_px_eq(bounds.size.width, px(8.));
    assert_px_eq(bounds.size.height, px(8.));
    dispatch_up(
        &state,
        cx,
        &pointer((300., 200.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(handle_id)),
    );
}

#[gpui_kit::test]
fn clicking_empty_space_clears_selection_and_removes_handles(cx: &mut TestAppContext) {
    let state = mounted_demo(cx);
    let rect_id = hit_at(cx, &state, (100., 100.)).expect("rect under the click");
    dispatch_down(
        &state,
        cx,
        &pointer((100., 100.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(rect_id)),
    );
    assert_eq!(handle_shapes(cx, &state).len(), 8);
    assert!(hit_at(cx, &state, (500., 60.)).is_none());
    dispatch_down(
        &state,
        cx,
        &pointer((500., 60.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(None),
    );
    assert!(cx.read(|cx| state.read(cx).selected_shape_ids().is_empty()));
    assert_eq!(handle_shapes(cx, &state).len(), 0);
}

#[gpui_kit::test]
fn path_resize_does_not_compound_across_moves(cx: &mut TestAppContext) {
    let seeded_points = [
        point(px(100.), px(430.)),
        point(px(160.), px(390.)),
        point(px(220.), px(450.)),
        point(px(280.), px(410.)),
    ];
    let path_id = |cx: &TestAppContext, state: &Entity<CanvasState>| {
        cx.read(|cx| {
            state.read(cx).document().layers()[0]
                .shapes()
                .iter()
                .find(|(_, shape)| matches!(shape, CanvasShape::Path { .. }))
                .expect("seeded path exists")
                .0
        })
    };
    let drag_top_left =
        |cx: &mut TestAppContext, state: &Entity<CanvasState>, stops: &[(f32, f32)]| {
            let path = path_id(cx, state);
            dispatch_down(
                state,
                cx,
                &pointer((160., 390.), MouseButton::Left, Modifiers::none()),
                &tool_ctx(cx.read(|cx| state.read(cx).hit_test(point(px(160.), px(390.)), px(4.)))),
            );
            let start_bounds = cx.read(|cx| {
                state
                    .read(cx)
                    .document()
                    .shape(path)
                    .expect("path still seeded")
                    .doc_bounds()
            });
            let handle = cx.read(|cx| {
                state
                    .read(cx)
                    .hit_test(handle_positions(start_bounds)[0], px(4.))
                    .expect("top-left handle")
            });
            dispatch_down(
                state,
                cx,
                &pointer(
                    (
                        handle_positions(start_bounds)[0].x.as_f32(),
                        handle_positions(start_bounds)[0].y.as_f32(),
                    ),
                    MouseButton::Left,
                    Modifiers::none(),
                ),
                &tool_ctx(Some(handle)),
            );
            for stop in stops {
                dispatch_move(
                    state,
                    cx,
                    &pointer(*stop, MouseButton::Left, Modifiers::none()),
                    &tool_ctx(Some(handle)),
                );
            }
            dispatch_up(
                state,
                cx,
                &pointer(
                    *stops.last().expect("at least one stop"),
                    MouseButton::Left,
                    Modifiers::none(),
                ),
                &tool_ctx(Some(handle)),
            );
            cx.read(|cx| match state.read(cx).document().shape(path) {
                Some(CanvasShape::Path { points, .. }) => points.clone(),
                _ => panic!("path stays a path"),
            })
        };

    let one_move = {
        let state = mounted_demo(cx);
        drag_top_left(cx, &state, &[(60., 360.)])
    };
    let two_moves = {
        let state = mounted_demo(cx);
        drag_top_left(cx, &state, &[(80., 375.), (60., 360.)])
    };
    assert_eq!(one_move.len(), seeded_points.len());
    for (direct, staged) in one_move.iter().zip(&two_moves) {
        assert!(
            (direct.x.as_f32() - staged.x.as_f32()).abs() < 0.01
                && (direct.y.as_f32() - staged.y.as_f32()).abs() < 0.01,
            "staged drag must match the direct drag: {direct:?} vs {staged:?}"
        );
    }

    let state = mounted_demo(cx);
    let path = path_id(cx, &state);
    dispatch_down(
        &state,
        cx,
        &pointer((160., 390.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(path)),
    );
    let start_bounds = cx.read(|cx| {
        state
            .read(cx)
            .document()
            .shape(path)
            .expect("path still seeded")
            .doc_bounds()
    });
    let handle = cx.read(|cx| {
        state
            .read(cx)
            .hit_test(handle_positions(start_bounds)[0], px(4.))
            .expect("top-left handle")
    });
    dispatch_down(
        &state,
        cx,
        &pointer((99., 389.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(handle)),
    );
    dispatch_move(
        &state,
        cx,
        &pointer((60., 360.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(handle)),
    );
    let resized = cx.read(|cx| match state.read(cx).document().shape(path) {
        Some(CanvasShape::Path { points, .. }) => points.clone(),
        _ => panic!("path stays a path"),
    });
    let expected = scale_points(
        &seeded_points,
        start_bounds,
        resize_bounds(start_bounds, 0, point(px(60.), px(360.))),
    );
    for (actual, expected) in resized.iter().zip(&expected) {
        assert!(
            (actual.x.as_f32() - expected.x.as_f32()).abs() < 0.01
                && (actual.y.as_f32() - expected.y.as_f32()).abs() < 0.01,
            "expected {expected:?}, got {actual:?}"
        );
    }
}
