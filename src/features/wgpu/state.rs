// The native render loop is the only consumer; wasm compiles the module
// without it.
#![cfg_attr(target_family = "wasm", allow(dead_code))]

use std::sync::Arc;

use gpui_kit::{Context, RenderImage};

use super::frame::FpsTracker;
use crate::platform::clock::{Duration, Instant};

pub(in crate::features::wgpu) const PARTICLE_COUNTS: [u32; 3] = [100_000, 500_000, 1_000_000];

const FRAME_INTERVAL: Duration = Duration::from_millis(1000 / 60);
const STALE_FRAME_CAP: usize = 4;

/// Playback and sizing state for the wgpu page. Platform-neutral: holds no
/// wgpu or `DemoId` values, so it compiles on wasm too.
pub(in crate::features::wgpu) struct WgpuPageState {
    demo_ix: usize,
    playing: bool,
    particle_count: u32,
    pending_size: (u32, u32),
    frame_in_flight: bool,
    last_frame_started_at: Option<Instant>,
    next_frame_at: Option<Instant>,
    started_at: Instant,
    fps: FpsTracker,
    engine_ms_ema: f32,
    displayed: Option<Arc<RenderImage>>,
    stale_frames: Vec<Arc<RenderImage>>,
    runtime_error: Option<String>,
    frames_rendered: u64,
}

impl WgpuPageState {
    pub(in crate::features::wgpu) fn new(cx: &Context<Self>) -> Self {
        Self {
            demo_ix: 0,
            playing: !cx.reduce_motion(),
            particle_count: 1_000_000,
            pending_size: (0, 0),
            frame_in_flight: false,
            last_frame_started_at: None,
            next_frame_at: None,
            started_at: Instant::now(),
            fps: FpsTracker::new(),
            engine_ms_ema: 0.0,
            displayed: None,
            stale_frames: Vec::new(),
            runtime_error: None,
            frames_rendered: 0,
        }
    }

    pub(in crate::features::wgpu) fn demo_ix(&self) -> usize {
        self.demo_ix
    }

    pub(in crate::features::wgpu) fn set_demo_ix(&mut self, ix: usize, cx: &mut Context<Self>) {
        if ix == self.demo_ix {
            return;
        }
        self.demo_ix = ix;
        self.frames_rendered = 0;
        self.runtime_error = None;
        cx.notify();
    }

    pub(in crate::features::wgpu) fn is_playing(&self) -> bool {
        self.playing
    }

    pub(in crate::features::wgpu) fn toggle_playing(&mut self, cx: &mut Context<Self>) {
        self.playing = !self.playing;
        if !self.playing {
            self.frame_in_flight = false;
        } else {
            self.next_frame_at = None;
        }
        cx.notify();
    }

    pub(in crate::features::wgpu) fn particle_count(&self) -> u32 {
        self.particle_count
    }

    pub(in crate::features::wgpu) fn set_particle_count(
        &mut self,
        count: u32,
        cx: &mut Context<Self>,
    ) {
        if count == self.particle_count || !PARTICLE_COUNTS.contains(&count) {
            return;
        }
        self.particle_count = count;
        cx.notify();
    }

    pub(in crate::features::wgpu) fn viewport_changed(
        &mut self,
        physical: (u32, u32),
        _cx: &mut Context<Self>,
    ) {
        let quantized = (quantize_axis(physical.0), quantize_axis(physical.1));
        if quantized != self.pending_size {
            self.pending_size = quantized;
        }
    }

    pub(in crate::features::wgpu) fn pending_size(&self) -> (u32, u32) {
        self.pending_size
    }

    pub(in crate::features::wgpu) fn is_frame_in_flight(&self) -> bool {
        self.frame_in_flight
    }

    pub(in crate::features::wgpu) fn mark_frame_started(
        &mut self,
        now: Instant,
        _cx: &mut Context<Self>,
    ) {
        self.last_frame_started_at = Some(now);
        self.frame_in_flight = true;
    }

    pub(in crate::features::wgpu) fn frame_done(
        &mut self,
        now: Instant,
        image: Arc<RenderImage>,
        engine_ms: f32,
        cx: &mut Context<Self>,
    ) {
        if !self.playing {
            return;
        }
        if let Some(previous) = self.displayed.take() {
            self.stale_frames.push(previous);
            if self.stale_frames.len() > STALE_FRAME_CAP {
                self.stale_frames.remove(0);
            }
        }
        self.displayed = Some(image);
        self.fps.push(now);
        self.engine_ms_ema += 0.2 * (engine_ms - self.engine_ms_ema);
        self.frames_rendered += 1;
        self.frame_in_flight = false;
        // The deadline advances from the previous deadline, not from now, so
        // render time overlaps the sleep instead of adding to it; the clamp
        // drops the debt once a frame can no longer make its slot.
        self.next_frame_at = Some(match self.next_frame_at {
            Some(previous) => (previous + FRAME_INTERVAL).max(now),
            None => now + FRAME_INTERVAL,
        });
        cx.notify();
    }

    pub(in crate::features::wgpu) fn cancel_frame(&mut self, cx: &mut Context<Self>) {
        self.frame_in_flight = false;
        cx.notify();
    }

    pub(in crate::features::wgpu) fn take_stale_frames(&mut self) -> Vec<Arc<RenderImage>> {
        std::mem::take(&mut self.stale_frames)
    }

    pub(in crate::features::wgpu) fn displayed(&self) -> Option<&Arc<RenderImage>> {
        self.displayed.as_ref()
    }

    pub(in crate::features::wgpu) fn fps(&self) -> u32 {
        self.fps.fps()
    }

    pub(in crate::features::wgpu) fn engine_ms(&self) -> f32 {
        self.engine_ms_ema
    }

    pub(in crate::features::wgpu) fn frames_rendered(&self) -> u64 {
        self.frames_rendered
    }

    pub(in crate::features::wgpu) fn runtime_error(&self) -> Option<&str> {
        self.runtime_error.as_deref()
    }

    pub(in crate::features::wgpu) fn frame_failed(
        &mut self,
        err: impl Into<String>,
        cx: &mut Context<Self>,
    ) {
        self.frame_in_flight = false;
        self.runtime_error = Some(err.into());
        cx.notify();
    }

    pub(in crate::features::wgpu) fn clear_error(&mut self, cx: &mut Context<Self>) {
        if self.runtime_error.take().is_some() {
            cx.notify();
        }
    }

    pub(in crate::features::wgpu) fn pacing_wait(&self, now: Instant) -> Duration {
        match self.next_frame_at {
            Some(deadline) => deadline
                .checked_duration_since(now)
                .unwrap_or(Duration::ZERO),
            None => Duration::ZERO,
        }
    }

    pub(in crate::features::wgpu) fn frame_dt(&self, now: Instant) -> f32 {
        match self.last_frame_started_at {
            Some(started) => (now - started).as_secs_f32().min(0.25),
            None => 1.0 / 60.0,
        }
    }

    pub(in crate::features::wgpu) fn elapsed_at(&self, now: Instant) -> f32 {
        (now - self.started_at).as_secs_f32()
    }
}

fn quantize_axis(value: u32) -> u32 {
    (value.div_ceil(16) * 16).clamp(64, 4096)
}

#[cfg(test)]
#[path = "state.test.rs"]
mod state_test;
