use std::sync::Arc;

use super::*;

#[test]
fn fixed_point_roundtrip_is_exact_at_anchor_amplitudes() {
    for amplitude in [0.0f32, 0.5, 1.0] {
        assert_eq!(decode(encode(amplitude)), amplitude);
    }
}

#[test]
fn update_then_read_returns_the_written_values() {
    let meter = LevelMeter::new();
    meter.update(0.2, 0.7);
    let (rms, peak) = meter.read();
    assert!((rms - 0.2).abs() < 1. / SCALE);
    assert!((peak - 0.7).abs() < 1. / SCALE);
}

#[test]
fn consecutive_updates_accumulate_the_max_peak() {
    let meter = LevelMeter::new();
    meter.update(0.5, 0.3);
    meter.update(0.1, 0.9);
    let (rms, peak) = meter.read();
    assert!((peak - 0.9).abs() < 1. / SCALE);
    assert!((rms - 0.1).abs() < 1. / SCALE);
}

#[test]
fn read_resets_the_peak_but_not_the_rms() {
    let meter = LevelMeter::new();
    meter.update(0.4, 0.8);
    let _ = meter.read();
    let (rms, peak) = meter.read();
    assert_eq!(peak, 0.0);
    assert!((rms - 0.4).abs() < 1. / SCALE);
}

#[test]
fn reset_zeroes_both_values() {
    let meter = LevelMeter::new();
    meter.update(0.6, 0.6);
    meter.reset();
    assert_eq!(meter.read(), (0.0, 0.0));
}

#[test]
fn update_clamps_out_of_range_amplitudes() {
    let meter = LevelMeter::new();
    meter.update(2.0, -1.0);
    let (rms, peak) = meter.read();
    assert_eq!(rms, 1.0);
    assert_eq!(peak, 0.0);
}

fn _assert_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Arc<LevelMeter>>();
}
