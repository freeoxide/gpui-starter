use std::collections::VecDeque;

#[cfg(not(target_family = "wasm"))]
use std::sync::Arc;

#[cfg(not(target_family = "wasm"))]
use gpui_kit::RenderImage;

use crate::platform::clock::{Duration, Instant};

const BYTES_PER_ROW_ALIGNMENT: usize = 256;
const FPS_WINDOW: Duration = Duration::from_millis(750);

/// Bytes per row a wgpu texture copy needs for an RGBA8 target: width*4
/// rounded up to a 256-byte boundary. wgpu::COPY_BYTES_PER_ROW_ALIGNMENT
/// cannot be imported here without breaking the wasm build.
pub fn padded_bytes_per_row(width: u32) -> usize {
    (width as usize * 4 + BYTES_PER_ROW_ALIGNMENT - 1) & !(BYTES_PER_ROW_ALIGNMENT - 1)
}

/// Swizzles `width`x`height` RGBA rows laid out at `padded_bytes_per_row`
/// stride into tightly packed BGRA bytes, dropping the row padding.
pub fn rgba_to_bgra(padded_rows: &[u8], width: u32, height: u32) -> Vec<u8> {
    let padded = padded_bytes_per_row(width);
    let mut out = Vec::with_capacity(width as usize * height as usize * 4);
    for row in padded_rows.chunks_exact(padded).take(height as usize) {
        for chunk in row.as_chunks::<4>().0.iter().take(width as usize) {
            out.extend_from_slice(&[chunk[2], chunk[1], chunk[0], chunk[3]]);
        }
    }
    out
}

/// Builds a displayable image from tightly packed BGRA bytes; panics unless
/// `bgra` holds exactly width*height*4 bytes.
#[cfg(not(target_family = "wasm"))]
pub fn bgra_to_render_image(width: u32, height: u32, bgra: Vec<u8>) -> Arc<RenderImage> {
    debug_assert_eq!(bgra.len(), width as usize * height as usize * 4);
    let buffer = image::RgbaImage::from_raw(width, height, bgra)
        .expect("renderer returned width*height*4 bytes");
    Arc::new(RenderImage::new([image::Frame::new(buffer)]))
}

/// [`bgra_to_render_image`] after swizzling tightly packed RGBA input
/// (unlike [`rgba_to_bgra`], the input carries no row padding).
#[cfg(not(target_family = "wasm"))]
pub fn rgba_to_render_image(width: u32, height: u32, rgba: Vec<u8>) -> Arc<RenderImage> {
    let mut bgra = Vec::with_capacity(rgba.len());
    for chunk in rgba.as_chunks::<4>().0 {
        bgra.extend_from_slice(&[chunk[2], chunk[1], chunk[0], chunk[3]]);
    }
    bgra_to_render_image(width, height, bgra)
}

/// Frames delivered over the last 750 ms, reported scaled to one second.
pub struct FpsTracker {
    stamps: VecDeque<Instant>,
}

impl FpsTracker {
    pub fn new() -> Self {
        Self {
            stamps: VecDeque::new(),
        }
    }

    pub fn push(&mut self, now: Instant) {
        while let Some(&oldest) = self.stamps.front() {
            if now.duration_since(oldest) <= FPS_WINDOW {
                break;
            }
            self.stamps.pop_front();
        }
        self.stamps.push_back(now);
    }

    pub fn fps(&self) -> u32 {
        (self.stamps.len() as f32 / 0.75).round() as u32
    }
}

#[cfg(test)]
#[path = "frame.test.rs"]
mod frame_test;
