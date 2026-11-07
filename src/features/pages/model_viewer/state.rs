// The native render loop is the only consumer; wasm compiles the module
// without it.
#![cfg_attr(target_family = "wasm", allow(dead_code))]

use std::sync::Arc;

use gpui_kit::{Context, RenderImage};

use super::obj::{Mesh, MeshStats};
use crate::features::wgpu::FpsTracker;
use crate::platform::clock::{Duration, Instant};

const FRAME_INTERVAL: Duration = Duration::from_millis(1000 / 60);
const STALE_FRAME_CAP: usize = 4;
const MAX_DELTA: f32 = 0.25;
const DEFAULT_YAW: f32 = 0.6;
const DEFAULT_PITCH: f32 = 0.4;
const DEFAULT_DISTANCE: f32 = 4.0;
const PITCH_LIMIT: f32 = 1.45;
const MIN_DISTANCE: f32 = 1.6;
const MAX_DISTANCE: f32 = 16.0;
const SPIN_RATE: f32 = 0.5;
/// Viewport reconfigure quantum: smaller viewport churn (scrollbars,
/// fractional resizes) must not rebuild render targets per frame.
const VIEWPORT_STEP: u32 = 8;
const VIEWPORT_MIN: u32 = 64;
const VIEWPORT_MAX: u32 = 4096;

/// Everything one frame needs; `swap_mesh` installs a newly loaded model
/// inside the fenced render (buffer creation is queue work).
pub(in crate::features::pages::model_viewer) struct RenderJob {
    pub(in crate::features::pages::model_viewer) width: u32,
    pub(in crate::features::pages::model_viewer) height: u32,
    pub(in crate::features::pages::model_viewer) yaw: f32,
    pub(in crate::features::pages::model_viewer) pitch: f32,
    pub(in crate::features::pages::model_viewer) distance: f32,
    pub(in crate::features::pages::model_viewer) spin: f32,
    pub(in crate::features::pages::model_viewer) swap_mesh: Option<Arc<Mesh>>,
}

pub(in crate::features::pages::model_viewer) struct ViewerState {
    auto_rotate: bool,
    yaw: f32,
    pitch: f32,
    distance: f32,
    spin: f32,
    /// Currently installed mesh; the renderer restart path re-reads it.
    mesh: Arc<Mesh>,
    mesh_name: String,
    mesh_stats: MeshStats,
    /// Set until a frame carrying the new mesh has landed; the renderer
    /// swaps its buffers from the job, and a failed render retries it.
    pending_mesh: Option<Arc<Mesh>>,
    viewport: (u32, u32),
    frame_in_flight: bool,
    last_frame_started_at: Option<Instant>,
    next_frame_at: Option<Instant>,
    fps: FpsTracker,
    displayed: Option<Arc<RenderImage>>,
    stale_frames: Vec<Arc<RenderImage>>,
    runtime_error: Option<String>,
    frames_rendered: u64,
    /// On-demand kick while paused: camera or model changed.
    needs_render: bool,
    loading: bool,
}

impl ViewerState {
    pub(in crate::features::pages::model_viewer) fn new(cx: &Context<Self>) -> Self {
        let mesh = Arc::new(super::obj::builtin_torus());
        let mesh_stats = mesh.stats;
        Self {
            auto_rotate: !cx.reduce_motion(),
            yaw: DEFAULT_YAW,
            pitch: DEFAULT_PITCH,
            distance: DEFAULT_DISTANCE,
            spin: 0.0,
            mesh,
            mesh_stats,
            mesh_name: String::new(),
            pending_mesh: None,
            viewport: (0, 0),
            frame_in_flight: false,
            last_frame_started_at: None,
            next_frame_at: None,
            fps: FpsTracker::new(),
            displayed: None,
            stale_frames: Vec::new(),
            runtime_error: None,
            frames_rendered: 0,
            needs_render: true,
            loading: false,
        }
    }

    pub(in crate::features::pages::model_viewer) fn orbit_by(
        &mut self,
        dx: f32,
        dy: f32,
        cx: &mut Context<Self>,
    ) {
        self.yaw += dx;
        self.pitch = (self.pitch + dy).clamp(-PITCH_LIMIT, PITCH_LIMIT);
        self.needs_render = true;
        cx.notify();
    }

    pub(in crate::features::pages::model_viewer) fn zoom_by(
        &mut self,
        factor: f32,
        cx: &mut Context<Self>,
    ) {
        self.distance = (self.distance * factor).clamp(MIN_DISTANCE, MAX_DISTANCE);
        self.needs_render = true;
        cx.notify();
    }

    pub(in crate::features::pages::model_viewer) fn reset_view(&mut self, cx: &mut Context<Self>) {
        self.yaw = DEFAULT_YAW;
        self.pitch = DEFAULT_PITCH;
        self.distance = DEFAULT_DISTANCE;
        self.spin = 0.0;
        self.needs_render = true;
        cx.notify();
    }

    pub(in crate::features::pages::model_viewer) fn is_auto_rotating(&self) -> bool {
        self.auto_rotate
    }

    pub(in crate::features::pages::model_viewer) fn toggle_auto_rotate(
        &mut self,
        cx: &mut Context<Self>,
    ) {
        self.auto_rotate = !self.auto_rotate;
        if !self.auto_rotate {
            self.frame_in_flight = false;
        } else {
            self.next_frame_at = None;
        }
        cx.notify();
    }

    pub(in crate::features::pages::model_viewer) fn set_loading(
        &mut self,
        loading: bool,
        cx: &mut Context<Self>,
    ) {
        self.loading = loading;
        cx.notify();
    }

    pub(in crate::features::pages::model_viewer) fn is_loading(&self) -> bool {
        self.loading
    }

    pub(in crate::features::pages::model_viewer) fn install_mesh(
        &mut self,
        mesh: Arc<Mesh>,
        name: impl Into<String>,
        cx: &mut Context<Self>,
    ) {
        self.mesh_stats = mesh.stats;
        self.mesh = mesh.clone();
        self.mesh_name = name.into();
        self.pending_mesh = Some(mesh);
        self.spin = 0.0;
        self.runtime_error = None;
        self.needs_render = true;
        cx.notify();
    }

    /// The mesh the renderer rebuild starts from (a runtime restart after a
    /// GPU failure re-uploads the model the user last loaded).
    pub(in crate::features::pages::model_viewer) fn mesh_for_restart(&self) -> Arc<Mesh> {
        self.mesh.clone()
    }

    pub(in crate::features::pages::model_viewer) fn mesh_name(&self) -> &str {
        &self.mesh_name
    }

    pub(in crate::features::pages::model_viewer) fn mesh_stats(&self) -> MeshStats {
        self.mesh_stats
    }

    pub(in crate::features::pages::model_viewer) fn has_loaded_model(&self) -> bool {
        !self.mesh_name.is_empty()
    }

    /// Reports the physical viewport size; true when the quantized size
    /// changed and the next frame must rebuild render targets.
    pub(in crate::features::pages::model_viewer) fn viewport_changed(
        &mut self,
        (width, height): (u32, u32),
        cx: &mut Context<Self>,
    ) -> bool {
        let quantize =
            |value: u32| (value / VIEWPORT_STEP * VIEWPORT_STEP).clamp(VIEWPORT_MIN, VIEWPORT_MAX);
        let next = (quantize(width.max(1)), quantize(height.max(1)));
        if next == self.viewport {
            return false;
        }
        self.viewport = next;
        self.needs_render = true;
        cx.notify();
        true
    }

    pub(in crate::features::pages::model_viewer) fn wants_frame(&self) -> bool {
        self.auto_rotate || self.needs_render || self.pending_mesh.is_some()
    }

    pub(in crate::features::pages::model_viewer) fn viewport_is_sized(&self) -> bool {
        self.viewport != (0, 0)
    }

    pub(in crate::features::pages::model_viewer) fn is_frame_in_flight(&self) -> bool {
        self.frame_in_flight
    }

    pub(in crate::features::pages::model_viewer) fn mark_frame_started(
        &mut self,
        now: Instant,
        _cx: &mut Context<Self>,
    ) {
        self.last_frame_started_at = Some(now);
        self.frame_in_flight = true;
        self.needs_render = false;
    }

    pub(in crate::features::pages::model_viewer) fn take_render_job(&self) -> RenderJob {
        RenderJob {
            width: self.viewport.0,
            height: self.viewport.1,
            yaw: self.yaw,
            pitch: self.pitch,
            distance: self.distance,
            spin: self.spin,
            swap_mesh: self.pending_mesh.clone(),
        }
    }

    /// On-demand frames (paused orbit/zoom) must land, so unlike the shader
    /// canvas there is no playing gate here.
    pub(in crate::features::pages::model_viewer) fn frame_done(
        &mut self,
        now: Instant,
        image: Arc<RenderImage>,
        cx: &mut Context<Self>,
    ) {
        let delta = match self.last_frame_started_at {
            Some(started) => (now - started).as_secs_f32().min(MAX_DELTA),
            None => 1.0 / 60.0,
        };
        if self.auto_rotate {
            self.spin += delta * SPIN_RATE;
        }
        self.pending_mesh = None;
        if let Some(previous) = self.displayed.take() {
            self.stale_frames.push(previous);
            if self.stale_frames.len() > STALE_FRAME_CAP {
                self.stale_frames.remove(0);
            }
        }
        self.displayed = Some(image);
        self.fps.push(now);
        self.frames_rendered += 1;
        self.frame_in_flight = false;
        // The deadline advances from the previous deadline, not now, so
        // render time overlaps the sleep; the clamp drops missed-slot debt.
        self.next_frame_at = Some(match self.next_frame_at {
            Some(previous) => (previous + FRAME_INTERVAL).max(now),
            None => now + FRAME_INTERVAL,
        });
        cx.notify();
    }

    pub(in crate::features::pages::model_viewer) fn take_stale_frames(
        &mut self,
    ) -> Vec<Arc<RenderImage>> {
        std::mem::take(&mut self.stale_frames)
    }

    pub(in crate::features::pages::model_viewer) fn displayed(&self) -> Option<&Arc<RenderImage>> {
        self.displayed.as_ref()
    }

    pub(in crate::features::pages::model_viewer) fn fps(&self) -> u32 {
        self.fps.fps()
    }

    pub(in crate::features::pages::model_viewer) fn frames_rendered(&self) -> u64 {
        self.frames_rendered
    }

    pub(in crate::features::pages::model_viewer) fn runtime_error(&self) -> Option<&str> {
        self.runtime_error.as_deref()
    }

    pub(in crate::features::pages::model_viewer) fn frame_failed(
        &mut self,
        err: impl Into<String>,
        cx: &mut Context<Self>,
    ) {
        self.frame_in_flight = false;
        self.runtime_error = Some(err.into());
        cx.notify();
    }

    pub(in crate::features::pages::model_viewer) fn clear_error(&mut self, cx: &mut Context<Self>) {
        if self.runtime_error.take().is_some() {
            cx.notify();
        }
    }

    pub(in crate::features::pages::model_viewer) fn pacing_wait(&self, now: Instant) -> Duration {
        match self.next_frame_at {
            Some(deadline) => deadline
                .checked_duration_since(now)
                .unwrap_or(Duration::ZERO),
            None => Duration::ZERO,
        }
    }
}

#[cfg(test)]
#[path = "state.test.rs"]
mod state_test;
