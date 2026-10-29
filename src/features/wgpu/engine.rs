use std::sync::Mutex;
use std::sync::mpsc;
use std::time::Duration;

use anyhow::{Context as _, Result};

use super::fence::{EngineFence, FRAME_DEADLINE, FenceError, INIT_DEADLINE, run_bounded};
use super::frame::padded_bytes_per_row;

const LOG: &str = "gpui_starter::wgpu_engine";

#[cfg(test)]
use std::cell::Cell;

#[cfg(test)]
thread_local! {
    /// Test seam for the fenced init: Park wedges the init thread below
    /// any deadline a test can afford.
    static INIT_SEAM: Cell<InitSeam> = const { Cell::new(InitSeam::Real) };
}

#[cfg(test)]
#[derive(Clone, Copy)]
enum InitSeam {
    Real,
    Park,
}

#[cfg(test)]
fn set_init_seam(seam: InitSeam) {
    INIT_SEAM.with(|slot| slot.set(seam));
}

/// Headless wgpu device shared by this feature and future consumers.
/// `Send + Sync`; every handle wraps the same underlying device.
pub struct WgpuEngine {
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter_label: String,
    readback: Mutex<Option<(u64, wgpu::Buffer)>>,
    fence: EngineFence,
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
                // None: headless readback needs no discrete GPU, and the
                // dGPU path is a known wedge vector on dual-GPU Macs.
                power_preference: wgpu::PowerPreference::None,
                force_fallback_adapter: false,
                compatible_surface: None,
                apply_limit_buckets: false,
            })
            .await
            .context("requesting wgpu adapter")?;
        let info = adapter.get_info();
        let adapter_label = format!("{} ({})", info.name, info.backend);
        tracing::info!(target: LOG, adapter = %adapter_label, "wgpu adapter acquired");
        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor::default())
            .await
            .context("requesting wgpu device")?;
        tracing::debug!(target: LOG, "wgpu device acquired");
        Ok(Self {
            device,
            queue,
            adapter_label,
            readback: Mutex::new(None),
            fence: EngineFence::new(),
        })
    }

    /// Blocking [`Self::new`] bounded by `INIT_DEADLINE`: the async init
    /// runs on a dedicated thread the caller waits on, so a hung
    /// `request_adapter` cannot outlive its caller. On timeout the init
    /// thread is detached (it cannot be cancelled) and an init-timeout
    /// error is returned. Blocks up to `INIT_DEADLINE`; never call from
    /// a UI thread.
    pub fn new_fenced() -> Result<Self> {
        Self::new_fenced_with_deadline(INIT_DEADLINE)
    }

    /// Deadline-taking core of [`Self::new_fenced`]; tests pass
    /// sub-second deadlines so the timeout path runs fast.
    pub(in crate::features::wgpu) fn new_fenced_with_deadline(deadline: Duration) -> Result<Self> {
        #[cfg(test)]
        let seam = INIT_SEAM.with(Cell::get);
        let engine = run_bounded("wgpu-init", deadline, move || {
            #[cfg(test)]
            if matches!(seam, InitSeam::Park) {
                // Stands in for request_adapter wedged inside the driver;
                // the fence trips while this parks.
                let (_tx, rx) = mpsc::channel::<()>();
                let _ = rx.recv();
            }
            // One current-thread runtime per init: gpui's executor is not
            // tokio and this thread owns nothing else to poll.
            let runtime = tokio::runtime::Builder::new_current_thread()
                .build()
                .context("building wgpu init runtime")?;
            runtime.block_on(Self::new())
        })
        .map_err(|err| match err {
            FenceError::Deadline { secs } => {
                tracing::warn!(target: LOG, secs, "wgpu init missed its deadline; thread detached");
                anyhow::Error::new(FenceError::InitTimeout { secs })
            }
            other => other.into_anyhow(),
        })?;
        Ok(engine)
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

    /// Runs one unit of blocking render/readback work on a dedicated OS
    /// thread bounded by `FRAME_DEADLINE`, outside wgpu's own calls (the
    /// 30s waits inside `read_texture_bytes` stay as defense in depth).
    /// On a missed deadline the engine is poisoned: this call returns a
    /// deadline error, every later fenced call refuses work without
    /// touching wgpu, and the wedged thread stays detached (one leaked
    /// thread per poisoned engine). Blocks the caller up to the
    /// deadline; never call from a UI thread.
    pub fn render_fenced<T, F>(&self, work: F) -> Result<T>
    where
        F: FnOnce() -> Result<T> + Send + 'static,
        T: Send + 'static,
    {
        self.render_fenced_with_deadline(FRAME_DEADLINE, work)
    }

    /// Deadline-taking core of [`Self::render_fenced`]; tests pass
    /// sub-second deadlines so the deadline path runs fast.
    pub(in crate::features::wgpu) fn render_fenced_with_deadline<T, F>(
        &self,
        deadline: Duration,
        work: F,
    ) -> Result<T>
    where
        F: FnOnce() -> Result<T> + Send + 'static,
        T: Send + 'static,
    {
        self.fence
            .run(deadline, work)
            .map_err(FenceError::into_anyhow)
    }

    /// True once a fenced render missed its deadline; a poisoned engine
    /// refuses all fenced work until the caller builds a replacement.
    pub fn poisoned(&self) -> bool {
        self.fence.poisoned()
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
        let mapped = buffer
            .slice(..)
            .get_mapped_range()
            .context("reading wgpu readback range")?;
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
