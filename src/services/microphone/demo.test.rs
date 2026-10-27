use super::*;

#[test]
fn fill_chunk_appends_stereo_frames() {
    let mut signal = DemoSignal::new();
    let mut out = Vec::new();
    signal.fill_chunk(&mut out, 1584);
    assert_eq!(out.len(), 3168);
}

#[test]
fn channels_are_identical() {
    let mut signal = DemoSignal::new();
    let mut out = Vec::new();
    signal.fill_chunk(&mut out, 1584);
    for frame in out.chunks(2) {
        assert_eq!(frame[0], frame[1]);
    }
}

#[test]
fn rms_and_peak_stay_within_unit_range() {
    let mut signal = DemoSignal::new();
    let mut out = Vec::new();
    let (rms, peak) = signal.fill_chunk(&mut out, 1584);
    assert!(rms > 0. && rms <= 1.);
    assert!(peak > 0. && peak <= 1.);
}

#[test]
fn peak_dominates_rms() {
    let mut signal = DemoSignal::new();
    let mut out = Vec::new();
    for _ in 0..8 {
        let (rms, peak) = signal.fill_chunk(&mut out, 1584);
        assert!(peak >= rms);
    }
}

#[test]
fn envelope_varies_over_time() {
    let mut signal = DemoSignal::new();
    let mut out = Vec::new();
    let (first, _) = signal.fill_chunk(&mut out, 1584);
    for _ in 0..30 {
        signal.fill_chunk(&mut out, 1584);
    }
    let mut later = Vec::new();
    let (second, _) = signal.fill_chunk(&mut later, 1584);
    assert!(
        (first - second).abs() > 1e-4,
        "0.25 Hz LFO must move the chunk RMS over ~2 s: {first} vs {second}"
    );
}

#[test]
fn fresh_signals_are_deterministic() {
    let (mut a, mut b) = (DemoSignal::new(), DemoSignal::new());
    let (mut left, mut right) = (Vec::new(), Vec::new());
    a.fill_chunk(&mut left, 1584);
    b.fill_chunk(&mut right, 1584);
    assert_eq!(left, right);
}
