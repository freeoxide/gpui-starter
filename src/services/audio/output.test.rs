use super::{
    SharedTransport, adapt_to_channels, fill_buffer, pick_channels_index, pick_rate,
    resample_linear,
};

#[test]
fn pick_rate_keeps_in_range_sources() {
    assert_eq!(pick_rate(44_100, 192_000, 44_100), 44_100);
    assert_eq!(pick_rate(44_100, 192_000, 96_000), 96_000);
    assert_eq!(pick_rate(44_100, 192_000, 22_050), 44_100);
    assert_eq!(pick_rate(44_100, 192_000, 250_000), 192_000);
}

#[test]
fn pick_channels_prefers_two_then_wide_then_mono() {
    assert_eq!(pick_channels_index(&[2, 6, 1]), Some(0));
    assert_eq!(pick_channels_index(&[6, 2]), Some(1));
    assert_eq!(pick_channels_index(&[6, 1]), Some(0));
    assert_eq!(pick_channels_index(&[1, 6]), Some(1));
    assert_eq!(pick_channels_index(&[1]), Some(0));
    assert_eq!(pick_channels_index(&[5, 7]), Some(0));
    assert_eq!(pick_channels_index(&[]), None);
}

#[test]
fn fill_buffer_copies_and_advances() {
    let transport = SharedTransport::new(1.);
    let mut out = [0f32; 10];
    let mut tap = Vec::new();
    let eof = fill_buffer(&[0.5; 100], 1, &transport, &mut out, &mut tap);
    assert!(!eof);
    assert!(out.iter().all(|s| *s == 0.5));
    assert_eq!(tap.len(), 10);
    assert_eq!(transport.cursor_frames(), 10);
}

#[test]
fn fill_buffer_applies_volume() {
    let transport = SharedTransport::new(0.5);
    let mut out = [0f32; 4];
    let mut tap = Vec::new();
    fill_buffer(&[0.5; 100], 1, &transport, &mut out, &mut tap);
    assert_eq!(out, [0.25; 4]);
}

#[test]
fn fill_buffer_pads_with_silence_at_end_of_source() {
    let transport = SharedTransport::new(1.);
    transport.set_cursor(95);
    let mut out = [1f32; 10];
    let mut tap = Vec::new();
    let eof = fill_buffer(&[0.5; 100], 1, &transport, &mut out, &mut tap);
    assert!(eof);
    assert_eq!(&out[..5], &[0.5; 5]);
    assert_eq!(&out[5..], &[0.; 5]);
    assert_eq!(tap.len(), 5);
    assert_eq!(transport.cursor_frames(), 100);
}

#[test]
fn fill_buffer_preserves_stereo_interleaving() {
    let transport = SharedTransport::new(1.);
    let src: Vec<f32> = (0..20).flat_map(|_| [1., -1.]).collect();
    let mut out = [0f32; 8];
    let mut tap = Vec::new();
    fill_buffer(&src, 2, &transport, &mut out, &mut tap);
    assert_eq!(out, [1., -1., 1., -1., 1., -1., 1., -1.]);
}

#[test]
fn resample_is_identity_when_rates_match() {
    let src = vec![0.25f32, -0.5, 0.75];
    let out = resample_linear(&src, 1, 48_000, 48_000);
    assert_eq!(out, src);
}

#[test]
fn resample_stretches_dc_signal_to_target_length() {
    let src = vec![0.5f32; 44_100];
    let out = resample_linear(&src, 1, 44_100, 48_000);
    assert_eq!(out.len(), 48_000);
    assert!(out.iter().all(|s| (*s - 0.5).abs() < 1e-6));
}

#[test]
fn resample_keeps_sine_peak_within_tolerance() {
    let src: Vec<f32> = (0..44_100)
        .map(|i| ((2. * std::f64::consts::PI * 440. * i as f64 / 44_100.) as f32) * 0.8)
        .collect();
    let out = resample_linear(&src, 1, 44_100, 48_000);
    let src_peak = src.iter().fold(0f32, |acc, s| acc.max(s.abs()));
    let out_peak = out.iter().fold(0f32, |acc, s| acc.max(s.abs()));
    assert!((out_peak - src_peak).abs() / src_peak < 0.05);
}

#[test]
fn resample_keeps_stereo_channels_separate() {
    let frames = 44_100;
    let mut src = Vec::with_capacity(frames * 2);
    for i in 0..frames {
        src.push(1.);
        src.push(i as f32 / frames as f32);
    }
    let out = resample_linear(&src, 2, 44_100, 48_000);
    assert_eq!(out.len(), 48_000 * 2);
    assert!(out.iter().step_by(2).all(|s| *s == 1.));
    assert!(
        out.iter()
            .skip(1)
            .step_by(2)
            .all(|s| (0f32..=1.).contains(s))
    );
}

#[test]
fn adapt_duplicates_mono_and_averages_stereo() {
    assert_eq!(
        adapt_to_channels(&[0.5, -0.25], 1, 2),
        [0.5, 0.5, -0.25, -0.25]
    );
    assert_eq!(adapt_to_channels(&[0.5, -0.25], 2, 1), [0.125]);
    assert_eq!(adapt_to_channels(&[1., 2.], 2, 2), [1., 2.]);
}
