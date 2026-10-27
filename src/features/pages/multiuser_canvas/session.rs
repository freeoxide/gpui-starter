use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use collab_protocol::{
    Board, Change, ClientId, CursorMsg, Peer, PeerLeaveMsg, ShapeKey, SnapshotMsg, WireColor,
    WireOp, WireShape,
};
use gpui_kit::*;

use crate::capabilities::CapabilityStatus;
use crate::platform::clock::{Duration, Instant};
use crate::ui::canvas::{CanvasShape, CanvasState, ShapeId};

use super::fake;

const TARGET: &str = "gpui_starter::multiuser_canvas";
const CURSOR_THROTTLE: Duration = Duration::from_millis(50);
pub(crate) const PEER_PALETTE: [WireColor; 8] = [
    WireColor([0., 0.65, 0.5, 1.]),
    WireColor([45., 0.65, 0.5, 1.]),
    WireColor([90., 0.65, 0.45, 1.]),
    WireColor([150., 0.6, 0.4, 1.]),
    WireColor([200., 0.65, 0.5, 1.]),
    WireColor([250., 0.65, 0.55, 1.]),
    WireColor([290., 0.55, 0.55, 1.]),
    WireColor([330., 0.65, 0.55, 1.]),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Status {
    Demo,
    Offline,
    Connecting,
    Connected,
    // Wasm never leaves Demo/Offline; only the native net path constructs this.
    #[cfg_attr(target_family = "wasm", allow(dead_code))]
    Reconnecting,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    Demo,
    Live,
}

/// One remote (or simulated) collaborator as the view reads it.
#[derive(Clone)]
pub(crate) struct PeerCursor {
    pub peer: Peer,
    pub doc: Option<Point<Pixels>>,
}

// Wasm builds have no net thread to produce these; native net.rs is the
// sole producer.
#[cfg_attr(target_family = "wasm", allow(dead_code))]
pub(crate) enum InboundEvent {
    Connected,
    Reconnecting,
    ServerMessage {
        event: String,
        payload: serde_json::Value,
    },
    Error {
        message: String,
    },
}

/// Cfg-neutral link seam: native `net::NetHandle` implements it; wasm
/// sessions hold `None`, which makes every send a no-op.
pub(crate) trait NetLink: Send + Sync {
    fn send(&self, event: &'static str, payload: serde_json::Value);
    fn shutdown(&self);
}

pub(crate) struct CollabSession {
    canvas: Entity<CanvasState>,
    identity: Peer,
    status: Status,
    mode: Mode,
    peers: BTreeMap<ClientId, PeerCursor>,
    board: Board,
    by_wire: HashMap<ShapeKey, ShapeId>,
    by_local: HashMap<ShapeId, ShapeKey>,
    next_seq: u32,
    link: Option<Arc<dyn NetLink>>,
    last_cursor: Option<Instant>,
    demo_generation: u64,
}

impl CollabSession {
    pub(crate) fn new(canvas: Entity<CanvasState>, cx: &mut Context<Self>) -> Self {
        let id = uuid::Uuid::new_v4().as_u64_pair().0 as ClientId;
        let identity = Peer {
            id,
            name: format!(
                "{} #{:04x}",
                crate::i18n::localize("multiuser_you"),
                (id & 0xffff) as u16
            ),
            color: PEER_PALETTE[(id % PEER_PALETTE.len() as u64) as usize],
        };
        #[cfg(not(target_family = "wasm"))]
        crate::capabilities::set("collab_canvas", CapabilityStatus::supported_enabled(), cx);
        #[cfg(target_family = "wasm")]
        crate::capabilities::set(
            "collab_canvas",
            CapabilityStatus {
                supported: false,
                enabled: true,
                degraded: true,
                reason: Some("live sync needs native networking; demo peers only".into()),
                last_error: None,
            },
            cx,
        );
        let mut session = Self {
            canvas,
            identity,
            status: Status::Demo,
            mode: Mode::Demo,
            peers: BTreeMap::new(),
            board: Board::new(),
            by_wire: HashMap::new(),
            by_local: HashMap::new(),
            next_seq: 0,
            link: None,
            last_cursor: None,
            demo_generation: 0,
        };
        session.start_demo(cx);
        session
    }

    pub(crate) fn status(&self) -> Status {
        self.status
    }

    pub(crate) fn mode(&self) -> Mode {
        self.mode
    }

    pub(crate) fn identity(&self) -> &Peer {
        &self.identity
    }

    pub(crate) fn peers(&self) -> impl Iterator<Item = &PeerCursor> + '_ {
        self.peers.values()
    }

    pub(crate) fn set_mode(&mut self, mode: Mode, cx: &mut Context<Self>) {
        #[cfg(target_family = "wasm")]
        {
            if mode == Mode::Live {
                tracing::info!(
                    target: TARGET,
                    "live sync needs native networking; staying in demo mode"
                );
                return;
            }
        }
        if self.mode == mode {
            return;
        }
        self.mode = mode;
        self.peers.clear();
        match mode {
            Mode::Demo => {
                self.status = Status::Demo;
                self.start_demo(cx);
            }
            Mode::Live => {
                self.status = Status::Offline;
            }
        }
        self.announce_status(cx);
        cx.notify();
    }

    pub(crate) fn set_name(&mut self, name: String, cx: &mut Context<Self>) {
        if self.identity.name == name {
            return;
        }
        self.identity.name = name;
        cx.notify();
    }

    pub(crate) fn connect(&mut self, url: String, cx: &mut Context<Self>) {
        #[cfg(not(target_family = "wasm"))]
        {
            if self.mode != Mode::Live {
                self.set_mode(Mode::Live, cx);
            }
            if let Some(old) = self.link.take() {
                old.shutdown();
            }
            self.status = Status::Connecting;
            self.announce_status(cx);
            cx.notify();
            let (events, link) = super::net::connect_link(url, self.identity.clone());
            self.link = Some(link);
            cx.spawn(async move |this, cx| {
                while let Ok(event) = events.recv_async().await {
                    if this
                        .update(cx, |session, cx| session.handle_net_event(event, cx))
                        .is_err()
                    {
                        break;
                    }
                }
            })
            .detach();
        }
        #[cfg(target_family = "wasm")]
        {
            let _ = (url, cx);
            tracing::info!(target: TARGET, "live connect needs native networking; staying in demo mode");
        }
    }

    pub(crate) fn disconnect(&mut self, cx: &mut Context<Self>) {
        if let Some(link) = self.link.take() {
            link.shutdown();
        }
        self.peers.clear();
        self.status = if self.mode == Mode::Live {
            Status::Offline
        } else {
            Status::Demo
        };
        self.announce_status(cx);
        cx.notify();
    }

    #[cfg_attr(target_family = "wasm", allow(dead_code))]
    pub(crate) fn handle_net_event(&mut self, event: InboundEvent, cx: &mut Context<Self>) {
        match event {
            InboundEvent::Connected => {
                if self.mode == Mode::Live {
                    self.status = Status::Connected;
                    self.announce_status(cx);
                    cx.notify();
                }
            }
            InboundEvent::Reconnecting => {
                if self.mode == Mode::Live {
                    self.status = Status::Reconnecting;
                    self.announce_status(cx);
                    cx.notify();
                }
            }
            InboundEvent::ServerMessage { event, payload } => {
                self.handle_server_message(&event, payload, cx);
            }
            InboundEvent::Error { message } => {
                tracing::warn!(target: TARGET, error = %message, "collab socket error");
                if self.mode == Mode::Live {
                    crate::capabilities::set("collab_canvas", CapabilityStatus::error(message), cx);
                    cx.notify();
                }
            }
        }
    }

    #[cfg_attr(target_family = "wasm", allow(dead_code))]
    fn handle_server_message(
        &mut self,
        event: &str,
        payload: serde_json::Value,
        cx: &mut Context<Self>,
    ) {
        match event {
            "snapshot" => {
                if let Ok(msg) = serde_json::from_value::<SnapshotMsg>(payload) {
                    self.apply_snapshot(msg, cx);
                }
            }
            "peer_join" => {
                if let Ok(peer) = serde_json::from_value::<Peer>(payload) {
                    if peer.id != self.identity.id {
                        self.peers.insert(peer.id, PeerCursor { peer, doc: None });
                        cx.notify();
                    }
                }
            }
            "peer_leave" => {
                if let Ok(msg) = serde_json::from_value::<PeerLeaveMsg>(payload) {
                    if self.peers.remove(&msg.id).is_some() {
                        cx.notify();
                    }
                }
            }
            "op" => {
                if let Ok(op) = serde_json::from_value::<WireOp>(payload) {
                    self.apply_op(op, cx);
                }
            }
            "cursor" => {
                if let Ok(msg) = serde_json::from_value::<CursorMsg>(payload) {
                    if let Some(entry) = self.peers.get_mut(&msg.origin) {
                        entry.doc = Some(point(px(msg.doc[0]), px(msg.doc[1])));
                        cx.notify();
                    }
                }
            }
            other => {
                tracing::debug!(target: TARGET, event = other, "unhandled collab event");
            }
        }
    }

    pub(crate) fn apply_op(&mut self, op: WireOp, cx: &mut Context<Self>) {
        self.board.observe(op.version());
        let key = op.key();
        let accepted = match &op {
            WireOp::Add { version, shape, .. } | WireOp::Update { version, shape, .. } => self
                .board
                .integrate(key, *version, Change::Put(shape.clone())),
            WireOp::Delete { version, .. } => self.board.integrate(key, *version, Change::Remove),
        };
        if !accepted {
            return;
        }
        match op {
            WireOp::Add { shape, .. } | WireOp::Update { shape, .. } => {
                let canvas_shape = wire_shape_to_canvas(&shape);
                match self.by_wire.get(&key).copied() {
                    Some(local_id) => {
                        self.canvas.update(cx, |state, cx| {
                            state.edit_document(cx, |doc| {
                                doc.update_shape(local_id, |s| *s = canvas_shape);
                            });
                        });
                    }
                    None => {
                        let local_id = self.canvas.update(cx, |state, cx| {
                            let layer = state
                                .document()
                                .top_layer()
                                .unwrap_or_else(|| state.add_layer(cx));
                            state.add_shape(layer, canvas_shape, cx)
                        });
                        if let Some(local_id) = local_id {
                            self.by_wire.insert(key, local_id);
                            self.by_local.insert(local_id, key);
                        }
                    }
                }
            }
            WireOp::Delete { .. } => {
                if let Some(local_id) = self.by_wire.remove(&key) {
                    self.by_local.remove(&local_id);
                    self.canvas.update(cx, |state, cx| {
                        state.remove_shape(local_id, cx);
                    });
                }
            }
        }
        cx.notify();
    }

    #[cfg_attr(target_family = "wasm", allow(dead_code))]
    pub(crate) fn apply_snapshot(&mut self, msg: SnapshotMsg, cx: &mut Context<Self>) {
        self.board.replace_all(msg.shapes.clone());
        let mut by_wire = HashMap::new();
        let mut by_local = HashMap::new();
        self.canvas.update(cx, |state, cx| {
            state.edit_document(cx, |doc| {
                doc.clear();
                let layer = doc.add_layer();
                for entry in &msg.shapes {
                    let shape = wire_shape_to_canvas(&entry.shape);
                    if let Some(local_id) = doc.add_shape(layer, shape) {
                        by_wire.insert(entry.key, local_id);
                        by_local.insert(local_id, entry.key);
                    }
                }
            });
        });
        self.by_wire = by_wire;
        self.by_local = by_local;
        self.peers = msg
            .peers
            .iter()
            .filter(|peer| peer.id != self.identity.id)
            .map(|peer| {
                (
                    peer.id,
                    PeerCursor {
                        peer: peer.clone(),
                        doc: None,
                    },
                )
            })
            .collect();
        self.status = Status::Connected;
        self.announce_status(cx);
        cx.notify();
    }

    pub(crate) fn publish_add(
        &mut self,
        shape: &CanvasShape,
        local_id: ShapeId,
        cx: &mut Context<Self>,
    ) {
        let Some(wire) = canvas_shape_to_wire(shape) else {
            debug_assert!(false, "image shapes are not syncable");
            return;
        };
        let key = self.mint_key();
        let version = self.board.next_version(self.identity.id);
        self.board
            .integrate(key, version, Change::Put(wire.clone()));
        self.by_wire.insert(key, local_id);
        self.by_local.insert(local_id, key);
        self.send_op(WireOp::Add {
            key,
            version,
            shape: wire,
        });
        cx.notify();
    }

    pub(crate) fn publish_update(
        &mut self,
        local_id: ShapeId,
        shape: &CanvasShape,
        cx: &mut Context<Self>,
    ) {
        if !self.by_local.contains_key(&local_id) {
            self.publish_add(shape, local_id, cx);
            return;
        }
        let Some(wire) = canvas_shape_to_wire(shape) else {
            debug_assert!(false, "image shapes are not syncable");
            return;
        };
        let key = self.by_local[&local_id];
        let version = self.board.next_version(self.identity.id);
        self.board
            .integrate(key, version, Change::Put(wire.clone()));
        self.send_op(WireOp::Update {
            key,
            version,
            shape: wire,
        });
        cx.notify();
    }

    pub(crate) fn publish_delete(&mut self, local_id: ShapeId, cx: &mut Context<Self>) {
        if let Some(key) = self.by_local.remove(&local_id) {
            self.by_wire.remove(&key);
            let version = self.board.next_version(self.identity.id);
            self.board.integrate(key, version, Change::Remove);
            self.send_op(WireOp::Delete { key, version });
        }
        self.canvas.update(cx, |state, cx| {
            state.remove_shape(local_id, cx);
        });
        cx.notify();
    }

    #[cfg(test)]
    pub(crate) fn wire_key(&self, local_id: ShapeId) -> Option<ShapeKey> {
        self.by_local.get(&local_id).copied()
    }

    /// Throttles the own cursor to 20 Hz and pushes it over the link when
    /// one exists. Nothing rendered reads this state, so there is no notify.
    pub(crate) fn set_own_cursor(&mut self, doc: Point<Pixels>, _cx: &mut Context<Self>) {
        let now = Instant::now();
        let due = self
            .last_cursor
            .is_none_or(|last| now.duration_since(last) >= CURSOR_THROTTLE);
        if !due {
            return;
        }
        self.last_cursor = Some(now);
        if let Some(link) = &self.link {
            let msg = CursorMsg {
                origin: self.identity.id,
                doc: [doc.x.as_f32(), doc.y.as_f32()],
            };
            link.send("cursor", serde_json::to_value(msg).unwrap_or_default());
        }
    }

    pub(crate) fn delete_selected(&mut self, cx: &mut Context<Self>) {
        let selected: Vec<ShapeId> = self.canvas.read(cx).selected_shape_ids().to_vec();
        for id in selected {
            self.publish_delete(id, cx);
        }
        self.canvas
            .update(cx, |state, cx| state.clear_selection(cx));
    }

    pub(crate) fn clear_board(&mut self, cx: &mut Context<Self>) {
        let origin = self.identity.id;
        let keys: Vec<ShapeKey> = std::mem::take(&mut self.by_local).into_values().collect();
        self.by_wire.clear();
        for key in keys {
            let version = self.board.next_version(origin);
            self.board.integrate(key, version, Change::Remove);
            self.send_op(WireOp::Delete { key, version });
        }
        self.canvas.update(cx, |state, cx| state.clear(cx));
        cx.notify();
    }

    fn mint_key(&mut self) -> ShapeKey {
        self.next_seq += 1;
        ShapeKey {
            origin: self.identity.id,
            seq: self.next_seq,
        }
    }

    fn start_demo(&mut self, cx: &mut Context<Self>) {
        self.demo_generation += 1;
        let generation = self.demo_generation;
        let seed = self.identity.id;
        let mut driver = fake::FakePeerDriver::new(seed);
        self.peers = driver
            .peer_list()
            .into_iter()
            .map(|peer| (peer.id, PeerCursor { peer, doc: None }))
            .collect();
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(80))
                    .await;
                let events = driver.advance();
                let alive = this.update(cx, |session, cx| {
                    if session.mode != Mode::Demo || session.demo_generation != generation {
                        return false;
                    }
                    for event in events {
                        match event {
                            fake::PeerEvent::Cursor(msg) => {
                                if let Some(entry) = session.peers.get_mut(&msg.origin) {
                                    entry.doc = Some(point(px(msg.doc[0]), px(msg.doc[1])));
                                }
                            }
                            fake::PeerEvent::Op(op) => session.apply_op(op, cx),
                        }
                    }
                    cx.notify();
                    true
                });
                if !matches!(alive, Ok(true)) {
                    break;
                }
            }
        })
        .detach();
    }

    fn send_op(&self, op: WireOp) {
        if let Some(link) = &self.link {
            link.send("op", serde_json::to_value(op).unwrap_or_default());
        }
    }

    fn announce_status(&self, cx: &mut Context<Self>) {
        let status = match self.status {
            Status::Reconnecting => CapabilityStatus::degraded("reconnecting"),
            _ => CapabilityStatus::supported_enabled(),
        };
        crate::capabilities::set("collab_canvas", status, cx);
    }
}

pub(crate) fn wire_color_to_hsla(color: WireColor) -> Hsla {
    let [h, s, l, a] = color.0;
    Hsla {
        h: h / 360.,
        s,
        l,
        a,
    }
}

fn hsla_to_wire(color: Hsla) -> WireColor {
    WireColor([color.h * 360., color.s, color.l, color.a])
}

fn wire_stroke_to_canvas(stroke: &collab_protocol::WireStroke) -> crate::ui::canvas::StrokeStyle {
    crate::ui::canvas::StrokeStyle {
        width: px(stroke.width),
        color: wire_color_to_hsla(stroke.color),
        dash: stroke
            .dash
            .as_ref()
            .map(|dashes| dashes.iter().map(|dash| px(*dash)).collect()),
    }
}

fn wire_bounds_to_canvas(bounds: &[f32; 4]) -> Bounds<Pixels> {
    Bounds {
        origin: point(px(bounds[0]), px(bounds[1])),
        size: size(px(bounds[2]), px(bounds[3])),
    }
}

fn canvas_bounds_to_wire(bounds: &Bounds<Pixels>) -> [f32; 4] {
    [
        bounds.origin.x.as_f32(),
        bounds.origin.y.as_f32(),
        bounds.size.width.as_f32(),
        bounds.size.height.as_f32(),
    ]
}

fn wire_shape_to_canvas(shape: &WireShape) -> CanvasShape {
    use collab_protocol::WireStyle;
    let style_to_canvas = |style: &WireStyle| match (
        style.fill.map(wire_color_to_hsla),
        style.stroke.as_ref().map(wire_stroke_to_canvas),
    ) {
        (Some(fill), Some(stroke)) => crate::ui::canvas::ShapeStyle::both(fill, stroke),
        (Some(fill), None) => crate::ui::canvas::ShapeStyle::filled(fill),
        (None, Some(stroke)) => crate::ui::canvas::ShapeStyle::stroked(stroke),
        (None, None) => crate::ui::canvas::ShapeStyle::default(),
    };
    match shape {
        WireShape::Rect { bounds, style } => {
            CanvasShape::rect(wire_bounds_to_canvas(bounds), style_to_canvas(style))
        }
        WireShape::Ellipse { bounds, style } => {
            CanvasShape::ellipse(wire_bounds_to_canvas(bounds), style_to_canvas(style))
        }
        WireShape::Path { points, stroke } => CanvasShape::path(
            points.iter().map(|p| point(px(p[0]), px(p[1]))).collect(),
            wire_stroke_to_canvas(stroke),
        ),
        WireShape::Text {
            position,
            text,
            font_size,
            color,
        } => CanvasShape::text(
            point(px(position[0]), px(position[1])),
            text.clone(),
            px(*font_size),
            wire_color_to_hsla(*color),
        ),
    }
}

pub(crate) fn canvas_shape_to_wire(shape: &CanvasShape) -> Option<WireShape> {
    use collab_protocol::WireStyle;
    let stroke_from = |stroke: &crate::ui::canvas::StrokeStyle| collab_protocol::WireStroke {
        width: stroke.width.as_f32(),
        color: hsla_to_wire(stroke.color),
        dash: stroke
            .dash
            .as_ref()
            .map(|dashes| dashes.iter().map(|dash| dash.as_f32()).collect()),
    };
    Some(match shape {
        CanvasShape::Rect { bounds, style } => WireShape::Rect {
            bounds: canvas_bounds_to_wire(bounds),
            style: WireStyle {
                fill: style.fill().map(hsla_to_wire),
                stroke: style.stroke().map(stroke_from),
            },
        },
        CanvasShape::Ellipse { bounds, style } => WireShape::Ellipse {
            bounds: canvas_bounds_to_wire(bounds),
            style: WireStyle {
                fill: style.fill().map(hsla_to_wire),
                stroke: style.stroke().map(stroke_from),
            },
        },
        CanvasShape::Path { points, style } => WireShape::Path {
            points: points
                .iter()
                .map(|p| [p.x.as_f32(), p.y.as_f32()])
                .collect(),
            stroke: stroke_from(style),
        },
        CanvasShape::Text {
            position,
            text,
            font_size,
            color,
        } => WireShape::Text {
            position: [position.x.as_f32(), position.y.as_f32()],
            text: text.to_string(),
            font_size: font_size.as_f32(),
            color: hsla_to_wire(*color),
        },
        CanvasShape::Image { .. } => return None,
    })
}

#[cfg(test)]
impl CollabSession {
    pub(crate) fn install_link_for_test(&mut self, link: Arc<dyn NetLink>, cx: &mut Context<Self>) {
        self.link = Some(link);
        cx.notify();
    }

    pub(crate) fn board_lamport_for_test(&self) -> u64 {
        self.board.lamport()
    }
}

#[cfg(test)]
#[path = "session.test.rs"]
mod session_test;
