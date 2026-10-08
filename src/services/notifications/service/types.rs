use std::fmt;

use gpui_kit::SharedString;

pub const CATEGORY_ACTIONS: &str = "gpui-starter.actions";
pub const CATEGORY_REPLY: &str = "gpui-starter.reply";
// Action ids shared by the macOS category registration and the Windows
// toast XML builder (src/platform/win_toast.rs).
pub const ACTION_OPEN: &str = "settings.open";
pub const ACTION_SNOOZE: &str = "settings.snooze";
pub const ACTION_REPLY: &str = "settings.reply";

// App identity: three DISTINCT ids — display name, desktop-entry/bundle id,
// X11 WM_CLASS (must equal .desktop StartupWMClass).

/// Display name shown verbatim by the daemon (notify-rust `.appname`).
pub const APP_DISPLAY_NAME: &str = "GPUI Starter";
/// Reverse-DNS desktop-entry id: the `.desktop` filename, notify-rust
/// `Hint::DesktopEntry`, and the Wayland app_id. (Also the macOS bundle id.)
pub const DESKTOP_ENTRY_ID: &str = "com.gpui-starter.app";
/// X11 WM_CLASS value (binary basename) — must equal `.desktop` StartupWMClass.
pub const WM_CLASS: &str = "gpui-starter";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotificationBackendKind {
    UserNotify,
    NotifyRust,
    #[cfg(feature = "notifications-portal")]
    Portal,
    /// Compiled unconditionally to avoid cfg ripples; on native builds the
    /// variant never occurs (the wasm-only `WebNotificationBackend` produces it).
    Web,
    UiOnly,
}

impl fmt::Display for NotificationBackendKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UserNotify => f.write_str("user-notify"),
            Self::NotifyRust => f.write_str("notify-rust"),
            #[cfg(feature = "notifications-portal")]
            Self::Portal => f.write_str("xdg-portal"),
            Self::Web => f.write_str("web-notification"),
            Self::UiOnly => f.write_str("in-app only"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NotificationPermissionState {
    Unknown,
    Unsupported,
    Unavailable(String),
    NotDetermined,
    Denied,
    Authorized,
}

impl NotificationPermissionState {
    pub fn label(&self) -> SharedString {
        let key = match self {
            Self::Unknown => "notifications_permission_unknown",
            Self::Unsupported => "notifications_permission_unsupported",
            Self::Unavailable(_) => "notifications_permission_unavailable",
            Self::NotDetermined => "notifications_permission_not_requested",
            Self::Denied => "notifications_permission_denied",
            Self::Authorized => "notifications_permission_authorized",
        };
        let label = crate::i18n::localize(key);
        match self {
            Self::Unavailable(reason) => format!("{label}: {reason}").into(),
            _ => label.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotificationImportance {
    ForegroundOnly,
    BackgroundWorthy,
}

impl fmt::Display for NotificationImportance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ForegroundOnly => f.write_str("foreground-only"),
            Self::BackgroundWorthy => f.write_str("background-worthy"),
        }
    }
}

#[derive(Clone, Debug)]
pub struct NotificationRequest {
    pub title: SharedString,
    pub body: SharedString,
    pub play_sound: bool,
    pub thread_id: Option<String>,
    pub category: Option<String>,
    pub prefer_native: bool,
    pub importance: NotificationImportance,
    /// Force the in-app feedback toast even on native success: a D-Bus `Ok`
    /// only means accepted — GNOME suppresses banners under DND/mute/fullscreen.
    pub force_in_app_feedback: bool,
}

impl NotificationRequest {
    pub fn foreground(title: impl Into<SharedString>, body: impl Into<SharedString>) -> Self {
        Self {
            title: title.into(),
            body: body.into(),
            play_sound: true,
            thread_id: None,
            category: None,
            prefer_native: true,
            importance: NotificationImportance::ForegroundOnly,
            force_in_app_feedback: false,
        }
    }

    /// A user-initiated "send a test notification": `Critical` urgency (the
    /// only level piercing GNOME's DND/mute/fullscreen gates) + forced feedback.
    pub fn test_notification(
        title: impl Into<SharedString>,
        body: impl Into<SharedString>,
    ) -> Self {
        let mut request = Self::foreground(title, body);
        request.importance = NotificationImportance::BackgroundWorthy;
        request.force_in_app_feedback = true;
        request
    }

    pub fn action_buttons(title: impl Into<SharedString>, body: impl Into<SharedString>) -> Self {
        let mut request = Self::foreground(title, body);
        request.category = Some(CATEGORY_ACTIONS.to_string());
        request.thread_id = Some("settings-actions".to_string());
        request
    }

    pub fn reply(title: impl Into<SharedString>, body: impl Into<SharedString>) -> Self {
        let mut request = Self::foreground(title, body);
        request.category = Some(CATEGORY_REPLY.to_string());
        request.thread_id = Some("settings-reply".to_string());
        request
    }

    pub fn background_worthy(
        title: impl Into<SharedString>,
        body: impl Into<SharedString>,
    ) -> Self {
        let mut request = Self::foreground(title, body);
        request.importance = NotificationImportance::BackgroundWorthy;
        request.thread_id = Some("settings-background-worthy".to_string());
        request
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NotificationCapabilities {
    pub can_request_permission: bool,
    pub can_read_permission_state: bool,
    pub can_send_immediate_native: bool,
    pub can_send_interactive: bool,
    pub requires_packaged_runtime: bool,
}

#[derive(Clone, Debug)]
pub struct NotificationSendResult {
    pub backend_used: NotificationBackendKind,
    pub degraded: bool,
    /// True iff the backend's send returned `Ok` — "accepted by the daemon",
    /// not proof the user saw a banner (FreeDesktop suppresses silently).
    pub delivered_natively: bool,
    pub error_summary: Option<SharedString>,
    pub importance: NotificationImportance,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn web_backend_kind_displays() {
        // The kind flows into inbox records and the status bar through
        // Display — lock the string.
        assert_eq!(NotificationBackendKind::Web.to_string(), "web-notification");
    }
}
