//! Wgpu bridge plus its demo page. The engine, fence, frame helpers, and
//! mesh math are shared with the model viewer and shader playground;
//! `WgpuPage` is this module's own page.

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

pub use page::WgpuPage;

// Pure helper, compiles on every target, including wasm.
pub use frame::FpsTracker;

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
