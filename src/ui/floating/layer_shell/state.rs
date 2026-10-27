use gpui_kit::{App, Global, SharedString};

use super::{CAPABILITY_LAYER_SHELL_INPUT_PASSTHROUGH, CAPABILITY_LAYER_SHELL_WIDGETS};

#[derive(Default)]
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub struct LayerShellState {
    pub(crate) available: Option<bool>,
    pub(crate) fallbacks: u32,
    pub(crate) last_error: Option<SharedString>,
}

impl Global for LayerShellState {}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn record_available(cx: &mut App) {
    let state = cx.default_global::<LayerShellState>();
    state.available = Some(true);
    state.last_error = None;
    crate::capabilities::set(
        CAPABILITY_LAYER_SHELL_WIDGETS,
        crate::capabilities::CapabilityStatus::supported_enabled(),
        cx,
    );
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn record_unsupported(err: &anyhow::Error, cx: &mut App) {
    let message = err.to_string();
    let state = cx.default_global::<LayerShellState>();
    state.available = Some(false);
    state.fallbacks += 1;
    state.last_error = Some(message.clone().into());
    crate::capabilities::set(
        CAPABILITY_LAYER_SHELL_WIDGETS,
        crate::capabilities::CapabilityStatus::degraded(message),
        cx,
    );
}

/// Called once from `floating::init`. Writes the two capability rows for
/// the current platform and session; idempotent.
pub fn init(cx: &mut App) {
    #[cfg(target_family = "wasm")]
    {
        set_unsupported(
            CAPABILITY_LAYER_SHELL_WIDGETS,
            "Wayland layer-shell only",
            cx,
        );
        set_unsupported(
            CAPABILITY_LAYER_SHELL_INPUT_PASSTHROUGH,
            "Wayland layer-shell only",
            cx,
        );
    }
    #[cfg(not(target_family = "wasm"))]
    {
        #[cfg(not(target_os = "linux"))]
        {
            set_unsupported(
                CAPABILITY_LAYER_SHELL_WIDGETS,
                "Wayland layer-shell only",
                cx,
            );
            set_unsupported(
                CAPABILITY_LAYER_SHELL_INPUT_PASSTHROUGH,
                "Wayland layer-shell only",
                cx,
            );
        }
        #[cfg(target_os = "linux")]
        match cx.compositor_name() {
            "Wayland" => {
                crate::capabilities::set(
                    CAPABILITY_LAYER_SHELL_WIDGETS,
                    crate::capabilities::CapabilityStatus::supported_enabled(),
                    cx,
                );
                crate::capabilities::set(
                    CAPABILITY_LAYER_SHELL_INPUT_PASSTHROUGH,
                    crate::capabilities::CapabilityStatus::supported_enabled(),
                    cx,
                );
            }
            "X11" => {
                set_unsupported(
                    CAPABILITY_LAYER_SHELL_WIDGETS,
                    "layer shell requires a Wayland session; X11 uses the floating toolkit",
                    cx,
                );
                set_unsupported(
                    CAPABILITY_LAYER_SHELL_INPUT_PASSTHROUGH,
                    "input regions are Wayland-only; X11 click-through is plan 25's XShape shim",
                    cx,
                );
            }
            _ => {
                set_unsupported(
                    CAPABILITY_LAYER_SHELL_WIDGETS,
                    "Wayland layer-shell only",
                    cx,
                );
                set_unsupported(
                    CAPABILITY_LAYER_SHELL_INPUT_PASSTHROUGH,
                    "Wayland layer-shell only",
                    cx,
                );
            }
        }
    }
}

fn set_unsupported(name: &str, reason: &str, cx: &mut App) {
    crate::capabilities::set(
        name,
        crate::capabilities::CapabilityStatus {
            supported: false,
            enabled: false,
            degraded: false,
            reason: Some(reason.into()),
            last_error: None,
        },
        cx,
    );
}

#[cfg(test)]
#[path = "state.test.rs"]
mod state_test;
