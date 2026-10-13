// No glob `use super::*`: gpui's `test` proc-macro glob would make bare
// `#[test]` recurse infinitely. Import only what we need.
use std::time::Duration;

use gpui_kit::{AppContext, Entity, TestAppContext};

use crate::ui::waveform::{DEFAULT_RETENTION, SampleProducer, WaveformMode, WaveformState};

const SAMPLE_RATE: u32 = 48_000;

fn sine_interleaved(frames: usize, freq: f32, amplitude: f32) -> Vec<f32> {
    (0..frames)
        .flat_map(|i| {
            let s = (2. * std::f32::consts::PI * freq * i as f32 / SAMPLE_RATE as f32).sin()
                * amplitude;
            [s, s]
        })
        .collect()
}

fn silence_interleaved(frames: usize) -> Vec<f32> {
    vec![0.; frames * 2]
}

fn attach_stereo(cx: &mut TestAppContext, state: &Entity<WaveformState>) -> SampleProducer {
    state.update(cx, |s, cx| {
        s.attach_source(SAMPLE_RATE, 2, DEFAULT_RETENTION, cx)
    })
}

fn refresh(cx: &mut TestAppContext, state: &Entity<WaveformState>) {
    state.update(cx, |s, cx| s.refresh(cx));
}

#[gpui_kit::test]
fn attach_returns_matching_producer(cx: &mut TestAppContext) {
    let state = cx.new(|_| WaveformState::new());
    let producer = attach_stereo(cx, &state);
    assert_eq!(producer.sample_rate(), SAMPLE_RATE);
    assert_eq!(producer.channels(), 2);
    assert!(cx.read(|cx| state.read(cx).has_source()));
    assert_eq!(cx.read(|cx| state.read(cx).channels()), 2);
    assert_eq!(
        cx.read(|cx| state.read(cx).sample_rate()),
        Some(SAMPLE_RATE)
    );
}

#[gpui_kit::test]
fn refresh_integrates_stereo_data_and_builds_spectrum(cx: &mut TestAppContext) {
    let state = cx.new(|_| WaveformState::new());
    let mut producer = attach_stereo(cx, &state);
    assert!(cx.read(|cx| state.read(cx).spectrum().is_none()));
    producer.push_interleaved(&sine_interleaved(480, 440., 1.));
    refresh(cx, &state);
    assert_eq!(cx.read(|cx| state.read(cx).channels()), 2);
    let (peak_height, band) = cx.read(|cx| {
        let frame = state.read(cx).spectrum().expect("spectrum frame");
        let ix = frame
            .bars()
            .iter()
            .enumerate()
            .fold(
                (0, 0_f32),
                |(bi, bh), (i, &h)| if h > bh { (i, h) } else { (bi, bh) },
            )
            .0;
        (frame.bars()[ix], frame.bar_band(ix).unwrap())
    });
    assert!(peak_height > 0., "spectrum has a nonzero peak");
    assert!(band.0 <= 440. && 440. <= band.1, "band {:?}", band);
}

#[gpui_kit::test]
fn pause_freezes_display_and_resume_integrates_gap(cx: &mut TestAppContext) {
    let state = cx.new(|_| WaveformState::new());
    let mut producer = attach_stereo(cx, &state);
    producer.push_interleaved(&sine_interleaved(480, 440., 1.));
    refresh(cx, &state);

    state.update(cx, |s, cx| s.set_paused(true, cx));
    producer.push_interleaved(&silence_interleaved(200));
    refresh(cx, &state);
    refresh(cx, &state);
    assert!(cx.read(|cx| {
        state
            .read(cx)
            .spectrum()
            .unwrap()
            .bars()
            .iter()
            .any(|&v| v > 0.)
    }));

    state.update(cx, |s, cx| s.set_paused(false, cx));
    producer.push_interleaved(&silence_interleaved(1100));
    refresh(cx, &state);
    assert!(cx.read(|cx| {
        state
            .read(cx)
            .spectrum()
            .unwrap()
            .bars()
            .iter()
            .all(|&v| v == 0.)
    }));
}

#[gpui_kit::test]
fn set_window_clamps_to_min_window_and_retention(cx: &mut TestAppContext) {
    let state = cx.new(|_| WaveformState::new());
    attach_stereo(cx, &state);
    let retention = cx.read(|cx| state.read(cx).retention());
    state.update(cx, |s, cx| s.set_window(Duration::from_millis(1), cx));
    assert_eq!(
        cx.read(|cx| state.read(cx).window()),
        Duration::from_millis(50)
    );
    state.update(cx, |s, cx| s.set_window(Duration::from_secs(999), cx));
    assert_eq!(cx.read(|cx| state.read(cx).window()), retention);
}

#[gpui_kit::test]
fn detach_clears_source_and_abandons_producer(cx: &mut TestAppContext) {
    let state = cx.new(|_| WaveformState::new());
    let producer = attach_stereo(cx, &state);
    assert!(producer.is_attached());
    state.update(cx, |s, cx| s.detach_source(cx));
    assert!(!cx.read(|cx| state.read(cx).has_source()));
    assert_eq!(cx.read(|cx| state.read(cx).channels()), 0);
    assert!(cx.read(|cx| state.read(cx).spectrum().is_none()));
    assert!(!producer.is_attached());
}

#[gpui_kit::test]
fn overflow_reports_dropped_samples_on_both_sides(cx: &mut TestAppContext) {
    let state = cx.new(|_| WaveformState::new());
    let mut producer = attach_stereo(cx, &state);
    producer.push_interleaved(&vec![0.; 20_000 * 2]);
    let dropped = cx.read(|cx| state.read(cx).dropped_samples());
    assert_eq!(dropped, 3_616 * 2);
    assert_eq!(dropped, producer.dropped_samples());
}

#[gpui_kit::test]
fn tick_timer_drives_refresh_without_manual_calls(cx: &mut TestAppContext) {
    let state = cx.update(|cx| cx.new(|_| WaveformState::new()));
    let mut producer = attach_stereo(cx, &state);
    producer.push_interleaved(&sine_interleaved(480, 440., 1.));
    assert!(cx.read(|cx| state.read(cx).spectrum().is_none()));
    cx.executor().advance_clock(Duration::from_millis(2 * 33));
    cx.executor().run_until_parked();
    assert!(cx.read(|cx| state.read(cx).spectrum().is_some()));
}

#[gpui_kit::test]
fn set_mode_round_trips(cx: &mut TestAppContext) {
    let state = cx.new(|_| WaveformState::new());
    assert_eq!(
        cx.read(|cx| state.read(cx).mode()),
        WaveformMode::Oscilloscope
    );
    state.update(cx, |s, cx| s.set_mode(WaveformMode::Spectrum, cx));
    assert_eq!(cx.read(|cx| state.read(cx).mode()), WaveformMode::Spectrum);
}
