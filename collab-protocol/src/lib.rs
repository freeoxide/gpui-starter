//! Wire protocol for the shared-canvas demo: serde types exchanged over
//! socket.io between the app's multiuser-canvas page and `collab-server`,
//! plus the last-write-wins `Board` both sides integrate ops with.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// Client identity on the wire; a plain number so JSON stays a number.
pub type ClientId = u64;

/// LWW ordering key. Lexicographic `(lamport, origin)` order is the total
/// order every side agrees on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Version {
    pub lamport: u64,
    pub origin: ClientId,
}

/// Stable identity of one shape: which client minted it, and that client's
/// per-shape sequence number.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ShapeKey {
    pub origin: ClientId,
    pub seq: u32,
}

/// Serializable HSLA color: `[h degrees, s, l, a]`, the last three 0..=1.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WireColor(pub [f32; 4]);

/// Stroke in document units: width, color, optional dash lengths.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WireStroke {
    pub width: f32,
    pub color: WireColor,
    pub dash: Option<Vec<f32>>,
}

/// Fill and/or stroke for rect and ellipse shapes.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WireStyle {
    pub fill: Option<WireColor>,
    pub stroke: Option<WireStroke>,
}

/// Serializable shape payload; `bounds` is `[x, y, w, h]` in doc units.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WireShape {
    Rect {
        bounds: [f32; 4],
        style: WireStyle,
    },
    Ellipse {
        bounds: [f32; 4],
        style: WireStyle,
    },
    Path {
        points: Vec<[f32; 2]>,
        stroke: WireStroke,
    },
    Text {
        position: [f32; 2],
        text: String,
        font_size: f32,
        color: WireColor,
    },
}

/// One document operation, relayed on the `op` event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WireOp {
    Add {
        key: ShapeKey,
        version: Version,
        shape: WireShape,
    },
    /// Full replacement of the shape's geometry.
    Update {
        key: ShapeKey,
        version: Version,
        shape: WireShape,
    },
    Delete {
        key: ShapeKey,
        version: Version,
    },
}

impl WireOp {
    pub fn key(&self) -> ShapeKey {
        match self {
            WireOp::Add { key, .. } | WireOp::Update { key, .. } | WireOp::Delete { key, .. } => {
                *key
            }
        }
    }

    pub fn version(&self) -> Version {
        match self {
            WireOp::Add { version, .. }
            | WireOp::Update { version, .. }
            | WireOp::Delete { version, .. } => *version,
        }
    }
}

/// One collaborator: sent as handshake auth and on the `peer_join` event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Peer {
    pub id: ClientId,
    pub name: String,
    pub color: WireColor,
}

/// Cursor position in doc units, relayed on the `cursor` event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CursorMsg {
    pub origin: ClientId,
    pub doc: [f32; 2],
}

/// One board entry inside a `snapshot`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ShapeEntry {
    pub key: ShapeKey,
    pub version: Version,
    pub shape: WireShape,
}

/// Full board and presence state, sent to a client on join or reconnect.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SnapshotMsg {
    pub peers: Vec<Peer>,
    pub shapes: Vec<ShapeEntry>,
}

/// Sent on the `peer_leave` event.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct PeerLeaveMsg {
    pub id: ClientId,
}

/// What `Board::integrate` applies to a shape.
#[derive(Clone, Debug, PartialEq)]
pub enum Change {
    Put(WireShape),
    Remove,
}

/// Last-write-wins board shared by client and server: current shapes,
/// per-key tombstones, and the local lamport clock.
#[derive(Clone, Debug, Default)]
pub struct Board {
    shapes: HashMap<ShapeKey, ShapeEntry>,
    tombstones: HashMap<ShapeKey, Version>,
    lamport: u64,
}

impl Board {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn lamport(&self) -> u64 {
        self.lamport
    }

    /// Lamport-advancing version for a locally minted op.
    pub fn next_version(&mut self, origin: ClientId) -> Version {
        self.lamport += 1;
        Version {
            lamport: self.lamport,
            origin,
        }
    }

    /// Fold a received version into the clock: `lamport = max + 1`.
    pub fn observe(&mut self, version: Version) {
        self.lamport = self.lamport.max(version.lamport) + 1;
    }

    /// Pure LWW gate: accepts the change only when `version` outranks both
    /// any stored shape version and any tombstone, and returns the decision.
    pub fn integrate(&mut self, key: ShapeKey, version: Version, change: Change) -> bool {
        if let Some(entry) = self.shapes.get(&key)
            && version <= entry.version
        {
            return false;
        }
        if self
            .tombstones
            .get(&key)
            .is_some_and(|tombstone| version <= *tombstone)
        {
            return false;
        }
        match change {
            Change::Put(shape) => {
                self.tombstones.remove(&key);
                self.shapes.insert(
                    key,
                    ShapeEntry {
                        key,
                        version,
                        shape,
                    },
                );
            }
            Change::Remove => {
                self.shapes.remove(&key);
                self.tombstones.insert(key, version);
            }
        }
        true
    }

    pub fn shape(&self, key: &ShapeKey) -> Option<&ShapeEntry> {
        self.shapes.get(key)
    }

    pub fn shapes(&self) -> impl Iterator<Item = &ShapeEntry> + '_ {
        self.shapes.values()
    }

    /// Swap the whole board for a snapshot's contents. The lamport clock
    /// becomes `max(own, every seen lamport)`, so the next `next_version`
    /// outranks everything in the snapshot and post-snapshot local ops win.
    pub fn replace_all(&mut self, shapes: Vec<ShapeEntry>) {
        self.shapes.clear();
        self.tombstones.clear();
        for entry in shapes {
            self.lamport = self.lamport.max(entry.version.lamport);
            self.shapes.insert(entry.key, entry);
        }
    }

    /// Drop every shape and tombstone; the lamport clock keeps counting.
    pub fn clear(&mut self) {
        self.shapes.clear();
        self.tombstones.clear();
    }
}

#[cfg(test)]
#[path = "lib.test.rs"]
mod lib_test;
