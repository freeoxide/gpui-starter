use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
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

/// Device-wide error state installed once per engine. Without these
/// callbacks wgpu's default turns uncaptured errors into panics and device
/// loss surfaces only as an unexplained hang on the next driver call.
#[derive(Default)]
struct DeviceErrors {
    lost: AtomicBool,
    last: Mutex<Option<String>>,
}

impl DeviceErrors {
    fn install(device: &wgpu::Device) -> Arc<Self> {
        let errors = Arc::new(Self::default());
        let lost_state = errors.clone();
        device.set_device_lost_callback(move |reason, message| {
            tracing::error!(target: LOG, reason = ?reason, "wgpu device lost: {message}");
            if !matches!(reason, wgpu::DeviceLostReason::Destroyed) {
                lost_state.lost.store(true, Ordering::Release);
            }
        });
        let uncaptured = errors.clone();
        device.on_uncaptured_error(Arc::new(move |error| {
            tracing::error!(target: LOG, "wgpu uncaptured error: {error}");
            *uncaptured.last.lock().expect("wgpu device error state") = Some(error.to_string());
        }));
        errors
    }

    fn device_lost(&self) -> bool {
        self.lost.load(Ordering::Acquire)
    }

    fn last_error(&self) -> Option<String> {
        self.last.lock().expect("wgpu device error state").clone()
    }
}

/// Headless wgpu device shared by this feature and future consumers.
/// `Send + Sync`; every handle wraps the same underlying device.
pub struct WgpuEngine {
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter_label: String,
    errors: Arc<DeviceErrors>,
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
        let errors = DeviceErrors::install(&device);
        tracing::debug!(target: LOG, "wgpu device acquired");
        Ok(Self {
            device,
            queue,
            adapter_label,
            errors,
            readback: Mutex::new(None),
            fence: EngineFence::new(),
        })
    }

    /// Blocking [`Self::new`] bounded by `INIT_DEADLINE`: the init runs on
    /// a dedicated thread, so a hung `request_adapter` cannot outlive its
    /// caller; on timeout that thread is detached. Never call from a UI
    /// thread.
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
    /// thread bounded by `FRAME_DEADLINE`. A missed deadline poisons the
    /// engine: later fenced calls refuse work without touching wgpu, and
    /// the wedged thread stays detached. Never call from a UI thread.
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

    /// Fails with a distinct cause once the device is lost, so the error
    /// panel can tell a driver loss from a render failure.
    fn ensure_device_alive(&self) -> Result<()> {
        if !self.errors.device_lost() {
            return Ok(());
        }
        let detail = self
            .errors
            .last_error()
            .unwrap_or_else(|| "no uncaptured error recorded".to_string());
        anyhow::bail!("wgpu device lost ({detail}); rebuild the engine to retry")
    }

    /// Reads a COPY_SRC RGBA8 2D texture as tightly packed rows (width*4
    /// per row). BLOCKING; background thread only. The readback buffer is
    /// reused per size, so calls on one engine serialize behind its lock.
    pub fn read_texture_bytes(
        &self,
        texture: &wgpu::Texture,
        width: u32,
        height: u32,
    ) -> Result<Vec<u8>> {
        self.ensure_device_alive()?;
        let padded = padded_bytes_per_row(width);
        let size = (padded * height as usize) as u64;
        let mut slot = self
            .readback
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
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
        self.ensure_device_alive()?;
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
