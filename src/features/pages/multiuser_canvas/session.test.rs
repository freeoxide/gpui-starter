use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use collab_protocol::{
    ClientId, Peer, ShapeEntry, ShapeKey, SnapshotMsg, Version, WireColor, WireOp, WireShape,
};
use gpui_kit::prelude::*;
use gpui_kit::{Entity, TestAppContext, point, px, size};

use super::super::fake::PEER_COUNT;
use super::{CollabSession, InboundEvent, Mode, NetLink, Status};
use crate::platform::clock::Duration;
use crate::ui::canvas::{CanvasShape, CanvasState, ShapeStyle};

fn new_session(cx: &TestAppContext) -> (Entity<CanvasState>, Entity<CollabSession>) {
    cx.update(|cx| {
        let canvas = cx.new(|_| CanvasState::new());
        let session = cx.new(|cx| CollabSession::new(canvas.clone(), cx));
        (canvas, session)
    })
}

fn rect_shape(x: f32) -> CanvasShape {
    CanvasShape::rect(
        gpui_kit::Bounds {
            origin: point(px(x), px(0.)),
            size: size(px(20.), px(20.)),
        },
        ShapeStyle::default(),
    )
}

fn wire_rect(x: f32) -> WireShape {
    WireShape::Rect {
        bounds: [x, 0., 20., 20.],
        style: collab_protocol::WireStyle {
            fill: None,
            stroke: None,
        },
    }
}

fn add_op(origin: ClientId, seq: u32, lamport: u64, x: f32) -> WireOp {
    WireOp::Add {
        key: ShapeKey { origin, seq },
        version: Version { lamport, origin },
        shape: wire_rect(x),
    }
}

fn update_op(origin: ClientId, seq: u32, lamport: u64, x: f32) -> WireOp {
    WireOp::Update {
        key: ShapeKey { origin, seq },
        version: Version { lamport, origin },
        shape: wire_rect(x),
    }
}

fn delete_op(origin: ClientId, seq: u32, lamport: u64) -> WireOp {
    WireOp::Delete {
        key: ShapeKey { origin, seq },
        version: Version { lamport, origin },
    }
}

fn remote_peer(id: ClientId) -> Peer {
    Peer {
        id,
        name: format!("peer-{id}"),
        color: WireColor([10., 0.5, 0.5, 1.]),
    }
}

fn shape_count(canvas: &Entity<CanvasState>, cx: &TestAppContext) -> usize {
    cx.read(|cx| canvas.read(cx).document().shapes().count())
}

#[gpui_kit::test]
fn new_session_starts_in_demo_mode_with_seeded_peers(cx: &mut TestAppContext) {
    let (_, session) = new_session(cx);
    cx.read(|cx| {
        let session = session.read(cx);
        assert_eq!(session.status(), Status::Demo);
        assert_eq!(session.mode(), Mode::Demo);
        assert!(!session.identity().name.is_empty());
        assert_eq!(session.peers().count(), PEER_COUNT);
    });
}

#[gpui_kit::test]
fn capability_registered_at_construction(cx: &mut TestAppContext) {
    new_session(cx);
    cx.read(|cx| {
        let snapshot = crate::capabilities::snapshot(cx);
        let status = snapshot
            .get("collab_canvas")
            .expect("collab_canvas capability registered");
        assert_eq!(status.supported, !cfg!(target_family = "wasm"));
    });
}

#[gpui_kit::test]
fn remote_add_creates_shape_and_maps_ids_both_ways(cx: &mut TestAppContext) {
    let (canvas, session) = new_session(cx);
    session.update(cx, |s, cx| s.apply_op(add_op(9, 1, 1, 10.), cx));
    let key = cx.read(|cx| {
        let doc = canvas.read(cx).document();
        let (_, id, _) = doc.shapes().next().expect("shape added");
        session.read(cx).wire_key(id).expect("wire key mapped")
    });
    assert_eq!(key, ShapeKey { origin: 9, seq: 1 });
    assert_eq!(shape_count(&canvas, cx), 1);
}

#[gpui_kit::test]
fn stale_update_is_rejected_and_leaves_document_untouched(cx: &mut TestAppContext) {
    let (canvas, session) = new_session(cx);
    session.update(cx, |s, cx| s.apply_op(add_op(9, 1, 5, 10.), cx));
    let revision = cx.read(|cx| canvas.read(cx).document_revision());
    session.update(cx, |s, cx| s.apply_op(update_op(9, 1, 4, 99.), cx));
    cx.read(|cx| {
        assert_eq!(canvas.read(cx).document_revision(), revision);
        assert_eq!(canvas.read(cx).document().shapes().count(), 1);
    });
}

#[gpui_kit::test]
fn delete_tombstones_and_stale_add_is_dropped(cx: &mut TestAppContext) {
    let (canvas, session) = new_session(cx);
    session.update(cx, |s, cx| s.apply_op(add_op(9, 1, 2, 10.), cx));
    session.update(cx, |s, cx| s.apply_op(delete_op(9, 1, 3), cx));
    assert_eq!(shape_count(&canvas, cx), 0);
    session.update(cx, |s, cx| s.apply_op(add_op(9, 1, 2, 10.), cx));
    assert_eq!(shape_count(&canvas, cx), 0);
}

#[gpui_kit::test]
fn tombstoned_key_resurrects_on_higher_version_add(cx: &mut TestAppContext) {
    let (canvas, session) = new_session(cx);
    session.update(cx, |s, cx| s.apply_op(add_op(9, 1, 2, 10.), cx));
    session.update(cx, |s, cx| s.apply_op(delete_op(9, 1, 3), cx));
    session.update(cx, |s, cx| s.apply_op(add_op(9, 1, 4, 42.), cx));
    assert_eq!(shape_count(&canvas, cx), 1);
}

#[gpui_kit::test]
fn demo_publish_add_then_higher_remote_update_replaces_geometry(cx: &mut TestAppContext) {
    let (canvas, session) = new_session(cx);
    let local_id = canvas.update(cx, |state, cx| {
        let layer = state
            .document()
            .top_layer()
            .unwrap_or_else(|| state.add_layer(cx));
        state.add_shape(layer, rect_shape(10.), cx).unwrap()
    });
    session.update(cx, |s, cx| s.publish_add(&rect_shape(10.), local_id, cx));
    let (origin, seq) = cx.read(|cx| {
        let session = session.read(cx);
        let key = session.wire_key(local_id).unwrap();
        (session.identity().id, key.seq)
    });
    session.update(cx, |s, cx| s.apply_op(update_op(origin, seq, 9, 99.), cx));
    let origin_x = cx.read(|cx| {
        let CanvasShape::Rect { bounds, .. } = canvas.read(cx).document().shape(local_id).unwrap()
        else {
            panic!("expected rect");
        };
        bounds.origin.x.as_f32()
    });
    assert!((origin_x - 99.).abs() < 0.01);
}

#[gpui_kit::test]
fn apply_snapshot_replaces_the_document_wholesale(cx: &mut TestAppContext) {
    let (canvas, session) = new_session(cx);
    session.update(cx, |s, cx| s.apply_op(add_op(9, 1, 1, 10.), cx));
    let msg = SnapshotMsg {
        peers: vec![remote_peer(5)],
        shapes: vec![ShapeEntry {
            key: ShapeKey { origin: 7, seq: 7 },
            version: Version {
                lamport: 3,
                origin: 7,
            },
            shape: wire_rect(30.),
        }],
    };
    session.update(cx, |s, cx| s.apply_snapshot(msg, cx));
    cx.read(|cx| {
        assert_eq!(canvas.read(cx).document().shapes().count(), 1);
        assert_eq!(session.read(cx).peers().count(), 1);
        assert_eq!(session.read(cx).status(), Status::Connected);
    });
}

#[gpui_kit::test]
fn connected_event_requires_live_mode(cx: &mut TestAppContext) {
    let (_, session) = new_session(cx);
    session.update(cx, |s, cx| s.handle_net_event(InboundEvent::Connected, cx));
    cx.read(|cx| assert_eq!(session.read(cx).status(), Status::Demo));
    session.update(cx, |s, cx| s.set_mode(Mode::Live, cx));
    session.update(cx, |s, cx| s.handle_net_event(InboundEvent::Connected, cx));
    cx.read(|cx| assert_eq!(session.read(cx).status(), Status::Connected));
}

#[gpui_kit::test]
fn snapshot_server_message_flips_status_and_populates_peers(cx: &mut TestAppContext) {
    let (_, session) = new_session(cx);
    let payload = serde_json::to_value(SnapshotMsg {
        peers: vec![remote_peer(5), remote_peer(6)],
        shapes: vec![],
    })
    .unwrap();
    session.update(cx, |s, cx| {
        s.handle_net_event(
            InboundEvent::ServerMessage {
                event: "snapshot".into(),
                payload,
            },
            cx,
        )
    });
    cx.read(|cx| {
        let session = session.read(cx);
        assert_eq!(session.status(), Status::Connected);
        assert_eq!(session.peers().count(), 2);
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

#[gpui_kit::test]
fn cursor_sends_are_throttled(cx: &mut TestAppContext) {
    let (_, session) = new_session(cx);
    let link = std::sync::Arc::new(RecordingLink::default());
    session.update(cx, |s, cx| {
        s.install_link_for_test(link.clone(), cx);
        s.set_mode(Mode::Live, cx);
    });
    session.update(cx, |s, cx| {
        s.set_own_cursor(point(px(1.), px(2.)), cx);
        s.set_own_cursor(point(px(3.), px(4.)), cx);
    });
    let sends = link.sends.lock().unwrap();
    let cursors = sends.iter().filter(|(event, _)| *event == "cursor").count();
    assert!(
        cursors <= 1,
        "two immediate cursor updates must not both send, got {cursors}"
    );
}

#[gpui_kit::test]
fn disconnect_shuts_the_link_down(cx: &mut TestAppContext) {
    let (_, session) = new_session(cx);
    let link = std::sync::Arc::new(RecordingLink::default());
    session.update(cx, |s, cx| {
        s.install_link_for_test(link.clone(), cx);
        s.set_mode(Mode::Live, cx);
    });
    session.update(cx, |s, cx| s.disconnect(cx));
    assert!(link.shutdown.load(Ordering::SeqCst));
    cx.read(|cx| assert_eq!(session.read(cx).status(), Status::Offline));
}

const DEMO_TICK: Duration = Duration::from_millis(80);

fn run_demo_ticks(cx: &TestAppContext, ticks: u32) {
    for _ in 0..ticks {
        cx.executor().advance_clock(DEMO_TICK);
        cx.executor().run_until_parked();
    }
}

#[gpui_kit::test]
async fn demo_loop_draws_and_stops_on_live(cx: &TestAppContext) {
    let (canvas, session) = new_session(cx);

    run_demo_ticks(cx, 120);
    cx.read(|cx| {
        assert!(session.read(cx).peers().count() >= 1);
        assert!(canvas.read(cx).document().shapes().count() >= 1);
    });

    cx.update(|cx| session.update(cx, |s, cx| s.set_mode(Mode::Live, cx)));
    let frozen = shape_count(&canvas, cx);
    run_demo_ticks(cx, 13);
    assert_eq!(shape_count(&canvas, cx), frozen);
}
