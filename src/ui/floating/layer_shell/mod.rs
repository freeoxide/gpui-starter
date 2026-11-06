//! Wayland layer-shell surface option for floats. Removal: delete this
//! directory and drop `pub mod layer_shell;`, the re-exports, and the
//! `layer_shell::init(cx)` call from `src/ui/floating/mod.rs`, the
//! `layer_shell` field and builders from `src/ui/floating/spec.rs`, the
//! `surface` field from `src/ui/floating/registry.rs`, and the layer arms
//! from `src/ui/floating/spawn.rs`.

use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;

mod defer;
mod spec;
pub(crate) mod state;
#[cfg(target_os = "linux")]
pub(crate) mod wayland;

pub use defer::spawn_floating_after_displays;
pub use spec::{FloatSurface, LayerShellSpec, WidgetLayer};
pub use state::init;
#[cfg(target_os = "linux")]
pub use wayland::is_layer_shell_unsupported;

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
