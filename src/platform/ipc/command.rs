//! Wire types for the typed IPC command/response protocol. Transport is one
//! compact JSON object per line over `crate::single_instance`'s local socket.

use serde::{Deserialize, Serialize};

const LOG: &str = "gpui_starter::ipc::command";

/// A command a second instance can trigger in the primary process.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type", content = "payload")]
pub enum ForwardedCommand {
    ShowWindow,
    HideWindow,
    /// Toggle the command palette (or equivalent quick-action UI).
    TogglePalette,
    Quit,
    ReloadConfig,
    /// Open a deep link (`gpui-starter://...`) in the primary instance.
    DeepLink(String),
}

impl ForwardedCommand {
    pub fn label(&self) -> &'static str {
        match self {
            Self::ShowWindow => "show_window",
            Self::HideWindow => "hide_window",
            Self::TogglePalette => "toggle_palette",
            Self::Quit => "quit",
            Self::ReloadConfig => "reload_config",
            Self::DeepLink(_) => "deep_link",
        }
    }
}

/// Request envelope; `id` correlates the [`ForwardedResponse`].
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForwardedRequest {
    pub id: u64,
    pub command: ForwardedCommand,
}

impl ForwardedRequest {
    pub fn new(id: u64, command: ForwardedCommand) -> Self {
        Self { id, command }
    }
}

/// Reply to a [`ForwardedRequest`], keyed by the same `id`. The error string
/// is best-effort and log-safe, not for verbatim display to end users.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ForwardedResponse {
    pub id: u64,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub error: Option<String>,
}

impl ForwardedResponse {
    pub fn ok(id: u64) -> Self {
        Self {
            id,
            ok: true,
            error: None,
        }
    }

    pub fn error(id: u64, error: impl Into<String>) -> Self {
        let message = error.into();
        tracing::warn!(target: LOG, id, error = %message, "ipc request failed");
        Self {
            id,
            ok: false,
            error: Some(message),
        }
    }
}
