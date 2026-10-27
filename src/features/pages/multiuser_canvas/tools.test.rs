use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use collab_protocol::{ShapeKey, WireOp};
use gpui_kit::prelude::*;
use gpui_kit::{Bounds, Entity, Modifiers, MouseButton, TestAppContext, point, px, size};

use super::super::session::{CollabSession, Mode, NetLink};
use super::{PenTool, SelectTool};
use crate::ui::canvas::{
    CanvasPointerEvent, CanvasShape, CanvasState, CanvasTool, CanvasTransform, ShapeId, ShapeStyle,
    ToolContext,
};

fn harness(cx: &TestAppContext) -> (Entity<CanvasState>, Entity<CollabSession>) {
    cx.update(|cx| {
        let canvas = cx.new(|_| CanvasState::new());
        let session = cx.new(|cx| CollabSession::new(canvas.clone(), cx));
        (canvas, session)
    })
}

fn tool_ctx(hit: Option<ShapeId>) -> ToolContext {
    ToolContext {
        viewport: Bounds {
            origin: point(px(0.), px(0.)),
            size: size(px(600.), px(400.)),
        },
        transform: CanvasTransform::IDENTITY,
        hit,
    }
}

fn event(x: f32, y: f32) -> CanvasPointerEvent {
    CanvasPointerEvent {
        screen_position: point(px(x), px(y)),
        doc_position: point(px(x), px(y)),
        button: MouseButton::Left,
        click_count: 1,
        modifiers: Modifiers::none(),
    }
}

fn dispatch(
    canvas: &Entity<CanvasState>,
    cx: &mut TestAppContext,
    hit: Option<ShapeId>,
    ev: CanvasPointerEvent,
    step: &str,
) {
    canvas.update(cx, |state, cx| {
        let ctx = tool_ctx(hit);
        state.dispatch_tool_event(cx, |tool, state, cx| match step {
            "down" => tool.pointer_down(&ev, &ctx, state, cx),
            "move" => tool.pointer_move(&ev, &ctx, state, cx),
            _ => tool.pointer_up(&ev, &ctx, state, cx),
        });
    });
}

#[derive(Default)]
struct RecordingLink {
    sends: Mutex<Vec<(&'static str, serde_json::Value)>>,
    shutdown: AtomicBool,
}

impl NetLink for RecordingLink {
    fn send(&self, event: &'static str, payload: serde_json::Value) {
        self.sends.lock().unwrap().push((event, payload));
    }

    fn shutdown(&self) {
        self.shutdown.store(true, Ordering::SeqCst);
    }
}

fn install_recorder(
    session: &Entity<CollabSession>,
    cx: &mut TestAppContext,
) -> Arc<RecordingLink> {
    let link = Arc::new(RecordingLink::default());
    session.update(cx, |s, cx| {
        s.install_link_for_test(link.clone(), cx);
        s.set_mode(Mode::Live, cx);
    });
    link
}

fn board_lamport(session: &Entity<CollabSession>, cx: &TestAppContext) -> u64 {
    cx.read(|cx| session.read(cx).board_lamport_for_test())
}

fn op_count(link: &RecordingLink) -> usize {
    link.sends
        .lock()
        .unwrap()
        .iter()
        .filter(|(event, _)| *event == "op")
        .count()
}

fn ops_of_kind(link: &RecordingLink, matching: fn(&WireOp) -> bool) -> Vec<WireOp> {
    link.sends
        .lock()
        .unwrap()
        .iter()
        .filter(|(event, _)| *event == "op")
        .map(|(_, payload)| serde_json::from_value(payload.clone()).unwrap())
        .filter(|op| matching(op))
        .collect()
}

fn seed_published_rect(
    cx: &mut TestAppContext,
    canvas: &Entity<CanvasState>,
    session: &Entity<CollabSession>,
) -> ShapeId {
    let shape = CanvasShape::rect(
        Bounds {
            origin: point(px(100.), px(100.)),
            size: size(px(60.), px(60.)),
        },
        ShapeStyle::default(),
    );
    let id = canvas.update(cx, |state, cx| {
        let layer = state
            .document()
            .top_layer()
            .unwrap_or_else(|| state.add_layer(cx));
        state.add_shape(layer, shape.clone(), cx).unwrap()
    });
    session.update(cx, |s, cx| s.publish_add(&shape, id, cx));
    id
}

#[gpui_kit::test]
fn pen_stroke_publishes_one_add(cx: &mut TestAppContext) {
    let (canvas, session) = harness(cx);
    let link = install_recorder(&session, cx);
    canvas.update(cx, |state, cx| {
        state.set_tool(Box::new(PenTool::new(session.clone(), cx)), cx)
    });

    dispatch(&canvas, cx, None, event(100., 100.), "down");
    dispatch(&canvas, cx, None, event(150., 120.), "move");
    dispatch(&canvas, cx, None, event(200., 140.), "move");
    dispatch(&canvas, cx, None, event(200., 140.), "up");

    let (path_id, has_overlay) = cx.read(|cx| {
        let state = canvas.read(cx);
        let path_id = state
            .document()
            .shapes()
            .find(|(_, _, shape)| matches!(shape, CanvasShape::Path { .. }))
            .map(|(_, id, _)| id);
        (path_id, state.tool_overlay().is_some())
    });
    let path_id = path_id.expect("path committed to the document");
    assert!(!has_overlay);
    assert!(cx.read(|cx| session.read(cx).wire_key(path_id).is_some()));
    assert_eq!(op_count(&link), 1);
    assert_eq!(
        ops_of_kind(&link, |op| matches!(op, WireOp::Add { .. })).len(),
        1
    );
}

#[gpui_kit::test]
fn select_drag_publishes_one_update_on_release(cx: &mut TestAppContext) {
    let (canvas, session) = harness(cx);
    let link = install_recorder(&session, cx);
    let rect = seed_published_rect(cx, &canvas, &session);
    let seeded_ops = op_count(&link);
    let seeded_lamport = board_lamport(&session, cx);
    canvas.update(cx, |state, cx| {
        state.set_tool(Box::new(SelectTool::new(session.clone())), cx)
    });

    dispatch(&canvas, cx, Some(rect), event(120., 120.), "down");
    dispatch(&canvas, cx, Some(rect), event(160., 120.), "move");
    let moved = cx.read(|cx| {
        canvas
            .read(cx)
            .document()
            .shape(rect)
            .unwrap()
            .doc_bounds()
            .origin
            .x
            .as_f32()
    });
    assert!(
        (moved - 140.).abs() < 0.01,
        "shape must move during the drag"
    );
    assert_eq!(board_lamport(&session, cx), seeded_lamport);
    assert_eq!(op_count(&link), seeded_ops);

    dispatch(&canvas, cx, Some(rect), event(160., 120.), "up");
    assert_eq!(op_count(&link), seeded_ops + 1);
    let expected_key: ShapeKey = cx.read(|cx| session.read(cx).wire_key(rect).unwrap());
    let updates = ops_of_kind(&link, |op| matches!(op, WireOp::Update { .. }));
    assert_eq!(updates.len(), 1);
    assert!(matches!(&updates[0], WireOp::Update { key, .. } if *key == expected_key));
}

#[gpui_kit::test]
fn select_click_only_selects(cx: &mut TestAppContext) {
    let (canvas, session) = harness(cx);
    let link = install_recorder(&session, cx);
    let rect = seed_published_rect(cx, &canvas, &session);
    let seeded_ops = op_count(&link);
    let seeded_lamport = board_lamport(&session, cx);
    canvas.update(cx, |state, cx| {
        state.set_tool(Box::new(SelectTool::new(session.clone())), cx)
    });

    dispatch(&canvas, cx, Some(rect), event(120., 120.), "down");
    dispatch(&canvas, cx, Some(rect), event(120., 120.), "up");

    cx.read(|cx| assert_eq!(canvas.read(cx).selected_shape_ids(), &[rect]));
    assert_eq!(board_lamport(&session, cx), seeded_lamport);
    assert_eq!(op_count(&link), seeded_ops);
}

#[gpui_kit::test]
fn pointer_move_reports_own_cursor(cx: &mut TestAppContext) {
    let (canvas, session) = harness(cx);
    let link = install_recorder(&session, cx);
    canvas.update(cx, |state, cx| {
        state.set_tool(Box::new(PenTool::new(session.clone(), cx)), cx)
    });

    dispatch(&canvas, cx, None, event(100., 100.), "down");
    dispatch(&canvas, cx, None, event(150., 120.), "move");
    dispatch(&canvas, cx, None, event(200., 140.), "up");

    let sends = link.sends.lock().unwrap();
    assert!(
        sends.iter().any(|(event, _)| *event == "cursor"),
        "pen moves must report the own cursor"
    );
}
