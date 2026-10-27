use std::time::Duration;

use collab_protocol::{
    ClientId, Peer, ShapeKey, SnapshotMsg, Version, WireColor, WireOp, WireShape, WireStroke,
};
use futures_util::FutureExt;
use rust_socketio::{Payload, TransportType, asynchronous::ClientBuilder};
use tokio::sync::mpsc;

use super::{ServerState, build_router};

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

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn two_clients_snapshot_relay_and_reconnect_presence() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (router, _io) = build_router(ServerState::default());
    tokio::spawn(async move { axum::serve(listener, router).await });
    let url = format!("http://{addr}");

    let (client_a, mut relay_a) = client(&url, &peer(11, "A")).await;
    let snapshot: SnapshotMsg = serde_json::from_value(relay_a.next("snapshot").await).unwrap();
    assert!(snapshot.peers.iter().any(|p| p.id == 11));

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
    let relayed: WireOp = serde_json::from_value(relay_b.next("op").await).unwrap();
    assert_eq!(relayed, stale);

    let (_client_c, mut relay_c) = client(&url, &peer(33, "C")).await;
    let snapshot: SnapshotMsg = serde_json::from_value(relay_c.next("snapshot").await).unwrap();
    let entry = snapshot.shapes.iter().find(|e| e.key.seq == 1).unwrap();
    assert_eq!(entry.shape, rect_shape(5.));

    client_b.disconnect().await.unwrap();
    let left: collab_protocol::PeerLeaveMsg =
        serde_json::from_value(relay_a.next("peer_leave").await).unwrap();
    assert_eq!(left.id, 22);

    let _ = client_a.disconnect().await;
    let _ = _client_c.disconnect().await;
}
