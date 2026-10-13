use super::{PEAK_COLUMNS, PeakColumn, WaveformPeaks};

fn expected_column(samples: &[f32], channels: usize, column: usize) -> PeakColumn {
    let frames = samples.len() / channels;
    let start = frames * column / PEAK_COLUMNS;
    let end = frames * (column + 1) / PEAK_COLUMNS;
    let mut min = f32::INFINITY;
    let mut max = f32::NEG_INFINITY;
    let mut sq_sum = 0f32;
    let mut count = 0usize;
    for frame in start..end {
        for channel in 0..channels {
            let sample = samples[frame * channels + channel];
            min = min.min(sample);
            max = max.max(sample);
            sq_sum += sample * sample;
            count += 1;
        }
    }
    if count == 0 {
        PeakColumn {
            min: 0.,
            max: 0.,
            rms: 0.,
        }
    } else {
        PeakColumn {
            min,
            max,
            rms: (sq_sum / count as f32).sqrt(),
        }
    }
}

#[test]
fn ramp_decimates_to_expected_columns() {
    let samples: Vec<f32> = (0..1000).map(|i| i as f32 / 1000.).collect();
    let peaks = WaveformPeaks::from_interleaved(&samples, 1);
    for column in [0, 1, 1024, 2047] {
        assert_eq!(
            peaks.peak(column).unwrap().min,
            expected_column(&samples, 1, column).min
        );
        assert_eq!(
            peaks.peak(column).unwrap().max,
            expected_column(&samples, 1, column).max
        );
        assert_eq!(
            peaks.peak(column).unwrap().rms,
            expected_column(&samples, 1, column).rms
        );
    }
    let last = peaks.peak(2047).unwrap();
    assert_eq!(last.min, 0.999);
    assert_eq!(last.max, 0.999);
}

#[test]
fn stereo_interleave_uses_both_channels() {
    let samples: Vec<f32> = (0..PEAK_COLUMNS).flat_map(|_| [-1., 1.]).collect();
    let peaks = WaveformPeaks::from_interleaved(&samples, 2);
    let column = peaks.peak(0).unwrap();
    assert_eq!(column.min, -1.);
    assert_eq!(column.max, 1.);
    assert!((column.rms - 1.).abs() < 1e-6);
}

#[test]
fn every_column_populated_when_frames_exceed_columns() {
    let samples: Vec<f32> = (0..PEAK_COLUMNS * 2).map(|i| i as f32).collect();
    let peaks = WaveformPeaks::from_interleaved(&samples, 1);
    for column in 0..PEAK_COLUMNS {
        let peak = peaks.peak(column).unwrap();
        assert!(peak.max >= peak.min);
        assert!(peak.rms >= peak.min && peak.rms <= peak.max);
    }
}

#[test]
fn empty_input_yields_all_zero_columns() {
    let peaks = WaveformPeaks::from_interleaved(&[], 2);
    for column in 0..PEAK_COLUMNS {
        assert_eq!(
            peaks.peak(column).unwrap(),
            PeakColumn {
                min: 0.,
                max: 0.,
                rms: 0.
            }
        );
    }
}

#[test]
fn peak_past_the_end_is_none() {
    let peaks = WaveformPeaks::from_interleaved(&[0.5; 8], 1);
    assert!(peaks.peak(PEAK_COLUMNS).is_none());
}
