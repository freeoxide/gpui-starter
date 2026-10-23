use std::sync::Mutex;
use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Context as _, Result};

use super::frame::padded_bytes_per_row;

/// Headless wgpu device shared by this feature and future consumers.
/// `Send + Sync`; every handle wraps the same underlying device.
pub struct WgpuEngine {
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter_label: String,
    readback: Mutex<Option<(u64, wgpu::Buffer)>>,
}

impl WgpuEngine {
    /// Requests a surface-less adapter and device with default limits.
    /// Async; poll on any executor. Errors name the failing step.
    pub async fn new() -> Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            flags: wgpu::InstanceFlags::default(),
            backend_options: wgpu::BackendOptions::default(),
            memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
            display: None,
        });
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                force_fallback_adapter: false,
                compatible_surface: None,
            })
            .await
            .context("requesting wgpu adapter")?;
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .context("requesting wgpu device")?;
        let info = adapter.get_info();
        let adapter_label = format!("{} ({})", info.name, info.backend);
        Ok(Self {
            device,
            queue,
            adapter_label,
            readback: Mutex::new(None),
        })
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// Human-readable adapter and backend label for diagnostics.
    pub fn adapter_label(&self) -> &str {
        &self.adapter_label
    }

    /// Reads a COPY_SRC RGBA8 2D texture as tightly packed rows (width*4
    /// per row). BLOCKING: submits, maps, and polls; background thread
    /// only. The readback buffer is reused per size, so calls on one
    /// engine serialize behind its lock.
    pub fn read_texture_bytes(
        &self,
        texture: &wgpu::Texture,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>> {
        let padded = padded_bytes_per_row(width);
        let size = (padded * height as usize) as u64;
        // Held until the bytes are read back and the buffer is unmapped, so
        // overlapping callers serialize instead of double-mapping the
        // reused buffer; an early `?` return just drops the cache entry.
        let mut slot = self.readback.lock().expect("wgpu readback cache");
        let buffer = match slot.take() {
            Some((cached_size, buffer)) if cached_size == size => buffer,
            _ => self.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("wgpu-readback"),
                size,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("wgpu-readback-encode"),
            });
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded as u32),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        let submission = self.queue.submit(Some(encoder.finish()));

        let (tx, rx) = mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = tx.send(result);
            });
        self.device
            .poll(wgpu::PollType::Wait {
                submission_index: Some(submission),
                timeout: Some(Duration::from_secs(30)),
            })
            .context("polling wgpu device during readback")?;
        rx.recv_timeout(Duration::from_secs(30))
            .context("waiting for wgpu readback map")?
            .map_err(anyhow::Error::msg)
            .context("mapping wgpu readback buffer")?;

        let mut out = Vec::with_capacity(width as usize * height as usize * 4);
        let mapped = buffer.slice(..).get_mapped_range();
        for row in mapped.chunks_exact(padded).take(height as usize) {
            out.extend_from_slice(&row[..width as usize * 4]);
        }
        drop(mapped);
        buffer.unmap();
        *slot = Some((size, buffer));
        Ok(out)
    }
}

#[cfg(test)]
#[path = "engine.test.rs"]
mod engine_test;
