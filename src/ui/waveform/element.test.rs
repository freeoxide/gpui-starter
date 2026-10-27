// No glob `use gpui_kit::*` here: it would import gpui's `test` proc-macro and
// shadow this file's `#[gpui_kit::test]` attribute.
use std::time::Duration;

use gpui_kit::prelude::*;
use gpui_kit::{AppContext, Entity, TestAppContext, Window};

use crate::ui::waveform::{DEFAULT_RETENTION, MIN_WINDOW, Waveform, WaveformMode, WaveformState};

const SAMPLE_RATE: u32 = 48_000;

struct WaveformHost {
    state: Entity<WaveformState>,
}

impl WaveformHost {
    fn new(state: Entity<WaveformState>, cx: &mut Context<Self>) -> Self {
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        Self { state }
    }
}

impl Render for WaveformHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Waveform::new("test-waveform", self.state.clone())
    }
}

fn sine_interleaved(frames: usize, freq: f32) -> Vec<f32> {
    (0..frames)
        .flat_map(|i| {
            let s = (2. * std::f32::consts::PI * freq * i as f32 / SAMPLE_RATE as f32).sin();
            [s, s]
        })
        .collect()
}

fn draw_window(cx: &mut TestAppContext, state: Entity<WaveformState>) {
    cx.update(gpui_kit::component::theme::init);
    cx.add_window(|_, cx| WaveformHost::new(state.clone(), cx));
}

#[gpui_kit::test]
fn detached_idle_state_draws(cx: &mut TestAppContext) {
    let state = cx.new(|_| WaveformState::new());
    draw_window(cx, state.clone());
    assert!(!cx.read(|cx| state.read(cx).has_source()));
    assert_eq!(
        cx.read(|cx| state.read(cx).mode()),
        WaveformMode::Oscilloscope
    );
}

#[gpui_kit::test]
fn mono_attached_draws_after_refresh(cx: &mut TestAppContext) {
    let state = cx.new(|_| WaveformState::new());
    let mut producer = state.update(cx, |s, cx| {
        s.attach_source(SAMPLE_RATE, 1, DEFAULT_RETENTION, cx)
    });
    producer.push_interleaved(&sine_interleaved(2048, 440.));
    state.update(cx, |s, cx| s.refresh(cx));
    draw_window(cx, state.clone());
    assert_eq!(cx.read(|cx| state.read(cx).channels()), 1);
    assert!(cx.read(|cx| state.read(cx).spectrum().is_some()));
}

#[gpui_kit::test]
fn stereo_attached_draws_in_both_modes(cx: &mut TestAppContext) {
    for mode in [WaveformMode::Oscilloscope, WaveformMode::Spectrum] {
        let state = cx.new(|_| WaveformState::new());
        let mut producer = state.update(cx, |s, cx| {
            s.attach_source(SAMPLE_RATE, 2, DEFAULT_RETENTION, cx)
        });
        producer.push_interleaved(&sine_interleaved(2048, 440.));
        state.update(cx, |s, cx| {
            s.refresh(cx);
            s.set_mode(mode, cx);
        });
        draw_window(cx, state.clone());
        assert_eq!(cx.read(|cx| state.read(cx).mode()), mode);
        assert_eq!(cx.read(|cx| state.read(cx).channels()), 2);
    }
}

#[gpui_kit::test]
fn stereo_paused_after_data_draws_frozen_display(cx: &mut TestAppContext) {
    let state = cx.new(|_| WaveformState::new());
    let mut producer = state.update(cx, |s, cx| {
        s.attach_source(SAMPLE_RATE, 2, DEFAULT_RETENTION, cx)
    });
    producer.push_interleaved(&sine_interleaved(2048, 440.));
    state.update(cx, |s, cx| {
        s.refresh(cx);
        s.set_paused(true, cx);
    });
    draw_window(cx, state.clone());
    assert!(cx.read(|cx| state.read(cx).is_paused()));
    assert!(cx.read(|cx| state.read(cx).spectrum().is_some()));
}

#[gpui_kit::test]
fn short_window_draws_line_mode(cx: &mut TestAppContext) {
    let state = cx.new(|_| WaveformState::new());
    let mut producer = state.update(cx, |s, cx| {
        s.attach_source(SAMPLE_RATE, 2, DEFAULT_RETENTION, cx)
    });
    producer.push_interleaved(&sine_interleaved(2048, 440.));
    state.update(cx, |s, cx| {
        s.refresh(cx);
        s.set_window(MIN_WINDOW, cx);
    });
    draw_window(cx, state.clone());
    assert_eq!(
        cx.read(|cx| state.read(cx).window()),
        Duration::from_millis(50)
    );
    let window_samples = cx.read(|cx| state.read(cx).window_samples());
    assert!(window_samples <= 50 * SAMPLE_RATE as usize / 1000 + 1);
}
