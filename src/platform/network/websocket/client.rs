//! WebSocket client implementations. With the `websocket` feature this is the
//! tokio-tungstenite client; without it, a stub returning `FeatureDisabled`.

#[cfg(feature = "websocket")]
use super::{ConnectionState, MessageHandler, ReconnectPolicy, WebSocketError};
#[cfg(not(feature = "websocket"))]
use super::{ConnectionState, ReconnectPolicy, WebSocketError};

#[cfg(feature = "websocket")]
mod live {
    use super::*;
    use futures_util::stream::SplitSink;
    use futures_util::{SinkExt, StreamExt};
    use std::sync::Arc;
    use tokio::sync::Mutex;
    use tokio_tungstenite::connect_async;
    use tokio_tungstenite::tungstenite::Message;

    type WsStream = tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >;
    type WriteHalf = SplitSink<WsStream, Message>;
    type InnerSink = Arc<Mutex<Option<WriteHalf>>>;

    // Bound on messages queued while disconnected, so a long outage cannot
    // grow memory without limit.
    const MAX_PENDING_MESSAGES: usize = 64;

    /// Only ws/wss are dialable here; the exact-match check keeps
    /// `connect_async` from being aimed at other schemes or handlers.
    #[allow(clippy::result_large_err)]
    pub(crate) fn validate_ws_url(url: &str) -> Result<(), WebSocketError> {
        let ok = match url.split_once(':') {
            Some((scheme, rest)) => matches!(scheme, "ws" | "wss") && !rest.is_empty(),
            None => false,
        };
        if ok {
            Ok(())
        } else {
            Err(WebSocketError::InvalidUrl(url.to_owned()))
        }
    }

    /// Minimal WebSocket client with automatic reconnection; see
    /// `connect_loop` for the reconnect and buffering contract.
    pub struct WebSocketClient {
        pub url: String,
        pub state: ConnectionState,
        pub reconnect: ReconnectPolicy,
        pub on_message: Option<MessageHandler>,
        /// Write half of the active WebSocket, or `None` when disconnected.
        inner: InnerSink,
        /// Outbound messages queued while the connection is down.
        pending: Arc<Mutex<Vec<String>>>,
    }

    impl WebSocketClient {
        pub fn new(url: String) -> Self {
            Self {
                url,
                state: ConnectionState::Disconnected,
                reconnect: ReconnectPolicy::default(),
                on_message: None,
                inner: Arc::new(Mutex::new(None)),
                pending: Arc::new(Mutex::new(Vec::new())),
            }
        }

        pub fn with_reconnect_policy(mut self, policy: ReconnectPolicy) -> Self {
            self.reconnect = policy;
            self
        }

        pub fn on_message(mut self, handler: MessageHandler) -> Self {
            self.on_message = Some(handler);
            self
        }

        async fn flush_pending(&self, sink: &mut WriteHalf) {
            let messages = std::mem::take(&mut *self.pending.lock().await);
            if messages.is_empty() {
                return;
            }

            tracing::debug!(
                target: "gpui_starter::websocket",
                count = messages.len(),
                "flushing pending messages"
            );

            for (idx, msg) in messages.iter().enumerate() {
                if sink
                    .send(Message::Text(msg.clone()))
                    .await
                    .inspect_err(|e| {
                        tracing::warn!(
                            target: "gpui_starter::websocket",
                            error = %e,
                            "failed to send pending message"
                        );
                    })
                    .is_err()
                {
                    // Re-queue from `idx`: `position(|m| m == msg)` would
                    // mis-resolve a later equal duplicate to a sent item.
                    let remaining: Vec<String> = messages[idx..].to_vec();
                    if !remaining.is_empty() {
                        self.pending.lock().await.splice(0..0, remaining);
                    }
                    return;
                }
            }
        }

        /// Connect with exponential backoff; reconnect-time sends are buffered
        /// (bounded) and flushed before each read loop. Errors stay unboxed.
        #[allow(clippy::result_large_err)]
        pub async fn connect_loop(&mut self) -> Result<(), WebSocketError> {
            validate_ws_url(&self.url)?;
            let mut attempt: u8 = 0;

            loop {
                self.state = if attempt == 0 {
                    ConnectionState::Connecting
                } else {
                    ConnectionState::Reconnecting { attempt }
                };

                // Grow the backoff counter only across failed sessions.
                let connected = self.run_session(attempt).await.is_ok();
                if connected {
                    attempt = 0;
                }
                attempt += 1;

                if attempt > self.reconnect.max_retries {
                    tracing::error!(
                        target: "gpui_starter::websocket",
                        max_retries = self.reconnect.max_retries,
                        "exceeded max retries, giving up"
                    );
                    self.state = ConnectionState::Closed;
                    return Err(WebSocketError::Connection(format!(
                        "failed after {} retries",
                        self.reconnect.max_retries
                    )));
                }

                self.backoff_sleep(attempt).await;
            }
        }

        /// Run one session; `Ok` means the connection was established, `Err`
        /// only that `connect_async` itself failed.
        #[allow(clippy::result_large_err)]
        async fn run_session(&mut self, attempt: u8) -> Result<(), WebSocketError> {
            match connect_async(&self.url).await {
                Ok((ws_stream, _response)) => {
                    tracing::info!(
                        target: "gpui_starter::websocket",
                        url = %self.url,
                        "connected"
                    );
                    self.state = ConnectionState::Connected;

                    let (write, mut read) = ws_stream.split();
                    {
                        let mut guard = self.inner.lock().await;
                        *guard = Some(write);
                        if let Some(ref mut sink) = *guard {
                            self.flush_pending(sink).await;
                        }
                    }

                    while let Some(msg) = StreamExt::next(&mut read).await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                if let Some(ref handler) = self.on_message {
                                    handler(&text);
                                }
                            }
                            Ok(Message::Close(frame)) => {
                                tracing::info!(
                                    target: "gpui_starter::websocket",
                                    frame = ?frame,
                                    "server closed connection"
                                );
                                break;
                            }
                            Ok(_) => {} // binary, ping, pong — ignored
                            Err(e) => {
                                tracing::warn!(
                                    target: "gpui_starter::websocket",
                                    error = %e,
                                    "read error"
                                );
                                break;
                            }
                        }
                    }

                    self.state = ConnectionState::Disconnected;
                    *self.inner.lock().await = None;
                    Ok(())
                }
                Err(e) => {
                    tracing::warn!(
                        target: "gpui_starter::websocket",
                        attempt,
                        error = %e,
                        "connection failed"
                    );
                    Err(WebSocketError::Connection(e.to_string()))
                }
            }
        }

        /// Sleep for the backoff delay of the given 1-based attempt counter.
        async fn backoff_sleep(&mut self, attempt: u8) {
            let delay = self.reconnect.delay_for_attempt(attempt - 1);
            tracing::info!(
                target: "gpui_starter::websocket",
                attempt,
                delay_ms = delay.as_millis() as u64,
                "waiting before reconnect"
            );
            tokio::time::sleep(delay).await;
        }

        /// Send a text message, buffering it while disconnected (bounded by
        /// `MAX_PENDING_MESSAGES`); `NotConnected` means the buffer was full.
        #[allow(clippy::result_large_err)]
        pub async fn send(&self, message: &str) -> Result<(), WebSocketError> {
            let mut guard = self.inner.lock().await;
            match guard.as_mut() {
                Some(sink) => sink
                    .send(Message::Text(message.into()))
                    .await
                    .map_err(WebSocketError::Send),
                None => {
                    drop(guard);
                    let mut buf = self.pending.lock().await;
                    if buf.len() >= MAX_PENDING_MESSAGES {
                        tracing::warn!(
                            target: "gpui_starter::websocket",
                            limit = MAX_PENDING_MESSAGES,
                            "pending buffer full, dropping message"
                        );
                        Err(WebSocketError::NotConnected)
                    } else {
                        buf.push(message.to_owned());
                        Ok(())
                    }
                }
            }
        }

        /// Close the connection and drop any never-delivered buffered messages.
        #[allow(clippy::result_large_err)]
        pub async fn close(&mut self) -> Result<(), WebSocketError> {
            let sink = self.inner.lock().await.take();
            if let Some(mut sink) = sink {
                sink.close().await.map_err(WebSocketError::Close)?;
            }
            self.pending.lock().await.clear();
            self.state = ConnectionState::Closed;
            Ok(())
        }
    }
}

#[cfg(feature = "websocket")]
pub use live::WebSocketClient;

#[cfg(all(test, feature = "websocket"))]
pub(super) use live::validate_ws_url;

#[cfg(not(feature = "websocket"))]
mod stub {
    use super::*;

    /// Feature-off placeholder; every operation reports `FeatureDisabled`.
    pub struct WebSocketClient {
        pub url: String,
        pub state: ConnectionState,
        pub reconnect: ReconnectPolicy,
    }

    impl WebSocketClient {
        pub fn new(url: String) -> Self {
            Self {
                url,
                state: ConnectionState::Disconnected,
                reconnect: ReconnectPolicy::default(),
            }
        }

        pub fn with_reconnect_policy(self, _policy: ReconnectPolicy) -> Self {
            self
        }

        pub async fn connect_loop(&mut self) -> Result<(), WebSocketError> {
            Err(WebSocketError::FeatureDisabled)
        }

        pub async fn send(&self, _message: &str) -> Result<(), WebSocketError> {
            Err(WebSocketError::FeatureDisabled)
        }

        pub async fn close(&mut self) -> Result<(), WebSocketError> {
            Err(WebSocketError::FeatureDisabled)
        }
    }
}

#[cfg(not(feature = "websocket"))]
pub use stub::WebSocketClient;
