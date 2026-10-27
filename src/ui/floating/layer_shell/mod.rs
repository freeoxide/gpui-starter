//! Wayland layer-shell surface option for floats. Removal: delete this
//! directory and drop `pub mod layer_shell;`, the re-exports, and the
//! `layer_shell::init(cx)` call from `src/ui/floating/mod.rs`, the
//! `layer_shell` field and builders from `src/ui/floating/spec.rs`, the
//! `surface` field from `src/ui/floating/registry.rs`, and the layer arms
//! from `src/ui/floating/spawn.rs`.

use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

mod defer;
mod passthrough;
mod spec;
pub(crate) mod state;
#[cfg(target_os = "linux")]
pub(crate) mod wayland;

pub use defer::spawn_floating_after_displays;
pub use passthrough::set_input_passthrough;
pub use spec::{Edge4, FloatSurface, LayerShellSpec, WidgetKeyboardInteractivity, WidgetLayer};
#[cfg(target_os = "linux")]
pub use wayland::is_layer_shell_unsupported;

/// Called once from `floating::init`. Registers the two capability rows
/// for the current platform and session; idempotent.
pub use state::init;

pub const CAPABILITY_LAYER_SHELL_WIDGETS: &str = "layer_shell_widgets";
pub const CAPABILITY_LAYER_SHELL_INPUT_PASSTHROUGH: &str = "layer_shell_input_passthrough";

pub(crate) const LOG: &str = "gpui_starter::ui::floating::layer_shell";

static ANNOUNCED_IGNORED: AtomicBool = AtomicBool::new(false);

pub(crate) fn note_ignored_once() {
    if !ANNOUNCED_IGNORED.swap(true, Ordering::Relaxed) {
        tracing::info!(
            target: LOG,
            "layer_shell request ignored: not a Wayland session or no screen anchor"
        );
    }
}
