use collab_protocol::{CursorMsg, Peer, ShapeKey, Version, WireOp, WireShape, WireStroke};

use super::session::PEER_PALETTE;

pub(crate) const PEER_COUNT: usize = 2;
pub(crate) const DEMO_WIDTH: f32 = 800.;
pub(crate) const DEMO_HEIGHT: f32 = 600.;
const FAKE_ID_BASE: u64 = 0xFABE_0000_0000_0000;
const TAU: f32 = std::f32::consts::TAU;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum PeerEvent {
    Cursor(CursorMsg),
    Op(WireOp),
}

pub(crate) struct FakePeerDriver {
    peers: Vec<FakePeer>,
    rng_state: u64,
    tick: u64,
}

struct FakePeer {
    peer: Peer,
    phase: f32,
    cursor: [f32; 2],
    strokes: Vec<(ShapeKey, WireShape)>,
    next_op_tick: u64,
    lamport: u64,
    seq: u32,
}

impl FakePeerDriver {
    pub(crate) fn new(seed: u64) -> Self {
        let mut rng_state = seed.max(1);
        let mut peers = Vec::with_capacity(PEER_COUNT);
        for index in 0..PEER_COUNT {
            let id = FAKE_ID_BASE + index as u64;
            peers.push(FakePeer {
                peer: Peer {
                    id,
                    name: crate::i18n::localize_with_args(
                        "multiuser_demo_peer",
                        &[("n", &index.to_string())],
                    ),
                    color: PEER_PALETTE[(id % PEER_PALETTE.len() as u64) as usize],
                },
                phase: index as f32 * (TAU / PEER_COUNT as f32),
                cursor: [DEMO_WIDTH / 2., DEMO_HEIGHT / 2.],
                strokes: Vec::new(),
                next_op_tick: 40 + next_u64(&mut rng_state) % 51,
                lamport: 0,
                seq: 0,
            });
        }
        Self {
            peers,
            rng_state,
            tick: 0,
        }
    }

    pub(crate) fn peer_list(&self) -> Vec<Peer> {
        self.peers.iter().map(|peer| peer.peer.clone()).collect()
    }

    pub(crate) fn advance(&mut self) -> Vec<PeerEvent> {
        self.tick += 1;
        let mut events = Vec::new();
        for peer in &mut self.peers {
            let t = self.tick as f32 * 0.02 + peer.phase;
            peer.cursor = [
                (t.sin() * 0.5 + 0.5) * DEMO_WIDTH,
                ((t * 1.3).cos() * 0.5 + 0.5) * DEMO_HEIGHT,
            ];
            events.push(PeerEvent::Cursor(CursorMsg {
                origin: peer.peer.id,
                doc: peer.cursor,
            }));
            if self.tick < peer.next_op_tick {
                continue;
            }
            peer.next_op_tick = self.tick + 40 + next_u64(&mut self.rng_state) % 51;
            match next_u64(&mut self.rng_state) % 3 {
                0 => emit_add(peer, &mut self.rng_state, &mut events),
                1 => emit_update(peer, &mut self.rng_state, &mut events),
                _ => emit_delete(peer, &mut self.rng_state, &mut events),
            }
        }
        events
    }
}

fn emit_add(peer: &mut FakePeer, rng_state: &mut u64, events: &mut Vec<PeerEvent>) {
    let count = 10 + next_u64(rng_state) % 7;
    let mut points = Vec::with_capacity(count as usize);
    for step in 0..count {
        let angle = step as f32 / count as f32 * TAU;
        let radius = 12. + (next_u64(rng_state) % 18) as f32;
        points.push([
            peer.cursor[0] + angle.cos() * radius,
            peer.cursor[1] + angle.sin() * radius * 0.6,
        ]);
    }
    peer.seq += 1;
    peer.lamport += 1;
    let key = ShapeKey {
        origin: peer.peer.id,
        seq: peer.seq,
    };
    let shape = WireShape::Path {
        points,
        stroke: WireStroke {
            width: 2.5,
            color: peer.peer.color,
            dash: None,
        },
    };
    events.push(PeerEvent::Op(WireOp::Add {
        key,
        version: Version {
            lamport: peer.lamport,
            origin: peer.peer.id,
        },
        shape: shape.clone(),
    }));
    peer.strokes.push((key, shape));
}

fn emit_update(peer: &mut FakePeer, rng_state: &mut u64, events: &mut Vec<PeerEvent>) {
    if peer.strokes.is_empty() {
        emit_add(peer, rng_state, events);
        return;
    }
    let index = (next_u64(rng_state) % peer.strokes.len() as u64) as usize;
    let dx = (next_u64(rng_state) % 33) as f32 - 16.;
    let dy = (next_u64(rng_state) % 33) as f32 - 16.;
    let (key, shape) = &mut peer.strokes[index];
    translate_wire(shape, dx, dy);
    peer.lamport += 1;
    events.push(PeerEvent::Op(WireOp::Update {
        key: *key,
        version: Version {
            lamport: peer.lamport,
            origin: peer.peer.id,
        },
        shape: shape.clone(),
    }));
}

fn emit_delete(peer: &mut FakePeer, rng_state: &mut u64, events: &mut Vec<PeerEvent>) {
    if peer.strokes.len() <= 2 {
        emit_add(peer, rng_state, events);
        return;
    }
    let (key, _) = peer.strokes.remove(0);
    peer.lamport += 1;
    events.push(PeerEvent::Op(WireOp::Delete {
        key,
        version: Version {
            lamport: peer.lamport,
            origin: peer.peer.id,
        },
    }));
}

fn translate_wire(shape: &mut WireShape, dx: f32, dy: f32) {
    match shape {
        WireShape::Rect { bounds, .. } | WireShape::Ellipse { bounds, .. } => {
            bounds[0] += dx;
            bounds[1] += dy;
        }
        WireShape::Path { points, .. } => {
            for point in points {
                point[0] += dx;
                point[1] += dy;
            }
        }
        WireShape::Text { position, .. } => {
            position[0] += dx;
            position[1] += dy;
        }
    }
}

fn next_u64(state: &mut u64) -> u64 {
    let mut x = *state;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *state = x;
    x.wrapping_mul(0x2545_F491_4F6C_DD1D)
}

#[cfg(test)]
#[path = "fake.test.rs"]
mod fake_test;
