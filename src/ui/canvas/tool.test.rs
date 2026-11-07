// No `use super::*`: gpui's `test` proc-macro glob would make bare `#[test]`
// recurse infinitely. Import only what we need.
use std::cell::RefCell;
use std::rc::Rc;

use gpui_kit::{
    AppContext, Bounds, Context, CursorStyle, Entity, Modifiers, MouseButton, Pixels, Subscription,
    TestAppContext, point, px, size,
};

use crate::ui::canvas::{
    BrushConfig, BuiltinTool, CanvasEvent, CanvasPointerEvent, CanvasShape, CanvasState,
    CanvasTool, CanvasTransform, PanTool, SelectTool, ShapeConfig, ShapeId, ShapeKind, ShapeStyle,
    ToolContext,
};

fn pointer(screen: (f32, f32), button: MouseButton, modifiers: Modifiers) -> CanvasPointerEvent {
    let screen_position = point(px(screen.0), px(screen.1));
    CanvasPointerEvent {
        doc_position: screen_position,
        screen_position,
        button,
        click_count: 1,
        modifiers,
    }
}

fn tool_ctx(transform: CanvasTransform, hit: Option<ShapeId>) -> ToolContext {
    ToolContext {
        viewport: Bounds {
            origin: point(px(0.), px(0.)),
            size: size(px(1000.), px(1000.)),
        },
        transform,
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

fn rect_shape(bounds: Bounds<Pixels>) -> CanvasShape {
    CanvasShape::rect(bounds, ShapeStyle::default())
}

fn seeded_rect_doc(cx: &mut TestAppContext) -> (Entity<CanvasState>, ShapeId) {
    let state = cx.new(|_| CanvasState::new());
    let shape_id = state.update(cx, |s, cx| {
        let layer = s.add_layer(cx);
        s.add_shape(
            layer,
            rect_shape(Bounds {
                origin: point(px(100.), px(100.)),
                size: size(px(100.), px(100.)),
            }),
            cx,
        )
        .unwrap()
    });
    (state, shape_id)
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
fn set_tool_emits_tool_changed_and_clears_the_overlay(cx: &mut TestAppContext) {
    let state = cx.new(|_| CanvasState::new());
    state.update(cx, |s, cx| {
        s.set_tool_overlay(Some(rect_shape(Bounds::default())), cx)
    });
    assert!(cx.read(|cx| state.read(cx).tool_overlay().is_some()));
    state.update(cx, |s, cx| s.set_tool(Box::new(PanTool::new()), cx));
    assert!(cx.read(|cx| state.read(cx).tool_overlay().is_none()));
    assert!(cx.read(|cx| state.read(cx).active_tool_id() == "pan"));
    let (events, _subscription) = collect_events(cx, &state);
    state.update(cx, |s, cx| s.set_tool(Box::new(SelectTool::new()), cx));
    assert_eq!(&*events.borrow(), &[CanvasEvent::ToolChanged]);
    assert!(cx.read(|cx| state.read(cx).active_tool_id() == "select"));
}

#[gpui_kit::test]
fn builtin_tools_map_to_their_identifiers_and_cursors(cx: &mut TestAppContext) {
    let state = cx.new(|_| CanvasState::new());
    assert!(cx.read(|cx| state.read(cx).active_tool_id() == "pan"));
    state.update(cx, |s, cx| s.set_builtin_tool(BuiltinTool::Select, cx));
    assert!(cx.read(|cx| state.read(cx).active_tool_id() == "select"));
    state.update(cx, |s, cx| {
        s.set_builtin_tool(BuiltinTool::Brush(BrushConfig::default()), cx)
    });
    assert!(cx.read(|cx| state.read(cx).active_tool_id() == "brush"));
    assert_eq!(
        cx.read(|cx| state.read(cx).active_tool_cursor()),
        CursorStyle::Crosshair
    );
    state.update(cx, |s, cx| {
        s.set_builtin_tool(
            BuiltinTool::Shape(ShapeConfig {
                kind: ShapeKind::Triangle,
                style: ShapeStyle::default(),
            }),
            cx,
        )
    });
    assert!(cx.read(|cx| state.read(cx).active_tool_id() == "shape"));
    assert_eq!(
        cx.read(|cx| state.read(cx).active_tool_cursor()),
        CursorStyle::Crosshair
    );
}

#[gpui_kit::test]
fn pan_drag_sequence_moves_the_view_by_the_total_delta(cx: &mut TestAppContext) {
    let state = cx.new(|_| CanvasState::new());
    let ctx = tool_ctx(CanvasTransform::IDENTITY, None);
    dispatch_down(
        &state,
        cx,
        &pointer((10., 10.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    dispatch_move(
        &state,
        cx,
        &pointer((30., 20.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    dispatch_move(
        &state,
        cx,
        &pointer((15., 5.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    assert_px_eq(cx.read(|cx| state.read(cx).pan().x), px(5.));
    assert_px_eq(cx.read(|cx| state.read(cx).pan().y), px(-5.));
    dispatch_up(
        &state,
        cx,
        &pointer((15., 5.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    dispatch_move(
        &state,
        cx,
        &pointer((40., 40.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    assert_px_eq(cx.read(|cx| state.read(cx).pan().x), px(5.));
}

#[gpui_kit::test]
fn select_click_selects_and_drag_moves_once_per_move(cx: &mut TestAppContext) {
    let (state, shape_id) = seeded_rect_doc(cx);
    state.update(cx, |s, cx| s.set_builtin_tool(BuiltinTool::Select, cx));
    let ctx = tool_ctx(CanvasTransform::IDENTITY, Some(shape_id));
    dispatch_down(
        &state,
        cx,
        &pointer((150., 150.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    assert!(cx.read(|cx| state.read(cx).selected_shape_ids() == &[shape_id]));
    dispatch_move(
        &state,
        cx,
        &pointer((180., 150.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    let origin = cx.read(|cx| {
        state
            .read(cx)
            .document()
            .shape(shape_id)
            .unwrap()
            .doc_bounds()
            .origin
    });
    assert_px_eq(origin.x, px(130.));
    assert_px_eq(origin.y, px(100.));
    dispatch_move(
        &state,
        cx,
        &pointer((160., 170.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    let origin = cx.read(|cx| {
        state
            .read(cx)
            .document()
            .shape(shape_id)
            .unwrap()
            .doc_bounds()
            .origin
    });
    assert_px_eq(origin.x, px(110.));
    assert_px_eq(origin.y, px(120.));
    dispatch_up(
        &state,
        cx,
        &pointer((160., 170.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    dispatch_move(
        &state,
        cx,
        &pointer((200., 200.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    let origin = cx.read(|cx| {
        state
            .read(cx)
            .document()
            .shape(shape_id)
            .unwrap()
            .doc_bounds()
            .origin
    });
    assert_px_eq(origin.x, px(110.));
}

#[gpui_kit::test]
fn select_click_on_empty_canvas_clears_selection(cx: &mut TestAppContext) {
    let (state, shape_id) = seeded_rect_doc(cx);
    state.update(cx, |s, cx| s.select_shape(shape_id, false, cx));
    state.update(cx, |s, cx| s.set_builtin_tool(BuiltinTool::Select, cx));
    let ctx = tool_ctx(CanvasTransform::IDENTITY, None);
    dispatch_down(
        &state,
        cx,
        &pointer((500., 500.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    assert!(cx.read(|cx| state.read(cx).selected_shape_ids().is_empty()));
}

#[gpui_kit::test]
fn shift_click_selects_two_shapes_and_drag_moves_them_together(cx: &mut TestAppContext) {
    let state = cx.new(|_| CanvasState::new());
    let (first, second) = state.update(cx, |s, cx| {
        let layer = s.add_layer(cx);
        let first = s
            .add_shape(
                layer,
                rect_shape(Bounds {
                    origin: point(px(100.), px(100.)),
                    size: size(px(50.), px(50.)),
                }),
                cx,
            )
            .unwrap();
        let second = s
            .add_shape(
                layer,
                rect_shape(Bounds {
                    origin: point(px(200.), px(100.)),
                    size: size(px(50.), px(50.)),
                }),
                cx,
            )
            .unwrap();
        (first, second)
    });
    state.update(cx, |s, cx| s.set_builtin_tool(BuiltinTool::Select, cx));

    dispatch_down(
        &state,
        cx,
        &pointer((120., 120.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(CanvasTransform::IDENTITY, Some(first)),
    );
    dispatch_up(
        &state,
        cx,
        &pointer((120., 120.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(CanvasTransform::IDENTITY, None),
    );
    dispatch_down(
        &state,
        cx,
        &pointer((220., 120.), MouseButton::Left, Modifiers::shift()),
        &tool_ctx(CanvasTransform::IDENTITY, Some(second)),
    );
    assert!(cx.read(|cx| state.read(cx).selected_shape_ids() == &[first, second]));
    dispatch_up(
        &state,
        cx,
        &pointer((220., 120.), MouseButton::Left, Modifiers::shift()),
        &tool_ctx(CanvasTransform::IDENTITY, None),
    );

    // Dragging the already-selected second shape moves both by the same delta.
    dispatch_down(
        &state,
        cx,
        &pointer((220., 120.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(CanvasTransform::IDENTITY, Some(second)),
    );
    dispatch_move(
        &state,
        cx,
        &pointer((250., 130.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(CanvasTransform::IDENTITY, None),
    );
    dispatch_up(
        &state,
        cx,
        &pointer((250., 130.), MouseButton::Left, Modifiers::none()),
        &tool_ctx(CanvasTransform::IDENTITY, None),
    );
    let origins = cx.read(|cx| {
        let s = state.read(cx);
        (
            s.document().shape(first).unwrap().doc_bounds().origin,
            s.document().shape(second).unwrap().doc_bounds().origin,
        )
    });
    assert_px_eq(origins.0.x, px(130.));
    assert_px_eq(origins.0.y, px(110.));
    assert_px_eq(origins.1.x, px(230.));
    assert_px_eq(origins.1.y, px(110.));

    // Shift+click on empty canvas keeps the selection.
    dispatch_down(
        &state,
        cx,
        &pointer((900., 900.), MouseButton::Left, Modifiers::shift()),
        &tool_ctx(CanvasTransform::IDENTITY, None),
    );
    assert!(cx.read(|cx| state.read(cx).selected_shape_ids() == &[first, second]));
}

#[gpui_kit::test]
fn brush_drag_commits_one_path_with_deduped_points(cx: &mut TestAppContext) {
    let state = cx.new(|_| CanvasState::new());
    state.update(cx, |s, cx| {
        s.add_layer(cx);
        s.set_builtin_tool(BuiltinTool::Brush(BrushConfig::default()), cx);
    });
    let (events, _subscription) = collect_events(cx, &state);
    let ctx = tool_ctx(CanvasTransform::IDENTITY, None);
    dispatch_down(
        &state,
        cx,
        &pointer((10., 10.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    dispatch_move(
        &state,
        cx,
        &pointer((11., 10.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    dispatch_move(
        &state,
        cx,
        &pointer((40., 40.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    assert!(cx.read(|cx| state.read(cx).tool_overlay().is_some()));
    events.borrow_mut().clear();
    dispatch_up(
        &state,
        cx,
        &pointer((40., 40.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    assert!(cx.read(|cx| state.read(cx).tool_overlay().is_none()));
    let path_count = cx.read(|cx| {
        state
            .read(cx)
            .document()
            .shapes()
            .filter(|(_, _, s)| matches!(s, CanvasShape::Path { .. }))
            .count()
    });
    assert_eq!(path_count, 1);
    let point_count = cx.read(|cx| {
        state
            .read(cx)
            .document()
            .shapes()
            .find_map(|(_, _, s)| match s {
                CanvasShape::Path { points, .. } => Some(points.len()),
                _ => None,
            })
            .unwrap()
    });
    assert_eq!(point_count, 2);
    assert_eq!(&*events.borrow(), &[CanvasEvent::DocumentChanged]);
}

#[gpui_kit::test]
fn brush_dedup_measures_screen_distance_through_the_transform(cx: &mut TestAppContext) {
    let state = cx.new(|_| CanvasState::new());
    state.update(cx, |s, cx| {
        s.add_layer(cx);
        s.set_builtin_tool(BuiltinTool::Brush(Default::default()), cx);
    });
    // Zoom 2: 0.75 doc px is 1.5 screen px (kept), 0.4 doc px is 0.8 screen
    // px (dropped). Comparing raw doc distances would drop the 0.75 sample.
    let ctx = tool_ctx(CanvasTransform::new(point(px(0.), px(0.)), 2.), None);
    dispatch_down(
        &state,
        cx,
        &pointer((0., 0.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    dispatch_move(
        &state,
        cx,
        &pointer((0.4, 0.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    dispatch_move(
        &state,
        cx,
        &pointer((0.75, 0.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    dispatch_move(
        &state,
        cx,
        &pointer((1.5, 0.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    dispatch_up(
        &state,
        cx,
        &pointer((1.5, 0.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    let point_count = cx.read(|cx| {
        state
            .read(cx)
            .document()
            .shapes()
            .find_map(|(_, _, s)| match s {
                CanvasShape::Path { points, .. } => Some(points.len()),
                _ => None,
            })
            .unwrap()
    });
    assert_eq!(point_count, 3);
}

#[gpui_kit::test]
fn right_button_up_does_not_commit_an_in_flight_left_stroke(cx: &mut TestAppContext) {
    let state = cx.new(|_| CanvasState::new());
    state.update(cx, |s, cx| {
        s.add_layer(cx);
        s.set_builtin_tool(BuiltinTool::Brush(Default::default()), cx);
    });
    let ctx = tool_ctx(CanvasTransform::IDENTITY, None);
    dispatch_down(
        &state,
        cx,
        &pointer((10., 10.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    dispatch_move(
        &state,
        cx,
        &pointer((40., 40.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    dispatch_up(
        &state,
        cx,
        &pointer((40., 40.), MouseButton::Right, Modifiers::none()),
        &ctx,
    );
    assert!(cx.read(|cx| state.read(cx).tool_overlay().is_some()));
    let committed = cx.read(|cx| state.read(cx).document().shapes().count());
    assert_eq!(committed, 0);
    dispatch_up(
        &state,
        cx,
        &pointer((40., 40.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    assert!(cx.read(|cx| state.read(cx).tool_overlay().is_none()));
    let committed = cx.read(|cx| state.read(cx).document().shapes().count());
    assert_eq!(committed, 1);
}

#[gpui_kit::test]
fn brush_commit_uses_the_configured_width_and_color(cx: &mut TestAppContext) {
    let config = BrushConfig {
        width: px(5.),
        ..BrushConfig::default()
    };
    let state = cx.new(|_| CanvasState::new());
    state.update(cx, |s, cx| {
        s.add_layer(cx);
        s.set_builtin_tool(BuiltinTool::Brush(config), cx);
    });
    let ctx = tool_ctx(CanvasTransform::IDENTITY, None);
    dispatch_down(
        &state,
        cx,
        &pointer((10., 10.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    dispatch_up(
        &state,
        cx,
        &pointer((10., 10.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    let (width, color) = cx.read(|cx| {
        let committed = state
            .read(cx)
            .document()
            .shapes()
            .map(|(_, _, shape)| shape)
            .next()
            .expect("one committed stroke");
        let CanvasShape::Path { style, .. } = committed else {
            panic!("path variant");
        };
        (style.width, style.color)
    });
    assert_px_eq(width, px(5.));
    assert_eq!(color, config.color);
}

#[gpui_kit::test]
fn shape_drag_commits_the_requested_kind_with_normalized_bounds(cx: &mut TestAppContext) {
    let state = cx.new(|_| CanvasState::new());
    state.update(cx, |s, cx| {
        s.add_layer(cx);
        s.set_builtin_tool(
            BuiltinTool::Shape(ShapeConfig {
                kind: ShapeKind::Triangle,
                style: ShapeStyle::default(),
            }),
            cx,
        );
    });
    let (events, _subscription) = collect_events(cx, &state);
    let ctx = tool_ctx(CanvasTransform::IDENTITY, None);
    // Drag bottom-right to top-left: the committed bounds must normalize.
    dispatch_down(
        &state,
        cx,
        &pointer((110., 70.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    dispatch_move(
        &state,
        cx,
        &pointer((10., 20.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    assert!(cx.read(|cx| state.read(cx).tool_overlay().is_some()));
    events.borrow_mut().clear();
    dispatch_up(
        &state,
        cx,
        &pointer((10., 20.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    assert!(cx.read(|cx| state.read(cx).tool_overlay().is_none()));
    let bounds = cx.read(|cx| {
        let committed = state
            .read(cx)
            .document()
            .shapes()
            .map(|(_, _, shape)| shape)
            .next()
            .expect("one committed shape");
        let CanvasShape::Triangle { bounds, .. } = committed else {
            panic!("triangle variant");
        };
        *bounds
    });
    assert_px_eq(bounds.origin.x, px(10.));
    assert_px_eq(bounds.origin.y, px(20.));
    assert_px_eq(bounds.size.width, px(100.));
    assert_px_eq(bounds.size.height, px(50.));
    assert_eq!(&*events.borrow(), &[CanvasEvent::DocumentChanged]);
}

#[gpui_kit::test]
fn shape_click_without_drag_commits_nothing(cx: &mut TestAppContext) {
    let state = cx.new(|_| CanvasState::new());
    state.update(cx, |s, cx| {
        s.add_layer(cx);
        s.set_builtin_tool(
            BuiltinTool::Shape(ShapeConfig {
                kind: ShapeKind::Rect,
                style: ShapeStyle::default(),
            }),
            cx,
        );
    });
    let ctx = tool_ctx(CanvasTransform::IDENTITY, None);
    dispatch_down(
        &state,
        cx,
        &pointer((50., 50.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    dispatch_up(
        &state,
        cx,
        &pointer((50., 50.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    assert!(cx.read(|cx| state.read(cx).tool_overlay().is_none()));
    let count = cx.read(|cx| state.read(cx).document().shapes().count());
    assert_eq!(count, 0);
}

#[gpui_kit::test]
fn set_tool_inside_dispatch_wins_over_the_restored_tool(cx: &mut TestAppContext) {
    struct Replacer;
    impl CanvasTool for Replacer {
        fn id(&self) -> &'static str {
            "replacer"
        }
        fn pointer_down(
            &mut self,
            _: &CanvasPointerEvent,
            _: &ToolContext,
            state: &mut CanvasState,
            cx: &mut Context<CanvasState>,
        ) {
            state.set_tool(Box::new(SelectTool::new()), cx);
        }
    }
    let state = cx.new(|_| CanvasState::new());
    state.update(cx, |s, cx| s.set_tool(Box::new(Replacer), cx));
    let ctx = tool_ctx(CanvasTransform::IDENTITY, None);
    dispatch_down(
        &state,
        cx,
        &pointer((0., 0.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    assert!(cx.read(|cx| state.read(cx).active_tool_id() == "select"));
}

#[gpui_kit::test]
fn dispatch_restores_the_tool_when_the_callback_keeps_it(cx: &mut TestAppContext) {
    let state = cx.new(|_| CanvasState::new());
    let ctx = tool_ctx(CanvasTransform::IDENTITY, None);
    dispatch_down(
        &state,
        cx,
        &pointer((0., 0.), MouseButton::Left, Modifiers::none()),
        &ctx,
    );
    assert!(cx.read(|cx| state.read(cx).active_tool_id() == "pan"));
}
