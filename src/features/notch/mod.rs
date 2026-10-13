//! Removable notch floater. Removal: delete this directory, the
//! `pub mod notch;` line in `src/features/mod.rs`, the `notch::init(cx)` boot
//! hook in `src/app/init.rs`, the gallery hooks in
//! `src/gallery/sections/overlays/mod.rs`, and the `notch_*` locale keys.

pub mod controller;
pub mod geometry;
pub mod view;
pub mod widgets;

#[cfg(target_os = "macos")]
pub(crate) mod macos;

pub use controller::{init, is_open, toggle};

/// Capability-registry key for the notch floater.
pub const CAPABILITY_NOTCH_FLOATER: &str = "notch_floater";

pub(crate) const LOG: &str = "gpui_starter::features::notch";
