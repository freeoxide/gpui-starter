use std::time::Duration;

use collab_protocol::{
    ClientId, Peer, ShapeKey, SnapshotMsg, Version, WireColor, WireOp, WireShape, WireStroke,
};
use futures_util::FutureExt;
use rust_socketio::{Payload, TransportType, asynchronous::ClientBuilder};
use tokio::sync::mpsc;

use super::build_router;

fn peer(id: ClientId, name: &str) -> Peer {
    Peer {
        id,
        name: name.into(),
        color: WireColor([id as f32, 0.5, 0.5, 1.]),
    }
}

fn rect_shape(x: f32) -> WireShape {
    WireShape::Rect {
        bounds: [x, 0., 10., 10.],
        style: collab_protocol::WireStyle {
            fill: Some(WireColor([0., 0., 0., 1.])),
            stroke: Some(WireStroke {
                width: 1.,
                color: WireColor([0., 0., 0., 1.]),
                dash: None,
            }),
        },
    }
}

fn peer_ids(snapshot: &SnapshotMsg) -> Vec<ClientId> {
    let mut ids: Vec<ClientId> = snapshot.peers.iter().map(|p| p.id).collect();
    ids.sort_unstable();
    ids
}

fn json_of(payload: Payload) -> Option<serde_json::Value> {
    match payload {
        Payload::Text(values) => values.first().cloned(),
        #[expect(deprecated)]
        Payload::String(text) => serde_json::from_str(&text).ok(),
        Payload::Binary(_) => None,
    }
}

struct Relay {
    rx: mpsc::UnboundedReceiver<(&'static str, serde_json::Value)>,
}

impl Relay {
    async fn next(&mut self, event: &'static str) -> serde_json::Value {
        let deadline = Duration::from_secs(5);
        let waited = tokio::time::timeout(deadline, async {
            loop {
                let (seen, value) = self.rx.recv().await.expect("relay channel open");
                if seen == event {
                    return value;
                }
            }
        })
        .await;
        waited.unwrap_or_else(|_| panic!("timed out waiting for `{event}`"))
    }

    async fn assert_no(&mut self, event: &'static str) {
        let waited = tokio::time::timeout(Duration::from_millis(300), async {
            loop {
                let (seen, _) = self.rx.recv().await.expect("relay channel open");
                if seen == event {
                    return;
                }
            }
        })
        .await;
        assert!(waited.is_err(), "`{event}` was received");
    }
}

async fn client(url: &str, hello: &Peer) -> (rust_socketio::asynchronous::Client, Relay) {
    let (tx, rx) = mpsc::unbounded_channel();
    let forward = |event: &'static str| {
        let tx = tx.clone();
        move |payload: Payload, _| {
            let tx = tx.clone();
            async move {
                if let Some(value) = json_of(payload) {
                    let _ = tx.send((event, value));
                }
            }
            .boxed()
        }
    };
    let relay = Relay { rx };
    let client = ClientBuilder::new(url.to_string())
        .namespace("/")
        .auth(serde_json::to_value(hello).unwrap())
        .transport_type(TransportType::Websocket)
        .on("snapshot", forward("snapshot"))
        .on("peer_join", forward("peer_join"))
        .on("peer_leave", forward("peer_leave"))
        .on("op", forward("op"))
        .connect()
        .await
        .expect("client connects");
    (client, relay)
}

async fn spawn_server() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let router = build_router();
    tokio::spawn(async move { axum::serve(listener, router).await });
    format!("http://{addr}")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_clients_snapshot_relay_and_peer_leave() {
    let url = spawn_server().await;

    let (client_a, mut relay_a) = client(&url, &peer(11, "A")).await;
    let snapshot: SnapshotMsg = serde_json::from_value(relay_a.next("snapshot").await).unwrap();
    assert_eq!(peer_ids(&snapshot), vec![11]);

    let (client_b, mut relay_b) = client(&url, &peer(22, "B")).await;
    let joined: Peer = serde_json::from_value(relay_a.next("peer_join").await).unwrap();
    assert_eq!(joined.id, 22);

    let add = WireOp::Add {
        key: ShapeKey { origin: 22, seq: 1 },
        version: Version {
            lamport: 10,
            origin: 22,
        },
        shape: rect_shape(5.),
    };
    client_b
        .emit("op", Payload::from(serde_json::to_value(&add).unwrap()))
        .await
        .unwrap();
    let relayed: WireOp = serde_json::from_value(relay_a.next("op").await).unwrap();
    assert_eq!(relayed, add);
    relay_b.assert_no("op").await;

    let stale = WireOp::Update {
        key: ShapeKey { origin: 22, seq: 1 },
        version: Version {
            lamport: 4,
            origin: 11,
        },
        shape: rect_shape(99.),
    };
    client_a
        .emit("op", Payload::from(serde_json::to_value(&stale).unwrap()))
        .await
        .unwrap();
    relay_b.assert_no("op").await;

    let (_client_c, mut relay_c) = client(&url, &peer(33, "C")).await;
    let snapshot: SnapshotMsg = serde_json::from_value(relay_c.next("snapshot").await).unwrap();
    assert_eq!(peer_ids(&snapshot), vec![11, 22, 33]);
    let entry = snapshot.shapes.iter().find(|e| e.key.seq == 1).unwrap();
    assert_eq!(entry.shape, rect_shape(5.));

    client_b.disconnect().await.unwrap();
    let left: collab_protocol::PeerLeaveMsg =
        serde_json::from_value(relay_a.next("peer_leave").await).unwrap();
    assert_eq!(left.id, 22);

    let _ = client_a.disconnect().await;
    let _ = _client_c.disconnect().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn deleted_shapes_reach_new_joiners_as_tombstones() {
    let url = spawn_server().await;

    let (client_a, mut relay_a) = client(&url, &peer(11, "A")).await;
    let snapshot: SnapshotMsg = serde_json::from_value(relay_a.next("snapshot").await).unwrap();
    assert!(snapshot.tombstones.is_empty());

    let (client_b, mut relay_b) = client(&url, &peer(22, "B")).await;
    // Sequence on the join event: emitting before the server finishes the
    // connect handshake drops the op on some interleavings.
    relay_a.next("peer_join").await;
    let add = WireOp::Add {
        key: ShapeKey { origin: 22, seq: 1 },
        version: Version {
            lamport: 10,
            origin: 22,
        },
        shape: rect_shape(5.),
    };
    client_b
        .emit("op", Payload::from(serde_json::to_value(&add).unwrap()))
        .await
        .unwrap();
    let relayed: WireOp = serde_json::from_value(relay_a.next("op").await).unwrap();
    assert_eq!(relayed, add);

    let delete = WireOp::Delete {
        key: ShapeKey { origin: 22, seq: 1 },
        version: Version {
            lamport: 12,
            origin: 11,
        },
    };
    client_a
        .emit("op", Payload::from(serde_json::to_value(&delete).unwrap()))
        .await
        .unwrap();
    let relayed: WireOp = serde_json::from_value(relay_b.next("op").await).unwrap();
    assert_eq!(relayed, delete);

    let stale = WireOp::Add {
        key: ShapeKey { origin: 22, seq: 1 },
        version: Version {
            lamport: 11,
            origin: 22,
        },
        shape: rect_shape(9.),
    };
    client_b
        .emit("op", Payload::from(serde_json::to_value(&stale).unwrap()))
        .await
        .unwrap();
    relay_a.assert_no("op").await;

    let (_client_c, mut relay_c) = client(&url, &peer(33, "C")).await;
    let snapshot: SnapshotMsg = serde_json::from_value(relay_c.next("snapshot").await).unwrap();
    assert!(snapshot.shapes.iter().all(|entry| entry.key.seq != 1));
    assert_eq!(snapshot.tombstones.len(), 1);
    let (key, version) = snapshot.tombstones[0];
    assert_eq!(key, ShapeKey { origin: 22, seq: 1 });
    assert_eq!(version.lamport, 12);
    assert_eq!(version.origin, 11);

    let _ = client_a.disconnect().await;
    let _ = client_b.disconnect().await;
    let _ = _client_c.disconnect().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn reconnect_with_same_peer_keeps_presence() {
    let url = spawn_server().await;

    let (client_a, _relay_a) = client(&url, &peer(11, "A")).await;
    let (client_a2, mut relay_a2) = client(&url, &peer(11, "A")).await;
    let snapshot: SnapshotMsg = serde_json::from_value(relay_a2.next("snapshot").await).unwrap();
    assert_eq!(peer_ids(&snapshot), vec![11]);

    // The old socket's late disconnect must not evict the reconnected peer.
    client_a.disconnect().await.unwrap();
    relay_a2.assert_no("peer_leave").await;

    let (_client_b, mut relay_b) = client(&url, &peer(22, "B")).await;
    let snapshot: SnapshotMsg = serde_json::from_value(relay_b.next("snapshot").await).unwrap();
    assert_eq!(peer_ids(&snapshot), vec![11, 22]);

    let _ = client_a2.disconnect().await;
    let _ = _client_b.disconnect().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn duplicate_peer_id_with_a_different_peer_is_rejected() {
    let url = spawn_server().await;

    let (client_a, mut relay_a) = client(&url, &peer(11, "A")).await;
    let snapshot: SnapshotMsg = serde_json::from_value(relay_a.next("snapshot").await).unwrap();
    assert_eq!(peer_ids(&snapshot), vec![11]);

    let (client_spoof, mut relay_spoof) = client(&url, &peer(11, "spoof")).await;
    relay_spoof.assert_no("snapshot").await;
    relay_a.assert_no("peer_join").await;

    let (_client_c, mut relay_c) = client(&url, &peer(33, "C")).await;
    let snapshot: SnapshotMsg = serde_json::from_value(relay_c.next("snapshot").await).unwrap();
    assert_eq!(peer_ids(&snapshot), vec![11, 33]);
    assert!(snapshot.peers.iter().any(|p| p.id == 11 && p.name == "A"));

    let _ = client_a.disconnect().await;
    let _ = client_spoof.disconnect().await;
    let _ = _client_c.disconnect().await;
}
