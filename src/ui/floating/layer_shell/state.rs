use gpui_kit::App;

use crate::capabilities::Capability;

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn record_available(cx: &mut App) {
    crate::capabilities::set(
        Capability::LAYER_SHELL_WIDGETS,
        crate::capabilities::CapabilityStatus::supported_enabled(),
        cx,
    );
}

#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
pub(crate) fn record_unsupported(err: &anyhow::Error, cx: &mut App) {
    crate::capabilities::set(
        Capability::LAYER_SHELL_WIDGETS,
        crate::capabilities::CapabilityStatus::degraded(err.to_string()),
        cx,
    );
}

/// Called once from `floating::init`. Writes the two capability rows for
/// the current platform and session; idempotent.
pub fn init(cx: &mut App) {
    #[cfg(not(target_os = "linux"))]
    {
        set_unsupported(
            Capability::LAYER_SHELL_WIDGETS,
            "Wayland layer-shell only",
            cx,
        );
        set_unsupported(
            Capability::LAYER_SHELL_INPUT_PASSTHROUGH,
            "Wayland layer-shell only",
            cx,
        );
    }
    #[cfg(target_os = "linux")]
    match cx.compositor_name() {
        "Wayland" => {
            crate::capabilities::set(
                Capability::LAYER_SHELL_WIDGETS,
                crate::capabilities::CapabilityStatus::supported_enabled(),
                cx,
            );
            crate::capabilities::set(
                Capability::LAYER_SHELL_INPUT_PASSTHROUGH,
                crate::capabilities::CapabilityStatus::supported_enabled(),
                cx,
            );
        }
        "X11" => {
            set_unsupported(
                Capability::LAYER_SHELL_WIDGETS,
                "layer shell requires a Wayland session; X11 uses the floating toolkit",
                cx,
            );
            set_unsupported(
                Capability::LAYER_SHELL_INPUT_PASSTHROUGH,
                "input regions are Wayland-only; X11 click-through is plan 25's XShape shim",
                cx,
            );
        }
        _ => {
            set_unsupported(
                Capability::LAYER_SHELL_WIDGETS,
                "Wayland layer-shell only",
                cx,
            );
            set_unsupported(
                Capability::LAYER_SHELL_INPUT_PASSTHROUGH,
                "Wayland layer-shell only",
                cx,
            );
        }
    }
}

fn set_unsupported(capability: Capability, reason: &str, cx: &mut App) {
    crate::capabilities::set(
        capability,
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
