use std::collections::BTreeMap;
use std::fmt;
use std::rc::Rc;

use gpui_kit::{App, Global, SharedString};
use serde::{Deserialize, Serialize};

/// Registry key: one of the consts below, so an unregistered key cannot
/// compile; the inner string is the stable name diagnostics and tests show.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Capability(&'static str);

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

impl Capability {
    pub const ACCESSIBILITY: Self = Self("accessibility");
    pub const APP_MENU: Self = Self("app_menu");
    pub const APP_STATE: Self = Self("app_state");
    pub const AUTH_EMAIL: Self = Self("auth_email");
    pub const AUTH_SOCIAL: Self = Self("auth_social");
    pub const AUTH_SOCIAL_GITHUB: Self = Self("auth_social_github");
    pub const AUTH_SOCIAL_GOOGLE: Self = Self("auth_social_google");
    pub const AUTH_SOCIAL_OIDC: Self = Self("auth_social_oidc");
    pub const AUDIO_PLAYBACK: Self = Self("audio_playback");
    pub const BACKGROUND_TASKS: Self = Self("background_tasks");
    pub const CANVAS_SHADERS: Self = Self("canvas_shaders");
    pub const COLLAB_CANVAS: Self = Self("collab_canvas");
    pub const COMMAND_REGISTRY: Self = Self("command_registry");
    pub const CONNECTIVITY: Self = Self("connectivity");
    pub const DEEP_LINKS: Self = Self("deep_links");
    pub const DESKTOP_ACTIONS: Self = Self("desktop_actions");
    pub const DESKTOP_WIDGETS: Self = Self("desktop_widgets");
    pub const DIAGNOSTICS: Self = Self("diagnostics");
    pub const FILE_LOGGING: Self = Self("file_logging");
    pub const FIRST_RUN: Self = Self("first_run");
    pub const FLOATING_ALWAYS_ON_TOP: Self = Self("floating_always_on_top");
    pub const FLOATING_CLICK_THROUGH: Self = Self("floating_click_through");
    pub const FLOATING_UI_PALETTES: Self = Self("floating_ui_palettes");
    pub const FLOATING_WINDOWS: Self = Self("floating_windows");
    pub const GLOBAL_SHORTCUTS: Self = Self("global_shortcuts");
    pub const IMAGE_EDITOR: Self = Self("image_editor");
    pub const LAUNCHER: Self = Self("launcher");
    pub const LAYER_SHELL_INPUT_PASSTHROUGH: Self = Self("layer_shell_input_passthrough");
    pub const LAYER_SHELL_WIDGETS: Self = Self("layer_shell_widgets");
    pub const MENUBAR_CHARTS: Self = Self("menubar_charts");
    pub const MENUBAR_CHARTS_ANCHORED: Self = Self("menubar_charts_anchored");
    pub const MICROPHONE_INPUT: Self = Self("microphone_input");
    pub const MUSIC_PLAYBACK: Self = Self("music_playback");
    pub const NATIVE_NOTIFICATIONS: Self = Self("native_notifications");
    pub const NOTCH_FLOATER: Self = Self("notch_floater");
    pub const NOTCH_GEOMETRY: Self = Self("notch_geometry");
    pub const NOTIFICATION_INBOX: Self = Self("notification_inbox");
    pub const SECOND_INSTANCE_FORWARDING: Self = Self("second_instance_forwarding");
    pub const SECURE_STORAGE: Self = Self("secure_storage");
    pub const SESSION: Self = Self("session");
    pub const SINGLE_INSTANCE: Self = Self("single_instance");
    pub const STATUS_BAR: Self = Self("status_bar");
    pub const STORAGE: Self = Self("storage");
    pub const SYSTEM_CONTROLS_BATTERY: Self = Self("system_controls_battery");
    pub const SYSTEM_CONTROLS_BLUETOOTH: Self = Self("system_controls_bluetooth");
    pub const SYSTEM_CONTROLS_BRIGHTNESS: Self = Self("system_controls_brightness");
    pub const SYSTEM_CONTROLS_VOLUME: Self = Self("system_controls_volume");
    pub const SYSTEM_TRAY: Self = Self("system_tray");
    pub const TELEMETRY: Self = Self("telemetry");
    pub const TUTORIAL: Self = Self("tutorial");
    pub const UNDO_STACK: Self = Self("undo_stack");
    pub const WGPU_EMBED: Self = Self("wgpu_embed");
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilityStatus {
    pub supported: bool,
    pub enabled: bool,
    pub degraded: bool,
    pub reason: Option<SharedString>,
    pub last_error: Option<SharedString>,
}

impl CapabilityStatus {
    pub fn supported_enabled() -> Self {
        Self {
            supported: true,
            enabled: true,
            degraded: false,
            reason: None,
            last_error: None,
        }
    }

    /// Supported but a sub-component failed: stays enabled, flagged degraded,
    /// with `err` as both reason and last_error.
    pub fn degraded(err: impl ToString) -> Self {
        let msg: SharedString = err.to_string().into();
        Self {
            supported: true,
            enabled: true,
            degraded: true,
            reason: Some(msg.clone()),
            last_error: Some(msg),
        }
    }

    /// Failed to initialize entirely: supported but not enabled, flagged
    /// degraded, with `err` as both reason and last_error.
    pub fn error(err: impl ToString) -> Self {
        let msg: SharedString = err.to_string().into();
        Self {
            supported: true,
            enabled: false,
            degraded: true,
            reason: Some(msg.clone()),
            last_error: Some(msg),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct CapabilityRegistry {
    entries: Rc<BTreeMap<Capability, CapabilityStatus>>,
}

impl Global for CapabilityRegistry {}

pub fn initialize(cx: &mut App) {
    cx.set_global(CapabilityRegistry::default());
}

/// Writes one row and notifies observers; a rewrite whose status is equal
/// to the stored one returns early, so idempotent writers cannot re-render.
pub fn set(capability: Capability, status: CapabilityStatus, cx: &mut App) {
    if let Some(registry) = cx.try_global::<CapabilityRegistry>()
        && registry.entries.get(&capability) == Some(&status)
    {
        return;
    }
    tracing::debug!(
        target: "gpui_starter::capabilities",
        capability = %capability,
        supported = status.supported,
        enabled = status.enabled,
        degraded = status.degraded,
        reason = ?status.reason,
        last_error = ?status.last_error,
        "capability updated"
    );
    let registry = cx.default_global::<CapabilityRegistry>();
    Rc::make_mut(&mut registry.entries).insert(capability, status);
}

/// Reads one row, cloning just that status.
pub fn get(capability: Capability, cx: &App) -> Option<CapabilityStatus> {
    cx.try_global::<CapabilityRegistry>()?
        .entries
        .get(&capability)
        .cloned()
}

/// Shared view of every row: a refcount bump, not a per-row clone, so
/// render paths can call it every frame.
pub fn snapshot(cx: &App) -> Rc<BTreeMap<Capability, CapabilityStatus>> {
    cx.try_global::<CapabilityRegistry>()
        .map(|registry| registry.entries.clone())
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "capabilities.test.rs"]
mod capabilities_test;
