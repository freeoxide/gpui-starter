//! Removable floating-window toolkit. Removal: delete this directory and
//! revert `pub mod floating;` in `src/ui/mod.rs`, the `floating::init(cx)`
//! line in `src/app/init.rs`, the `floating_host` field/child in
//! `src/shell/root/app_root/{state,render}.rs`, the gallery section hooks in
//! `src/gallery/sections/overlays/`, the `floating_close` locale keys, and the
//! macOS `NSScreen` feature in `Cargo.toml`.

use gpui_kit::{Action, App, KeyBinding};

#[cfg(target_os = "macos")]
mod macos_probe;
mod overlay_host;
pub mod probe;
pub mod registry;
mod shell;
pub mod spawn;
pub mod spec;

pub use overlay_host::OverlayFloatHost;
pub use probe::{FloatingPlatformInfo, NotchGeometry, probe};
pub use registry::{
    FloatingHandle, FloatingWindowEntry, FloatingWindowRegistry, floating_snapshot,
};
pub use spawn::{close_all_floating, close_floating, respawn_floating, spawn_floating};
pub use spec::{FloatAnchor, FloatLevel, FloatingSpec, FloatingWindowId, Placement9};

pub const CAPABILITY_FLOATING_WINDOWS: &str = "floating_windows";
pub const CAPABILITY_ALWAYS_ON_TOP: &str = "floating_always_on_top";
pub const CAPABILITY_NOTCH_GEOMETRY: &str = "notch_geometry";

/// Esc action inside a floating shell or overlay pane; bound by `init`.
#[derive(Action, Clone, PartialEq, Eq, serde::Deserialize)]
#[action(namespace = floating, no_json)]
pub struct DismissFloating;

/// Call once from `app::init` before any spawn: capabilities, Esc binding,
/// window-close tracking.
pub fn init(cx: &mut App) {
    registry::install_close_tracker(cx);
    cx.bind_keys([KeyBinding::new("escape", DismissFloating, Some("floating"))]);
    probe::register_capabilities(cx);
}
