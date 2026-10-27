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
async fn callback_downmixes_four_channels_into_a_stereo_producer(cx: &mut TestAppContext) {
    let state = cx.new(|_| WaveformState::new());
    let mut producer = state.update(cx, |state, cx| {
        state.attach_source(48_000, 2, DEFAULT_RETENTION, cx)
    });
    let meter = LevelMeter::new();
    let gain_q8 = AtomicI32::new(0);

    let frames = 128;
    let mut buffer = Vec::with_capacity(frames * 4);
    for frame in 0..frames {
        for channel in 0..4 {
            buffer.push(((frame % 100) as i16 - 50) * (channel as i16 + 1) * 60);
        }
    }
    convert_chunk(&buffer, &mut producer, &meter, &gain_q8, 4, 2);

    cx.update(|cx| {
        assert_eq!(state.read(cx).channels(), 2);
        assert_eq!(state.read(cx).dropped_samples(), 0);
        state.update(cx, |state, cx| state.refresh(cx));
        assert_eq!(state.read(cx).dropped_samples(), 0);
    });
    let (_, peak) = meter.read();
    assert!(peak > 0.);
}
