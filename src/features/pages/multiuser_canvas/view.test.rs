use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

use collab_protocol::{ShapeKey, WireOp};
use gpui_kit::{
    Bounds, Entity, Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px, size,
};

use super::super::session::{CollabSession, Mode, NetLink, Status};
use super::super::{DeleteSelection, MultiuserCanvasPage};
use crate::platform::clock::Duration;
use crate::ui::canvas::{CanvasShape, CanvasState, ShapeId, ShapeStyle};

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

fn open_page(cx: &mut TestAppContext) -> (Entity<MultiuserCanvasPage>, VisualTestContext) {
    cx.update(gpui_kit::component::theme::init);
    let handle = cx.open_window(size(px(1024.), px(768.)), |window, cx| {
        MultiuserCanvasPage::new(window, cx)
    });
    let page = handle.root(cx).expect("page root");
    let mut cx = VisualTestContext::from_window(handle.into(), cx);
    cx.update(|window, cx| window.draw(cx).clear(cx));
    (page, cx)
}

fn page_entities(
    cx: &TestAppContext,
    page: &Entity<MultiuserCanvasPage>,
) -> (Entity<CanvasState>, Entity<CollabSession>) {
    cx.read(|cx| {
        let page = page.read(cx);
        (page.canvas.clone(), page.session.clone())
    })
}

fn shape_ids(cx: &TestAppContext, canvas: &Entity<CanvasState>) -> Vec<ShapeId> {
    cx.read(|cx| {
        canvas
            .read(cx)
            .document()
            .shapes()
            .map(|(_, id, _)| id)
            .collect()
    })
}

#[gpui_kit::test]
fn page_constructs_and_renders_a_frame(cx: &mut TestAppContext) {
    let (page, cx) = open_page(cx);
    let (canvas, session) = page_entities(&cx, &page);
    cx.read(|cx| {
        assert_eq!(session.read(cx).status(), Status::Demo);
        assert_eq!(canvas.read(cx).active_tool_id(), "pen");
    });
}

fn seeded_published_shape(
    cx: &mut TestAppContext,
    canvas: &Entity<CanvasState>,
    session: &Entity<CollabSession>,
) -> ShapeId {
    let shape = CanvasShape::rect(
        Bounds {
            origin: point(px(500.), px(500.)),
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
    session.update(cx, |s, cx| {
        s.set_mode(Mode::Live, cx);
        s.publish_add(&shape, id, cx);
    });
    canvas.update(cx, |state, cx| state.select_shape(id, false, cx));
    id
}

#[gpui_kit::test]
fn delete_selection_removes_locally_and_publishes_the_delete(cx: &mut TestAppContext) {
    let (page, mut cx) = open_page(cx);
    let (canvas, session) = page_entities(&cx, &page);
    let shape_id = seeded_published_shape(&mut cx, &canvas, &session);
    let wire_key: ShapeKey = cx.read(|cx| {
        session
            .read(cx)
            .wire_key(shape_id)
            .expect("the seeded shape carries a wire key")
    });
    let link = Arc::new(RecordingLink::default());
    cx.update(|_window, cx| {
        session.update(cx, |s, cx| s.install_link_for_test(link.clone(), cx));
    });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let focus = cx.read(|cx| page.read(cx).focus_handle.clone());
    cx.update(|window, cx| window.focus(&focus, cx));

    cx.update(|window, cx| {
        window.dispatch_action(Box::new(DeleteSelection), cx);
    });

    let present = cx.read(|cx| canvas.read(cx).document().shape(shape_id).is_some());
    assert!(!present, "the selected shape must be deleted by the action");
    let ops: Vec<WireOp> = link
        .sends
        .lock()
        .unwrap()
        .iter()
        .filter(|(event, _)| *event == "op")
        .map(|(_, payload)| serde_json::from_value(payload.clone()).unwrap())
        .collect();
    assert_eq!(ops.len(), 1, "delete must publish exactly one op to peers");
    assert!(
        matches!(&ops[0], WireOp::Delete { key, .. } if *key == wire_key),
        "the delete op must carry the shape's wire key"
    );
    assert_eq!(
        cx.read(|cx| session.read(cx).wire_key(shape_id)),
        None,
        "the local-to-wire mapping is dropped with the shape"
    );
}

#[gpui_kit::test]
fn demo_peers_then_local_pen_stroke(cx: &mut TestAppContext) {
    let (page, mut cx) = open_page(cx);
    let (canvas, session) = page_entities(&cx, &page);

    for _ in 0..150 {
        cx.executor().advance_clock(Duration::from_millis(80));
        cx.executor().run_until_parked();
        cx.update(|window, cx| window.draw(cx).clear(cx));
    }
    cx.read(|cx| {
        assert_eq!(session.read(cx).status(), Status::Demo);
        assert_eq!(session.read(cx).peers().count(), 2);
        assert!(canvas.read(cx).document().shapes().count() >= 1);
    });

    cx.update(|_, cx| session.update(cx, |s, cx| s.set_mode(Mode::Live, cx)));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let before = shape_ids(&cx, &canvas);

    cx.simulate_mouse_down(
        point(px(300.), px(400.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    cx.simulate_mouse_move(
        point(px(360.), px(420.)),
        Some(MouseButton::Left),
        Modifiers::none(),
    );
    cx.simulate_mouse_up(
        point(px(360.), px(420.)),
        MouseButton::Left,
        Modifiers::none(),
    );

    let has_wire_key = cx.read(|cx| {
        let state = canvas.read(cx);
        let mine = state
            .document()
            .shapes()
            .find(|(_, id, shape)| {
                !before.contains(id) && matches!(shape, CanvasShape::Path { .. })
            })
            .map(|(_, id, _)| id)
            .expect("pen stroke committed to the document");
        session.read(cx).wire_key(mine).is_some()
    });
    assert!(has_wire_key, "the local stroke must carry a wire key");
}
