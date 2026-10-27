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

pub(crate) fn build_router(state: ServerState) -> (Router, SocketIo) {
    let (layer, io) = SocketIo::new_layer();
    let shared = Arc::new(Mutex::new(state));

    io.ns("/", move |s: SocketRef, Data(hello): Data<Peer>| {
        let shared = shared.clone();
        async move {
            let snapshot = {
                let mut guard = shared.lock().unwrap();
                guard.peers.insert(hello.id, hello.clone());
                guard.sockets.insert(s.id.to_string(), hello.id);
                SnapshotMsg {
                    peers: guard.peers.values().cloned().collect(),
                    shapes: guard.board.shapes().cloned().collect(),
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
                    {
                        let mut guard = shared.lock().unwrap();
                        guard.board.observe(op.version());
                        guard.board.integrate(op.key(), op.version(), change);
                    }
                    s.broadcast().emit("op", &op).await.ok();
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

    (
        Router::new()
            .route("/healthz", get(|| async { "ok" }))
            .layer(layer),
        io,
    )
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
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .expect("bind collab-server port");
    tracing::info!(port, "collab-server listening");
    let (router, _io) = build_router(ServerState::default());
    axum::serve(listener, router)
        .await
        .expect("collab-server serve");
}

#[cfg(test)]
#[path = "main.test.rs"]
mod main_test;
