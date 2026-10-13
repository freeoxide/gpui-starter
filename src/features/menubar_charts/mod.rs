//! Removable menu-bar charts. Removal: delete this directory, the
//! `pub mod menubar_charts;` line in `src/features/mod.rs`, the
//! `menubar_charts::init(cx)` boot hook in `src/app/init.rs`, the command arms
//! in `src/services/commands.rs`, the tray routing in
//! `src/platform/desktop_shell/tray/{macos,windows}.rs` and its Windows tests,
//! and the `menubar_charts_*` / `command_menubar_charts*` locale keys
//! (re-adding the launcher tooltip key the macOS tray read before this
//! feature), plus the `sysinfo` line in `Cargo.toml` (regenerate `Cargo.lock`
//! with it).

pub(crate) mod metrics;
mod panel;
mod sheet;

#[cfg(not(target_family = "wasm"))]
pub(crate) mod popover;

pub use sheet::open_sheet;

#[cfg(not(target_family = "wasm"))]
pub use popover::toggle_popover;

use gpui_kit::App;

pub const CAPABILITY_MENUBAR_CHARTS: &str = "menubar_charts";
pub const CAPABILITY_MENUBAR_CHARTS_ANCHORED: &str = "menubar_charts_anchored";

/// Register capability rows. Call once from `app::init` after
/// `capabilities::initialize`. Idempotent; no key bindings.
pub fn init(cx: &mut App) {
    #[cfg(not(target_family = "wasm"))]
    crate::capabilities::set(
        CAPABILITY_MENUBAR_CHARTS,
        crate::capabilities::CapabilityStatus::supported_enabled(),
        cx,
    );
    #[cfg(target_family = "wasm")]
    crate::capabilities::set(
        CAPABILITY_MENUBAR_CHARTS,
        crate::capabilities::CapabilityStatus {
            supported: false,
            enabled: false,
            degraded: false,
            reason: Some("no menu bar or tray on wasm".into()),
            last_error: None,
        },
        cx,
    );
    #[cfg(target_os = "macos")]
    crate::capabilities::set(
        CAPABILITY_MENUBAR_CHARTS_ANCHORED,
        crate::capabilities::CapabilityStatus::supported_enabled(),
        cx,
    );
    #[cfg(not(target_os = "macos"))]
    crate::capabilities::set(
        CAPABILITY_MENUBAR_CHARTS_ANCHORED,
        crate::capabilities::CapabilityStatus {
            supported: false,
            enabled: false,
            degraded: false,
            reason: Some("menu-bar popover is macOS only; sheet fallback".into()),
            last_error: None,
        },
        cx,
    );
}

/// Tray-icon rect in physical pixels, top-left origin, as delivered by
/// TrayIconEvent::Click (tray-icon 0.24.1; macOS flips to top-left before
/// to_physical). Decouples this module from the tray_icon crate so non-tray
/// callers (tests, future hosts) can anchor too.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IconRect {
    pub x: f64,
    pub y: f64,
    pub width: u32,
    pub height: u32,
}
