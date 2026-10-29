// The native render loop is the only consumer; wasm compiles the module
// without it.
#![cfg_attr(target_family = "wasm", allow(dead_code))]

use std::sync::Arc;

use gpui_kit::{Context, RenderImage};

use super::demos::{DemoId, DemoSpec, ShaderParams, demo};
use crate::features::wgpu::FpsTracker;
use crate::platform::clock::{Duration, Instant};

const FRAME_INTERVAL: Duration = Duration::from_millis(1000 / 60);
const STALE_FRAME_CAP: usize = 4;
const MAX_DELTA: f32 = 0.25;

pub(in crate::features::pages::shaders) struct ShaderState {
    demo: DemoId,
    playing: bool,
    time: f32,
    delta: f32,
    slider: f32,
    mouse: (f32, f32),
    frame_in_flight: bool,
    last_frame_started_at: Option<Instant>,
    next_frame_at: Option<Instant>,
    fps: FpsTracker,
    displayed: Option<Arc<RenderImage>>,
    stale_frames: Vec<Arc<RenderImage>>,
    runtime_error: Option<String>,
    frames_rendered: u64,
    fit_pending: bool,
}

impl ShaderState {
    pub(in crate::features::pages::shaders) fn new(id: DemoId, cx: &Context<Self>) -> Self {
        Self {
            demo: id,
            playing: !cx.reduce_motion(),
            time: 0.0,
            delta: 1.0 / 60.0,
            slider: demo(id).slider.default,
            mouse: (0.5, 0.5),
            frame_in_flight: false,
            last_frame_started_at: None,
            next_frame_at: None,
            fps: FpsTracker::new(),
            displayed: None,
            stale_frames: Vec::new(),
            runtime_error: None,
            frames_rendered: 0,
            fit_pending: true,
        }
    }

    pub(in crate::features::pages::shaders) fn demo(&self) -> DemoId {
        self.demo
    }

    pub(in crate::features::pages::shaders) fn set_demo(
        &mut self,
        id: DemoId,
        cx: &mut Context<Self>,
    ) {
        if id == self.demo {
            return;
        }
        self.demo = id;
        self.time = 0.0;
        self.mouse = (0.5, 0.5);
        self.slider = demo(id).slider.default;
        self.runtime_error = None;
        self.frames_rendered = 0;
        self.fit_pending = true;
        cx.notify();
    }

    pub(in crate::features::pages::shaders) fn is_playing(&self) -> bool {
        self.playing
    }

    pub(in crate::features::pages::shaders) fn toggle_playing(&mut self, cx: &mut Context<Self>) {
        self.playing = !self.playing;
        if !self.playing {
            self.frame_in_flight = false;
        } else {
            self.next_frame_at = None;
        }
        cx.notify();
    }

    pub(in crate::features::pages::shaders) fn slider(&self) -> f32 {
        self.slider
    }

    pub(in crate::features::pages::shaders) fn set_slider(
        &mut self,
        value: f32,
        cx: &mut Context<Self>,
    ) {
        let spec = demo(self.demo).slider;
        let clamped = value.clamp(spec.min, spec.max);
        if clamped != self.slider {
            self.slider = clamped;
            cx.notify();
        }
    }

    pub(in crate::features::pages::shaders) fn set_mouse(
        &mut self,
        uv: (f32, f32),
        _cx: &mut Context<Self>,
    ) {
        self.mouse = (uv.0.clamp(0.0, 1.0), uv.1.clamp(0.0, 1.0));
    }

    pub(in crate::features::pages::shaders) fn is_frame_in_flight(&self) -> bool {
        self.frame_in_flight
    }

    pub(in crate::features::pages::shaders) fn mark_frame_started(
        &mut self,
        now: Instant,
        _cx: &mut Context<Self>,
    ) {
        self.last_frame_started_at = Some(now);
        self.frame_in_flight = true;
    }

    pub(in crate::features::pages::shaders) fn frame_done(
        &mut self,
        now: Instant,
        image: Arc<RenderImage>,
        cx: &mut Context<Self>,
    ) {
        if !self.playing {
            return;
        }
        let delta = match self.last_frame_started_at {
            Some(started) => (now - started).as_secs_f32().min(MAX_DELTA),
            None => 1.0 / 60.0,
        };
        self.delta = delta;
        self.time += delta;
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

    pub(in crate::features::pages::shaders) fn take_stale_frames(
        &mut self,
    ) -> Vec<Arc<RenderImage>> {
        std::mem::take(&mut self.stale_frames)
    }

    pub(in crate::features::pages::shaders) fn fps(&self) -> u32 {
        self.fps.fps()
    }

    pub(in crate::features::pages::shaders) fn frames_rendered(&self) -> u64 {
        self.frames_rendered
    }

    pub(in crate::features::pages::shaders) fn runtime_error(&self) -> Option<&str> {
        self.runtime_error.as_deref()
    }

    pub(in crate::features::pages::shaders) fn frame_failed(
        &mut self,
        err: impl Into<String>,
        cx: &mut Context<Self>,
    ) {
        self.frame_in_flight = false;
        self.runtime_error = Some(err.into());
        cx.notify();
    }

    pub(in crate::features::pages::shaders) fn clear_error(&mut self, cx: &mut Context<Self>) {
        if self.runtime_error.take().is_some() {
            cx.notify();
        }
    }

    pub(in crate::features::pages::shaders) fn pacing_wait(&self, now: Instant) -> Duration {
        match self.next_frame_at {
            Some(deadline) => deadline
                .checked_duration_since(now)
                .unwrap_or(Duration::ZERO),
            None => Duration::ZERO,
        }
    }

    pub(in crate::features::pages::shaders) fn params_snapshot(
        &self,
        spec: &DemoSpec,
    ) -> ShaderParams {
        ShaderParams {
            time: self.time,
            delta: self.delta,
            slider: self.slider,
            resolution: (spec.width as f32, spec.height as f32),
            mouse: self.mouse,
        }
    }

    pub(in crate::features::pages::shaders) fn take_fit(&mut self) -> bool {
        std::mem::take(&mut self.fit_pending)
    }

    /// Queues a zoom-to-fit on the next ticker paint.
    pub(in crate::features::pages::shaders) fn request_fit(&mut self, cx: &mut Context<Self>) {
        self.fit_pending = true;
        cx.notify();
    }
}

#[cfg(test)]
#[path = "state.test.rs"]
mod state_test;
