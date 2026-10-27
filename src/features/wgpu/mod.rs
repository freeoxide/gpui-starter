//! Removable wgpu bridge + demo page. `WgpuPage`, `WgpuEngine`, and the
//! frame helpers are the only public surface. Removal = delete this
//! directory and revert its ten seam touches: the wgpu/naga lines in
//! Cargo.toml plus the regenerated Cargo.lock (only if no other feature
//! still needs them; the common-table image line is not this feature's),
//! the `pub mod wgpu;` line in features/mod.rs, the
//! Page::Wgpu arms and all() entry in shell/sidebar.rs, the VALID_HOSTS
//! entry in shell/route.rs, the AppRoot wiring in shell/root/app_root/
//! state.rs, the page count in tests/e2e_navigation.rs, the wgpu_* and
//! nav_wgpu keys in both locale files, and the wgpu_embed capability lines
//! in app/init.rs.

#[cfg(not(target_family = "wasm"))]
mod demos;
#[cfg(not(target_family = "wasm"))]
mod engine;
mod frame;
mod math;
mod page;
mod state;

/// Capability-registry name registered at startup ("wgpu_embed").
pub const CAPABILITY_WGPU: &str = "wgpu_embed";

pub use page::WgpuPage;

// Pure helpers, compile on every target, including wasm.
pub use frame::{FpsTracker, padded_bytes_per_row, rgba_to_bgra};

// RenderImage builders are native-only by contract: only the native render
// loop produces frames to wrap.
#[cfg(not(target_family = "wasm"))]
pub use frame::{bgra_to_render_image, rgba_to_render_image};

// Native-only bridge.
#[cfg(not(target_family = "wasm"))]
pub use engine::WgpuEngine;
