use std::cell::RefCell;
use std::rc::Rc;

use gpui_kit::{
    Bounds, Entity, Modifiers, MouseButton, Pixels, Subscription, TestAppContext, point, px, size,
};

use crate::ui::canvas::{
    CanvasEvent, CanvasPointerEvent, CanvasShape, CanvasState, CanvasTransform, ShapeId,
    ToolContext,
};

use super::NodeGraphDemo;

const NODE_W: f32 = 120.;
const NODE_H: f32 = 56.;

const SEED_NODES: [(f32, f32); 4] = [(100., 120.), (320., 100.), (330., 260.), (560., 170.)];

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
            size: size(px(800.), px(500.)),
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
    let handle = cx.add_window(|_, cx| NodeGraphDemo::new(cx));
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

fn node_rect_id(cx: &TestAppContext, state: &Entity<CanvasState>, node_ix: usize) -> ShapeId {
    layer_shapes(cx, state, 1)[node_ix * 2].0
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

fn path_points_of(
    cx: &TestAppContext,
    state: &Entity<CanvasState>,
    id: ShapeId,
) -> Vec<gpui_kit::Point<Pixels>> {
    cx.read(|cx| match state.read(cx).document().shape(id) {
        Some(CanvasShape::Path { points, .. }) => points.clone(),
        _ => panic!("edge stays a path"),
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
fn seed_builds_four_nodes_and_four_bezier_edges(cx: &mut TestAppContext) {
    let state = mounted_demo(cx);
    assert!(cx.read(|cx| state.read(cx).active_tool_id() == "node-graph"));
    assert!(cx.read(|cx| !state.read(cx).grid().visible));
    let edges = layer_shapes(cx, &state, 0);
    let nodes = layer_shapes(cx, &state, 1);
    assert_eq!(edges.len(), 4);
    assert!(edges.iter().all(|(_, shape)| match shape {
        CanvasShape::Path { points, .. } => points.len() == 21,
        _ => false,
    }));
    assert_eq!(nodes.len(), 8);
    assert_eq!(
        nodes
            .iter()
            .filter(|(_, shape)| matches!(shape, CanvasShape::Rect { .. }))
            .count(),
        4
    );
    assert_eq!(
        nodes
            .iter()
            .filter(|(_, shape)| matches!(shape, CanvasShape::Text { .. }))
            .count(),
        4
    );
    let (source, target) = (SEED_NODES[0], SEED_NODES[1]);
    let first_edge = path_points_of(cx, &state, edges[0].0);
    assert_px_eq(first_edge[0].x, px(source.0 + NODE_W));
    assert_px_eq(first_edge[0].y, px(source.1 + NODE_H / 2.));
    assert_px_eq(first_edge[20].x, px(target.0));
    assert_px_eq(first_edge[20].y, px(target.1 + NODE_H / 2.));
}

#[gpui_kit::test]
fn dragging_a_node_moves_its_shapes_and_resamples_its_edges(cx: &mut TestAppContext) {
    let state = mounted_demo(cx);
    let input_rect = node_rect_id(cx, &state, 0);
    let input_label = layer_shapes(cx, &state, 1)[1].0;
    let filter_rect = node_rect_id(cx, &state, 1);
    let edge_zero = layer_shapes(cx, &state, 0)[0].0;
    dispatch_down(
        &state,
        cx,
        &pointer((110., 130.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(input_rect)),
    );
    dispatch_move(
        &state,
        cx,
        &pointer((150., 150.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(input_rect)),
    );
    let input_bounds = shape_bounds(cx, &state, input_rect);
    assert_px_eq(input_bounds.origin.x, px(SEED_NODES[0].0 + 40.));
    assert_px_eq(input_bounds.origin.y, px(SEED_NODES[0].1 + 20.));
    let label_position = cx.read(|cx| match state.read(cx).document().shape(input_label) {
        Some(CanvasShape::Text { position, .. }) => *position,
        _ => panic!("label stays text"),
    });
    assert_px_eq(label_position.x, px(SEED_NODES[0].0 + 40. + 8.));
    assert_px_eq(label_position.y, px(SEED_NODES[0].1 + 20. + 8.));
    let filter_bounds = shape_bounds(cx, &state, filter_rect);
    assert_px_eq(filter_bounds.origin.x, px(SEED_NODES[1].0));
    assert_px_eq(filter_bounds.origin.y, px(SEED_NODES[1].1));
    let first_edge = path_points_of(cx, &state, edge_zero);
    assert_px_eq(first_edge[0].x, px(SEED_NODES[0].0 + 40. + NODE_W));
    assert_px_eq(first_edge[0].y, px(SEED_NODES[0].1 + 20. + NODE_H / 2.));
    dispatch_up(
        &state,
        cx,
        &pointer((150., 150.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(input_rect)),
    );
}

#[gpui_kit::test]
fn each_node_drag_move_emits_exactly_one_document_change(cx: &mut TestAppContext) {
    let state = mounted_demo(cx);
    let input_rect = node_rect_id(cx, &state, 0);
    dispatch_down(
        &state,
        cx,
        &pointer((110., 130.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(input_rect)),
    );
    let (events, _subscription) = collect_events(cx, &state);
    events.borrow_mut().clear();
    dispatch_move(
        &state,
        cx,
        &pointer((120., 130.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(input_rect)),
    );
    assert_eq!(&*events.borrow(), &[CanvasEvent::DocumentChanged]);
}

#[gpui_kit::test]
fn rubber_band_selects_exactly_the_intersected_nodes(cx: &mut TestAppContext) {
    let state = mounted_demo(cx);
    dispatch_down(
        &state,
        cx,
        &pointer((90., 90.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(None),
    );
    dispatch_move(
        &state,
        cx,
        &pointer((450., 200.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(None),
    );
    assert!(cx.read(|cx| state.read(cx).tool_overlay().is_some()));
    dispatch_up(
        &state,
        cx,
        &pointer((450., 200.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(None),
    );
    let selected = cx.read(|cx| state.read(cx).selected_shape_ids().to_vec());
    assert_eq!(
        selected,
        vec![node_rect_id(cx, &state, 0), node_rect_id(cx, &state, 1)],
        "Input and Filter in seed order"
    );
    assert!(cx.read(|cx| state.read(cx).tool_overlay().is_none()));
}

#[gpui_kit::test]
fn shift_band_adds_to_the_existing_selection(cx: &mut TestAppContext) {
    let state = mounted_demo(cx);
    let filter_rect = node_rect_id(cx, &state, 1);
    dispatch_down(
        &state,
        cx,
        &pointer((330., 110.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(filter_rect)),
    );
    dispatch_up(
        &state,
        cx,
        &pointer((330., 110.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(filter_rect)),
    );
    assert!(cx.read(|cx| state.read(cx).selected_shape_ids() == &[filter_rect]));
    dispatch_down(
        &state,
        cx,
        &pointer((90., 90.), MouseButton::Left, Modifiers::shift()),
        &tool_ctx(None),
    );
    dispatch_move(
        &state,
        cx,
        &pointer((450., 200.), MouseButton::Left, Modifiers::shift()),
        &tool_ctx(None),
    );
    dispatch_up(
        &state,
        cx,
        &pointer((450., 200.), MouseButton::Left, Modifiers::shift()),
        &tool_ctx(None),
    );
    let selected = cx.read(|cx| state.read(cx).selected_shape_ids().to_vec());
    assert_eq!(selected.len(), 2);
    assert!(selected.contains(&filter_rect));
    assert!(selected.contains(&node_rect_id(cx, &state, 0)));
}

#[gpui_kit::test]
fn band_on_empty_space_without_shift_clears_selection(cx: &mut TestAppContext) {
    let state = mounted_demo(cx);
    let input_rect = node_rect_id(cx, &state, 0);
    dispatch_down(
        &state,
        cx,
        &pointer((110., 130.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(input_rect)),
    );
    dispatch_up(
        &state,
        cx,
        &pointer((110., 130.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(Some(input_rect)),
    );
    assert!(cx.read(|cx| state.read(cx).selected_shape_ids() == &[input_rect]));
    dispatch_down(
        &state,
        cx,
        &pointer((480., 300.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(None),
    );
    assert!(cx.read(|cx| state.read(cx).selected_shape_ids().is_empty()));
    dispatch_move(
        &state,
        cx,
        &pointer((500., 320.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(None),
    );
    dispatch_up(
        &state,
        cx,
        &pointer((500., 320.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(None),
    );
    assert!(cx.read(|cx| state.read(cx).selected_shape_ids().is_empty()));
    assert!(cx.read(|cx| state.read(cx).tool_overlay().is_none()));
}
