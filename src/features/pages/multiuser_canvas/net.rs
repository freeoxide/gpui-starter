use std::sync::Arc;
use std::time::Duration;

use collab_protocol::Peer;
use futures_util::FutureExt;
use rust_socketio::asynchronous::{Client, ClientBuilder, ReconnectSettings};
use rust_socketio::{Payload, TransportType};

use super::session::{InboundEvent, NetLink};

const TARGET: &str = "gpui_starter::multiuser_canvas::net";
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);

pub(crate) enum NetCommand {
    Emit {
        event: &'static str,
        payload: serde_json::Value,
    },
    Shutdown,
}

pub(crate) struct NetHandle {
    cmd_tx: flume::Sender<NetCommand>,
    done_rx: flume::Receiver<()>,
}

impl NetLink for NetHandle {
    fn send(&self, event: &'static str, payload: serde_json::Value) {
        if let Err(err) = self.cmd_tx.try_send(NetCommand::Emit { event, payload }) {
            tracing::warn!(target: TARGET, error = %err, event, "collab socket send dropped");
        }
    }

    fn shutdown(&self) {
        if self.cmd_tx.try_send(NetCommand::Shutdown).is_err() {
            return;
        }
        match self.done_rx.recv_timeout(SHUTDOWN_TIMEOUT) {
            Ok(()) | Err(flume::RecvTimeoutError::Disconnected) => {}
            Err(flume::RecvTimeoutError::Timeout) => {
                tracing::warn!(
                    target: TARGET,
                    timeout_secs = SHUTDOWN_TIMEOUT.as_secs(),
                    "collab socket thread did not exit in time; leaving it to finish on its own"
                );
            }
        }
    }
}

/// Spawn the dedicated socket thread. The receiver carries every inbound
/// socket event as an `InboundEvent`; the returned link queues sends. The
/// thread is detached: the `Shutdown` command and a dropped `cmd` sender
/// both terminate its loop.
pub(crate) fn connect_link(
    url: String,
    hello: Peer,
) -> (flume::Receiver<InboundEvent>, Arc<dyn NetLink>) {
    let (event_tx, event_rx) = flume::unbounded();
    let (cmd_tx, cmd_rx) = flume::unbounded::<NetCommand>();
    let (done_tx, done_rx) = flume::bounded::<()>(1);
    std::thread::Builder::new()
        .name("collab-socket".into())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("collab socket tokio runtime");
            runtime.block_on(socket_loop(&url, &hello, event_tx, cmd_rx));
            let _ = done_tx.send(());
        })
        .expect("spawn collab-socket thread");
    (event_rx, Arc::new(NetHandle { cmd_tx, done_rx }))
}

async fn socket_loop(
    url: &str,
    hello: &Peer,
    event_tx: flume::Sender<InboundEvent>,
    cmd_rx: flume::Receiver<NetCommand>,
) {
    let client = match connect_with_retry(url, hello, &event_tx, &cmd_rx).await {
        Some(client) => client,
        None => return,
    };
    let _ = event_tx.send(InboundEvent::Connected);

    while let Ok(cmd) = cmd_rx.recv_async().await {
        match cmd {
            NetCommand::Emit { event, payload } => {
                if let Err(err) = client.emit(event, Payload::from(payload)).await {
                    tracing::warn!(target: TARGET, error = %err, event, "collab socket emit failed");
                    let _ = event_tx.send(InboundEvent::Error {
                        message: err.to_string(),
                    });
                }
            }
            NetCommand::Shutdown => break,
        }
    }
    let _ = client.disconnect().await;
}

/// Initial-connect retry loop: rust_socketio only auto-retries established
/// connections, so a server that is down at connect time is on us.
async fn connect_with_retry(
    url: &str,
    hello: &Peer,
    event_tx: &flume::Sender<InboundEvent>,
    cmd_rx: &flume::Receiver<NetCommand>,
) -> Option<Client> {
    let mut attempt: u64 = 0;
    loop {
        match builder(url, hello, event_tx).connect().await {
            Ok(client) => return Some(client),
            Err(err) => {
                tracing::warn!(target: TARGET, error = %err, attempt, "collab connect failed");
                let _ = event_tx.send(InboundEvent::Reconnecting);
                let backoff = Duration::from_millis(500 * (attempt + 1).min(10));
                match tokio::time::timeout(backoff, cmd_rx.recv_async()).await {
                    Err(_) => {}
                    Ok(Ok(NetCommand::Shutdown)) | Ok(Err(_)) => return None,
                    Ok(Ok(NetCommand::Emit { .. })) => {}
                }
                attempt += 1;
            }
        }
    }
}

fn builder(url: &str, hello: &Peer, event_tx: &flume::Sender<InboundEvent>) -> ClientBuilder {
    let forward = |event: &'static str| {
        let event_tx = event_tx.clone();
        move |payload: Payload, _: Client| {
            let event_tx = event_tx.clone();
            async move {
                let payload = match payload {
                    Payload::Text(values) => values.first().cloned(),
                    #[expect(deprecated)]
                    Payload::String(text) => serde_json::from_str(&text).ok(),
                    Payload::Binary(_) => None,
                };
                if let Some(payload) = payload {
                    let _ = event_tx.send(InboundEvent::ServerMessage {
                        event: event.to_string(),
                        payload,
                    });
                }
            }
            .boxed()
        }
    };
    let on_reconnect = {
        let event_tx = event_tx.clone();
        move || {
            let event_tx = event_tx.clone();
            async move {
                let _ = event_tx.send(InboundEvent::Reconnecting);
                ReconnectSettings::new()
            }
            .boxed()
        }
    };
    let auth = serde_json::to_value(hello).unwrap_or_default();
    ClientBuilder::new(url)
        .namespace("/")
        .auth(auth)
        .transport_type(TransportType::Websocket)
        .reconnect(true)
        .reconnect_on_disconnect(true)
        .reconnect_delay(300, 5000)
        .on("op", forward("op"))
        .on("cursor", forward("cursor"))
        .on("peer_join", forward("peer_join"))
        .on("peer_leave", forward("peer_leave"))
        .on("snapshot", forward("snapshot"))
        .on("error", forward("error"))
        .on_reconnect(on_reconnect)
}
