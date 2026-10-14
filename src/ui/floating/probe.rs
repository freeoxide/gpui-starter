use gpui_kit::{App, Bounds, Edges, Pixels};

#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FloatingPlatformInfo {
    pub native_windows: bool,
    pub always_on_top: bool,
    pub non_focus_stealing: bool,
    pub menu_bar: Option<Pixels>,
    pub notch: Option<NotchGeometry>,
}

#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct NotchGeometry {
    /// Screen-edge insets in logical pixels; top includes the menu bar region
    /// on notched displays.
    pub safe_area: Edges<Pixels>,
    /// Camera-housing rect in screen coordinates (top-center), if present.
    pub camera_housing: Option<Bounds<Pixels>>,
}

/// Per-platform floating support facts; live notch data on macOS.
pub fn probe(cx: &App) -> FloatingPlatformInfo {
    #[cfg(target_family = "wasm")]
    let _ = cx;
    #[cfg(target_family = "wasm")]
    {
        FloatingPlatformInfo {
            native_windows: false,
            always_on_top: false,
            non_focus_stealing: false,
            menu_bar: None,
            notch: None,
        }
    }
    #[cfg(not(target_family = "wasm"))]
    {
        #[cfg(target_os = "macos")]
        let notch = super::macos_probe::notch_geometry();
        #[cfg(not(target_os = "macos"))]
        let notch = None;
        FloatingPlatformInfo {
            native_windows: true,
            always_on_top: true,
            #[cfg(any(target_os = "macos", target_os = "windows"))]
            non_focus_stealing: true,
            #[cfg(not(any(target_os = "macos", target_os = "windows")))]
            non_focus_stealing: false,
            menu_bar: menu_bar_height(cx),
            notch,
        }
    }
}

#[cfg(not(target_family = "wasm"))]
pub(crate) fn menu_bar_height(cx: &App) -> Option<Pixels> {
    let display = cx.primary_display()?;
    let visible = display.visible_bounds();
    let full = display.bounds();
    Some(visible.origin.y - full.origin.y)
}

pub(crate) fn register_capabilities(cx: &mut App) {
    #[cfg(target_family = "wasm")]
    {
        crate::capabilities::set(
            super::CAPABILITY_FLOATING_WINDOWS,
            crate::capabilities::CapabilityStatus {
                supported: false,
                enabled: true,
                degraded: true,
                reason: Some(
                    "native floating windows unavailable on wasm; in-window overlay fallback"
                        .into(),
                ),
                last_error: None,
            },
            cx,
        );
        crate::capabilities::set(
            super::CAPABILITY_ALWAYS_ON_TOP,
            crate::capabilities::CapabilityStatus {
                supported: false,
                enabled: false,
                degraded: false,
                reason: Some("overlay floats have no OS-level topmost".into()),
                last_error: None,
            },
            cx,
        );
        crate::capabilities::set(
            super::CAPABILITY_FLOATING_CLICK_THROUGH,
            crate::capabilities::CapabilityStatus {
                supported: false,
                enabled: false,
                degraded: false,
                reason: Some(
                    "overlay floats render inside the main window; no native window to pass input through"
                        .into(),
                ),
                last_error: None,
            },
            cx,
        );
        crate::capabilities::set(super::CAPABILITY_NOTCH_GEOMETRY, macos_only(), cx);
    }
    #[cfg(not(target_family = "wasm"))]
    {
        crate::capabilities::set(
            super::CAPABILITY_FLOATING_WINDOWS,
            crate::capabilities::CapabilityStatus::supported_enabled(),
            cx,
        );
        crate::capabilities::set(
            super::CAPABILITY_ALWAYS_ON_TOP,
            crate::capabilities::CapabilityStatus::supported_enabled(),
            cx,
        );
        crate::capabilities::set(
            super::CAPABILITY_FLOATING_CLICK_THROUGH,
            crate::capabilities::CapabilityStatus::supported_enabled(),
            cx,
        );
        #[cfg(target_os = "macos")]
        crate::capabilities::set(
            super::CAPABILITY_NOTCH_GEOMETRY,
            crate::capabilities::CapabilityStatus::supported_enabled(),
            cx,
        );
        #[cfg(not(target_os = "macos"))]
        crate::capabilities::set(super::CAPABILITY_NOTCH_GEOMETRY, macos_only(), cx);
    }
}

fn macos_only() -> crate::capabilities::CapabilityStatus {
    crate::capabilities::CapabilityStatus {
        supported: false,
        enabled: false,
        degraded: false,
        reason: Some("macOS only".into()),
        last_error: None,
    }
}

#[cfg(test)]
#[path = "probe.test.rs"]
mod probe_test;
