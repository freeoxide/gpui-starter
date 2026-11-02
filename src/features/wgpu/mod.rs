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
#[cfg(not(target_family = "wasm"))]
mod fence;
mod frame;
mod math;
mod page;
mod state;

/// Capability-registry name registered at startup ("wgpu_embed").
pub const CAPABILITY_WGPU: &str = "wgpu_embed";

pub use page::WgpuPage;

// Pure helpers, compile on every target, including wasm.
pub use frame::{FpsTracker, padded_bytes_per_row, rgba_to_bgra};

// Shared mesh math for the model viewer page (demos reaches the module
// directly); still pure, so wasm compiles it too.
pub(crate) use math::{
    MESH_VERTEX_STRIDE, Mat4, encode_f32_rows, mat4_look_at, mat4_mul, mat4_rotate_y, perspective,
};

// RenderImage builders are native-only by contract: only the native render
// loop produces frames to wrap.
#[cfg(not(target_family = "wasm"))]
pub use frame::{bgra_to_render_image, rgba_to_render_image};

// Native-only bridge.
#[cfg(not(target_family = "wasm"))]
pub use engine::WgpuEngine;

// Fence errors escape the engine wrapped in anyhow; the type is exported
// so callers can match on the wedge variants (Deadline, Poisoned,
// InitTimeout) after a downcast.
#[cfg(not(target_family = "wasm"))]
pub use fence::FenceError;
