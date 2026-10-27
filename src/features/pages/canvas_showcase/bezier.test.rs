use std::cell::RefCell;
use std::rc::Rc;

use gpui_kit::{
    Bounds, Entity, Modifiers, MouseButton, Pixels, Subscription, TestAppContext, point, px, size,
};

use crate::ui::canvas::{
    CanvasEvent, CanvasPointerEvent, CanvasShape, CanvasState, CanvasTransform, ShapeId,
    ToolContext,
};

use super::BezierDemo;
use super::geometry::sample_cubic_bezier;

const SEED: [Point; 4] = [
    point(px(120.), px(260.)),
    point(px(200.), px(80.)),
    point(px(360.), px(440.)),
    point(px(460.), px(260.)),
];

type Point = gpui_kit::Point<Pixels>;

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
    let handle = cx.add_window(|_, cx| BezierDemo::new(cx));
    let demo = handle.root(cx).expect("demo root entity");
    cx.read(|cx| demo.read(cx).state.clone())
}

fn layer_shapes(
    cx: &TestAppContext,
    state: &Entity<CanvasState>,
    ix: usize,
) -> Vec<(ShapeId, CanvasShape)> {
    cx.read(|cx| state.read(cx).document().layers()[ix].shapes().to_vec())
}

fn path_points(shape: &CanvasShape) -> &[Point] {
    let CanvasShape::Path { points, .. } = shape else {
        panic!("expected a path shape");
    };
    points
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

#[gpui_kit::test]
fn seed_builds_guides_curve_and_handles_across_three_layers(cx: &mut TestAppContext) {
    let state = mounted_demo(cx);
    assert!(cx.read(|cx| state.read(cx).active_tool_id() == "bezier-edit"));
    assert!(cx.read(|cx| !state.read(cx).grid().visible));
    let guides = layer_shapes(cx, &state, 0);
    let curve = layer_shapes(cx, &state, 1);
    let handles = layer_shapes(cx, &state, 2);
    assert_eq!(guides.len(), 2);
    assert!(
        guides
            .iter()
            .all(|(_, shape)| path_points(shape).len() == 2)
    );
    assert_eq!(curve.len(), 1);
    assert_eq!(path_points(&curve[0].1).len(), 25);
    assert_eq!(handles.len(), 4);
    assert!(
        handles
            .iter()
            .all(|(_, shape)| matches!(shape, CanvasShape::Ellipse { .. }))
    );
}

#[gpui_kit::test]
fn dragging_the_first_control_point_recomputes_the_curve(cx: &mut TestAppContext) {
    let state = mounted_demo(cx);
    let control_hit = cx
        .read(|cx| state.read(cx).hit_test(point(px(200.), px(80.)), px(4.)))
        .expect("C1 handle under the press");
    let curve_id = layer_shapes(cx, &state, 1)[0].0;
    let guide_before: Vec<Vec<Point>> = layer_shapes(cx, &state, 0)
        .iter()
        .map(|(_, shape)| path_points(shape).to_vec())
        .collect();
    dispatch_down(
        &state,
        cx,
        &pointer((200., 80.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(control_hit)),
    );
    dispatch_move(
        &state,
        cx,
        &pointer((240., 100.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(control_hit)),
    );
    let curve_points = cx.read(|cx| match state.read(cx).document().shape(curve_id) {
        Some(CanvasShape::Path { points, .. }) => points.clone(),
        _ => panic!("curve stays a seeded path"),
    });
    let [p0, _, c2, p1] = SEED;
    let expected = sample_cubic_bezier(p0, point(px(240.), px(100.)), c2, p1, 24);
    for (actual, expected) in curve_points.iter().zip(&expected) {
        assert!((actual.x.as_f32() - expected.x.as_f32()).abs() < 0.01);
        assert!((actual.y.as_f32() - expected.y.as_f32()).abs() < 0.01);
    }
    let guide_after: Vec<Vec<Point>> = layer_shapes(cx, &state, 0)
        .iter()
        .map(|(_, shape)| path_points(shape).to_vec())
        .collect();
    assert_eq!(guide_after[0][0], guide_before[0][0]);
    assert!((guide_after[0][1].x.as_f32() - 240.).abs() < 0.01);
    assert!((guide_after[0][1].y.as_f32() - 100.).abs() < 0.01);
    assert_eq!(guide_after[1], guide_before[1]);
    dispatch_up(
        &state,
        cx,
        &pointer((240., 100.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(control_hit)),
    );
}

#[gpui_kit::test]
fn each_bezier_drag_move_emits_exactly_one_document_change(cx: &mut TestAppContext) {
    let state = mounted_demo(cx);
    let control_hit = cx
        .read(|cx| state.read(cx).hit_test(point(px(200.), px(80.)), px(4.)))
        .expect("C1 handle under the press");
    let (events, _subscription) = collect_events(cx, &state);
    events.borrow_mut().clear();
    dispatch_down(
        &state,
        cx,
        &pointer((200., 80.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(control_hit)),
    );
    assert!(events.borrow().is_empty());
    dispatch_move(
        &state,
        cx,
        &pointer((210., 90.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(control_hit)),
    );
    assert_eq!(&*events.borrow(), &[CanvasEvent::DocumentChanged]);
}

#[gpui_kit::test]
fn pressing_the_curve_itself_changes_nothing(cx: &mut TestAppContext) {
    let state = mounted_demo(cx);
    let revision = cx.read(|cx| state.read(cx).document_revision());
    let curve_id = layer_shapes(cx, &state, 1)[0].0;
    dispatch_down(
        &state,
        cx,
        &pointer((282.5, 260.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(curve_id)),
    );
    dispatch_move(
        &state,
        cx,
        &pointer((282.5, 260.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(curve_id)),
    );
    assert_eq!(
        cx.read(|cx| state.read(cx).document_revision()),
        revision,
        "pressing the curve must not edit the document"
    );
    assert!(cx.read(|cx| state.read(cx).selected_shape_ids().is_empty()));
}
