use std::cell::RefCell;

use gpui_kit::{App, Bounds, Edges, Global, Pixels};

use crate::capabilities::Capability;

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct FloatingPlatformInfo {
    pub native_windows: bool,
    pub always_on_top: bool,
    pub non_focus_stealing: bool,
    pub menu_bar: Option<Pixels>,
    pub notch: Option<NotchGeometry>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct NotchGeometry {
    /// Screen-edge insets in logical pixels; top includes the menu bar region
    /// on notched displays.
    pub safe_area: Edges<Pixels>,
    /// Camera-housing rect in screen coordinates (top-center), if present.
    pub camera_housing: Option<Bounds<Pixels>>,
}

/// Caches platform facts so render-time callers skip per-frame native queries
/// (NSScreen on macOS); display changes drop the cache.
#[derive(Default)]
struct PlatformInfoCache(RefCell<Option<FloatingPlatformInfo>>);

impl Global for PlatformInfoCache {}

/// Per-platform floating support facts; live notch data on macOS. Cached
/// after the first call; computed live while the cache global is absent.
pub fn probe(cx: &App) -> FloatingPlatformInfo {
    let Some(cache) = cx.try_global::<PlatformInfoCache>() else {
        return compute(cx);
    };
    if let Some(info) = cache.0.borrow().clone() {
        return info;
    }
    let info = compute(cx);
    *cache.0.borrow_mut() = Some(info.clone());
    info
}

fn compute(cx: &App) -> FloatingPlatformInfo {
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
fn menu_bar_height(cx: &App) -> Option<Pixels> {
    let display = cx.primary_display()?;
    let visible = display.visible_bounds();
    let full = display.bounds();
    Some(visible.origin.y - full.origin.y)
}

/// Installs the probe cache. Not warmed here: display enumeration can lag
/// app start (zed#46378), so the first probe() takes the live read.
pub(crate) fn init(cx: &mut App) {
    cx.default_global::<PlatformInfoCache>();
    cx.observe_displays(|_, cx| {
        if let Some(cache) = cx.try_global::<PlatformInfoCache>() {
            *cache.0.borrow_mut() = None;
        }
    })
    .detach();
}

pub(crate) fn register_capabilities(cx: &mut App) {
    #[cfg(target_family = "wasm")]
    {
        crate::capabilities::set(
            Capability::FLOATING_WINDOWS,
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
            Capability::FLOATING_ALWAYS_ON_TOP,
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
            Capability::FLOATING_CLICK_THROUGH,
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
        crate::capabilities::set(Capability::NOTCH_GEOMETRY, macos_only(), cx);
    }
    #[cfg(not(target_family = "wasm"))]
    {
        crate::capabilities::set(
            Capability::FLOATING_WINDOWS,
            crate::capabilities::CapabilityStatus::supported_enabled(),
            cx,
        );
        crate::capabilities::set(
            Capability::FLOATING_ALWAYS_ON_TOP,
            crate::capabilities::CapabilityStatus::supported_enabled(),
            cx,
        );
        crate::capabilities::set(
            Capability::FLOATING_CLICK_THROUGH,
            crate::capabilities::CapabilityStatus::supported_enabled(),
            cx,
        );
        #[cfg(target_os = "macos")]
        crate::capabilities::set(
            Capability::NOTCH_GEOMETRY,
            crate::capabilities::CapabilityStatus::supported_enabled(),
            cx,
        );
        #[cfg(not(target_os = "macos"))]
        crate::capabilities::set(Capability::NOTCH_GEOMETRY, macos_only(), cx);
    }
}

#[cfg(not(target_os = "macos"))]
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
