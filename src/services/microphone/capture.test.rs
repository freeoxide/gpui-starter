use std::sync::atomic::AtomicI32;

use gpui_kit::{AppContext as _, TestAppContext};

use super::*;
use crate::ui::waveform::{DEFAULT_RETENTION, WaveformState};

fn device(id: &str, is_default: bool) -> InputDeviceInfo {
    InputDeviceInfo {
        id: id.to_string(),
        name: format!("Device {id}"),
        is_default,
        channels: 2,
        sample_rate: 48_000,
        sample_format: "F32".to_string(),
    }
}

#[test]
fn pick_device_prefers_the_exact_id_over_the_default() {
    let devices = vec![device("a", true), device("b", false)];
    let picked = pick_device(devices.iter(), Some("b"));
    assert_eq!(picked.unwrap().id, "b");
}

#[test]
fn pick_device_falls_back_to_default_when_the_wanted_id_is_stale() {
    let devices = vec![device("a", true), device("b", false)];
    let picked = pick_device(devices.iter(), Some("unplugged"));
    assert_eq!(picked.unwrap().id, "a");
}

#[test]
fn pick_device_picks_the_default_when_nothing_is_wanted() {
    let devices = vec![device("a", false), device("b", true)];
    let picked = pick_device(devices.iter(), None);
    assert_eq!(picked.unwrap().id, "b");
}

#[test]
fn pick_device_returns_none_for_an_empty_list() {
    let picked = pick_device(std::iter::empty(), None);
    assert!(picked.is_none());
}

#[test]
fn i16_conversion_covers_the_range() {
    assert_eq!((-32768i16).to_f32_(), -1.0);
    assert!((32767i16).to_f32_() - 0.99997 < 1e-4);
}

#[test]
fn u16_conversion_covers_the_range() {
    assert_eq!(0u16.to_f32_(), -1.0);
    assert!((65535u16).to_f32_() - 0.99997 < 1e-4);
}

#[test]
fn i32_conversion_covers_the_extremes() {
    assert_eq!(i32::MIN.to_f32_(), -1.0);
    assert!((i32::MAX.to_f32_() - 0.99999).abs() < 1e-4);
}

#[gpui_kit::test]
async fn convert_chunk_downmixes_the_first_two_device_channels(cx: &mut TestAppContext) {
    let state = cx.new(|_| WaveformState::new());
    let mut producer = state.update(cx, |state, cx| {
        state.attach_source(48_000, 2, DEFAULT_RETENTION, cx)
    });
    let meter = LevelMeter::new();
    let gain_q8 = AtomicI32::new(0);

    // 600 frames x 2 output samples crosses the 1024-sample buffer wrap; the
    // ch2/ch3 bait amplitudes make any wrong mapping shift rms and peak.
    let frames = 600;
    let mut buffer = Vec::with_capacity(frames * 4);
    for _ in 0..frames {
        buffer.extend_from_slice(&[0.30, 0.10, 0.90, 0.70]);
    }
    convert_chunk(&buffer, &mut producer, &meter, &gain_q8, 4, 2);

    state.update(cx, |state, cx| state.refresh(cx));
    cx.update(|cx| {
        assert!(state.read(cx).spectrum().is_some());
        assert_eq!(state.read(cx).dropped_samples(), 0);
    });
    let (rms, peak) = meter.read();
    assert!(
        (peak - 0.30).abs() < 1e-4,
        "peak must come from ch0: {peak}"
    );
    let expected = ((0.3f32 * 0.3 + 0.1 * 0.1) / 2.).sqrt();
    assert!(
        (rms - expected).abs() < 1e-4,
        "rms must mix ch0 and ch1: {rms}"
    );
}

#[gpui_kit::test]
async fn convert_chunk_zero_fills_the_missing_output_channel(cx: &mut TestAppContext) {
    let state = cx.new(|_| WaveformState::new());
    let mut producer = state.update(cx, |state, cx| {
        state.attach_source(48_000, 2, DEFAULT_RETENTION, cx)
    });
    let meter = LevelMeter::new();
    let gain_q8 = AtomicI32::new(0);

    let frames = 300;
    let buffer = vec![0.25f32; frames];
    convert_chunk(&buffer, &mut producer, &meter, &gain_q8, 1, 2);

    state.update(cx, |state, cx| state.refresh(cx));
    cx.update(|cx| assert!(state.read(cx).spectrum().is_some()));
    // Duplicating the mono channel into the right slot would raise rms to 0.25.
    let (rms, peak) = meter.read();
    assert!((peak - 0.25).abs() < 1e-4);
    let expected = (0.25f32 * 0.25 / 2.).sqrt();
    assert!(
        (rms - expected).abs() < 1e-4,
        "right channel must be zero: {rms}"
    );
}
