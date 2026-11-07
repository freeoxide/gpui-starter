//! Socket.io relay for the shared-canvas demo: presence, snapshot on join,
//! op/cursor broadcast. The board it holds is truth-ish: ops integrate with
//! the same last-write-wins rule the clients apply.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use axum::{Router, routing::get};
use collab_protocol::{
    Board, Change, ClientId, CursorMsg, Peer, PeerLeaveMsg, SnapshotMsg, WireOp,
};
use socketioxide::{
    SocketIo,
    extract::{Data, SocketRef},
};

#[derive(Default)]
struct ServerState {
    board: Board,
    peers: HashMap<ClientId, Peer>,
    sockets: HashMap<String, ClientId>,
}

pub(crate) fn build_router() -> Router {
    let (layer, io) = SocketIo::new_layer();
    let shared = Arc::new(Mutex::new(ServerState::default()));

    io.ns("/", move |s: SocketRef, Data(hello): Data<Peer>| {
        let shared = shared.clone();
        async move {
            let snapshot = {
                let mut guard = shared.lock().unwrap();
                // Reconnecting clients re-send their session Peer verbatim;
                // any other Peer on a taken id is a spoof, not an overwrite.
                if guard
                    .peers
                    .get(&hello.id)
                    .is_some_and(|existing| *existing != hello)
                {
                    tracing::warn!(id = hello.id, "rejecting duplicate peer id");
                    s.disconnect().ok();
                    return;
                }
                guard.peers.insert(hello.id, hello.clone());
                guard.sockets.insert(s.id.to_string(), hello.id);
                SnapshotMsg {
                    peers: guard.peers.values().cloned().collect(),
                    shapes: guard.board.shapes().cloned().collect(),
                    tombstones: guard.board.tombstones().collect(),
                }
            };
            s.emit("snapshot", &snapshot).ok();
            s.broadcast().emit("peer_join", &hello).await.ok();

            let op_state = shared.clone();
            s.on("op", move |s: SocketRef, Data(op): Data<WireOp>| {
                let shared = op_state.clone();
                async move {
                    let change = match &op {
                        WireOp::Add { shape, .. } | WireOp::Update { shape, .. } => {
                            Change::Put(shape.clone())
                        }
                        WireOp::Delete { .. } => Change::Remove,
                    };
                    // Rejected ops stop here: relaying one would resurrect a
                    // deleted shape on any peer whose snapshot lacked the tombstone.
                    let accepted = {
                        let mut guard = shared.lock().unwrap();
                        guard.board.observe(op.version());
                        guard.board.integrate(op.key(), op.version(), change)
                    };
                    if accepted {
                        s.broadcast().emit("op", &op).await.ok();
                    }
                }
            });

            s.on(
                "cursor",
                |s: SocketRef, Data(cursor): Data<CursorMsg>| async move {
                    s.broadcast().emit("cursor", &cursor).await.ok();
                },
            );

            let disconnect_state = shared.clone();
            s.on_disconnect(move |s: SocketRef| {
                let shared = disconnect_state.clone();
                async move {
                    let left = {
                        let mut guard = shared.lock().unwrap();
                        let Some(origin) = guard.sockets.remove(&s.id.to_string()) else {
                            return;
                        };
                        // A reconnect registers its replacement socket before the
                        // dead one disconnects; evict only with the last socket.
                        if guard.sockets.values().any(|&id| id == origin) {
                            return;
                        }
                        guard.peers.remove(&origin)
                    };
                    if let Some(peer) = left {
                        tracing::info!(peer = %peer.name, "peer left");
                        s.broadcast()
                            .emit("peer_leave", &PeerLeaveMsg { id: peer.id })
                            .await
                            .ok();
                    }
                }
            });
        }
    });

    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .layer(layer)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let port = std::env::var("COLLAB_PORT").unwrap_or_else(|_| "3001".into());
    // Loopback by default; set COLLAB_HOST=0.0.0.0 to expose the relay.
    let host = std::env::var("COLLAB_HOST").unwrap_or_else(|_| "127.0.0.1".into());
    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}"))
        .await
        .expect("bind collab-server port");
    tracing::info!(host, port, "collab-server listening");
    let router = build_router();
    axum::serve(listener, router)
        .await
        .expect("collab-server serve");
}

#[cfg(test)]
#[path = "main.test.rs"]
mod main_test;
