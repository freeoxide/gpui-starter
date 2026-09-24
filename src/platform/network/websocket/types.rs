//! Types for the WebSocket client scaffold, always compiled.

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Reconnecting { attempt: u8 },
    Closed,
}

pub type MessageHandler = Box<dyn Fn(&str) + Send + Sync>;

/// Reconnection behaviour: `base_delay_ms * 2^attempt`, capped at
/// `max_delay_ms` when set.
#[derive(Clone, Debug)]
pub struct ReconnectPolicy {
    pub max_retries: u8,
    pub base_delay_ms: u64,
    pub max_delay_ms: Option<u64>,
}

impl Default for ReconnectPolicy {
    fn default() -> Self {
        Self {
            max_retries: 5,
            base_delay_ms: 500,
            max_delay_ms: Some(30_000),
        }
    }
}

impl ReconnectPolicy {
    /// Delay for a given 0-indexed attempt number.
    pub fn delay_for_attempt(&self, attempt: u8) -> std::time::Duration {
        let exp = 1u64.checked_shl(attempt as u32).unwrap_or(u64::MAX);
        let raw = self.base_delay_ms.saturating_mul(exp);
        let capped = self.max_delay_ms.map_or(raw, |cap| raw.min(cap));
        std::time::Duration::from_millis(capped)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WebSocketError {
    #[error("connection failed: {0}")]
    Connection(String),
    #[error("not a ws/wss URL: {0}")]
    InvalidUrl(String),
    #[cfg(feature = "websocket")]
    #[error("send failed: {0}")]
    Send(#[source] tokio_tungstenite::tungstenite::Error),
    #[cfg(feature = "websocket")]
    #[error("close failed: {0}")]
    Close(#[source] tokio_tungstenite::tungstenite::Error),
    #[error("not connected")]
    NotConnected,
    #[error("websocket feature not enabled")]
    FeatureDisabled,
}
