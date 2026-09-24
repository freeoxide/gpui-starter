#[cfg(not(target_family = "wasm"))]
mod notify_rust;
// ashpd has no non-Unix build; the feature exists for Flatpak/Snap Linux runs.
#[cfg(all(feature = "notifications-portal", target_os = "linux"))]
mod portal;
#[cfg(any(target_os = "macos", target_os = "windows"))]
mod user_notify;
#[cfg(target_family = "wasm")]
mod web;

#[cfg(not(target_family = "wasm"))]
pub use notify_rust::NotifyRustBackend;
#[cfg(all(feature = "notifications-portal", target_os = "linux"))]
pub use portal::PortalBackend;
#[cfg(any(target_os = "macos", target_os = "windows"))]
pub use user_notify::UserNotifyBackend;
#[cfg(target_family = "wasm")]
pub use web::WebNotificationBackend;

use async_trait::async_trait;

use crate::notifications::{
    NotificationBackendKind, NotificationCapabilities, NotificationPermissionState,
    NotificationRequest,
};

// nightly clippy flags async_trait's generated #[must_use] wrappers as
// double_must_use; the allow lives here since the attribute is macro-emitted.
#[allow(clippy::double_must_use)]
#[async_trait]
pub trait NotificationBackend: Send + Sync {
    fn kind(&self) -> NotificationBackendKind;
    fn capabilities(&self) -> NotificationCapabilities;
    async fn refresh_permission_state(&self) -> NotificationPermissionState;
    async fn request_permission(&self) -> NotificationPermissionState;
    async fn send(&self, request: &NotificationRequest) -> anyhow::Result<()>;
}

// Web Notifications mapping helpers, kept here (not `service::types`) so the
// permission mapping stays unit-testable on native builds.

#[cfg_attr(not(target_family = "wasm"), allow(dead_code))]
pub(crate) fn web_permission_to_state(permission: &str) -> NotificationPermissionState {
    match permission {
        "granted" => NotificationPermissionState::Authorized,
        "denied" => NotificationPermissionState::Denied,
        "default" => NotificationPermissionState::NotDetermined,
        _ => NotificationPermissionState::Unknown,
    }
}

/// `NotificationOptions.tag` for a request: `thread_id` doubles as the web
/// tag, so same-thread notifications replace each other in the OS tray.
#[cfg_attr(not(target_family = "wasm"), allow(dead_code))]
pub(crate) fn web_tag_for_request(request: &NotificationRequest) -> Option<String> {
    request.thread_id.clone()
}

/// Wasm: no OS notification daemon exists — this no-op backend keeps the
/// secondary slot so every send through it fails into the in-app policy.
#[cfg(target_family = "wasm")]
pub struct WasmStubBackend;

#[cfg(target_family = "wasm")]
impl WasmStubBackend {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(target_family = "wasm")]
impl Default for WasmStubBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(target_family = "wasm")]
#[async_trait]
impl NotificationBackend for WasmStubBackend {
    fn kind(&self) -> NotificationBackendKind {
        NotificationBackendKind::UiOnly
    }

    fn capabilities(&self) -> NotificationCapabilities {
        NotificationCapabilities {
            can_request_permission: false,
            can_read_permission_state: false,
            can_send_immediate_native: false,
            can_send_interactive: false,
            requires_packaged_runtime: false,
        }
    }

    async fn refresh_permission_state(&self) -> NotificationPermissionState {
        NotificationPermissionState::Unsupported
    }

    async fn request_permission(&self) -> NotificationPermissionState {
        NotificationPermissionState::Unsupported
    }

    async fn send(&self, request: &NotificationRequest) -> anyhow::Result<()> {
        anyhow::bail!(
            "native notifications unavailable on wasm (title: {})",
            request.title
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_permission_strings_map_to_states() {
        assert_eq!(
            web_permission_to_state("granted"),
            NotificationPermissionState::Authorized
        );
        assert_eq!(
            web_permission_to_state("denied"),
            NotificationPermissionState::Denied
        );
        assert_eq!(
            web_permission_to_state("default"),
            NotificationPermissionState::NotDetermined
        );
    }

    #[test]
    fn web_permission_unknown_strings_map_to_unknown() {
        // The spec enum is lowercase-only; anything else (new spec value,
        // junk, wrong casing) must not masquerade as a real state.
        for value in ["", "Granted", "GRANTED", "prompt", "unsupported"] {
            assert_eq!(
                web_permission_to_state(value),
                NotificationPermissionState::Unknown,
                "\"{value}\" must map to Unknown"
            );
        }
    }

    #[test]
    fn web_tag_follows_thread_id() {
        let reply = NotificationRequest::reply("title", "body");
        assert_eq!(
            web_tag_for_request(&reply).as_deref(),
            Some("settings-reply")
        );

        // Unthreaded requests leave the tag unset so notifications stack.
        let foreground = NotificationRequest::foreground("title", "body");
        assert_eq!(web_tag_for_request(&foreground), None);
    }
}
