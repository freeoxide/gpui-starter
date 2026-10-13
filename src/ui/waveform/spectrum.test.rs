use super::*;

fn sine_wave(freq: f32, sample_rate: u32, samples: usize, amplitude: f32) -> Vec<f32> {
    (0..samples)
        .map(|i| {
            (2. * std::f32::consts::PI * freq * i as f32 / sample_rate as f32).sin() * amplitude
        })
        .collect()
}

fn peak_bar(frame: &SpectrumFrame) -> (usize, f32) {
    frame
        .bars()
        .iter()
        .enumerate()
        .fold(
            (0, 0_f32),
            |(best_ix, best), (ix, &v)| {
                if v > best { (ix, v) } else { (best_ix, best) }
            },
        )
}

#[test]
fn analyzer_places_440hz_peak_in_its_band() {
    let mut analyzer = SpectrumAnalyzer::new();
    let frame = analyzer.analyze(&sine_wave(440., 48_000, 1024, 1.), 48_000);
    let (ix, height) = peak_bar(&frame);
    assert!(height > 0.8, "peak bar height {height}");
    let (lo, hi) = frame.bar_band(ix).unwrap();
    assert!(lo <= 440. && 440. <= hi, "band {lo}..{hi}");
}

#[test]
fn analyzer_reports_silence_as_zero_bars() {
    let mut analyzer = SpectrumAnalyzer::new();
    let frame = analyzer.analyze(&vec![0.; 1024], 48_000);
    assert!(frame.bars().iter().all(|&v| v == 0.));
}

#[test]
fn analyzer_scales_with_amplitude() {
    let mut analyzer = SpectrumAnalyzer::new();
    let loud = analyzer.analyze(&sine_wave(440., 48_000, 1024, 1.), 48_000);
    let quiet = analyzer.analyze(&sine_wave(440., 48_000, 1024, 0.5), 48_000);
    assert!(peak_bar(&quiet).1 < peak_bar(&loud).1);
}

#[test]
fn bar_bands_span_20hz_floor_to_nyquist() {
    let mut analyzer = SpectrumAnalyzer::new();
    let frame = analyzer.analyze(&sine_wave(440., 48_000, 1024, 1.), 48_000);
    approx_eq(frame.bar_band(0).unwrap().0, 48_000. / 1024.);
    approx_eq(frame.bar_band(SPECTRUM_BARS - 1).unwrap().1, 24_000.);
    assert!(frame.bar_band(SPECTRUM_BARS).is_none());
    assert_eq!(frame.fft_size(), 1024);
    assert_eq!(frame.sample_rate(), 48_000);
}

#[test]
fn analyzer_zero_pads_short_mixdown() {
    let mut analyzer = SpectrumAnalyzer::new();
    let frame = analyzer.analyze(&sine_wave(440., 48_000, 200, 1.), 48_000);
    let (ix, height) = peak_bar(&frame);
    assert!(height > 0.5, "peak bar height {height}");
    let (lo, hi) = frame.bar_band(ix).unwrap();
    assert!(lo <= 440. && 440. <= hi, "band {lo}..{hi}");
}

fn approx_eq(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-3, "expected ~{b}, got {a}");
}
