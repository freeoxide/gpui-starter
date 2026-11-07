use super::{PEAK_COLUMNS, PeakColumn, WaveformPeaks};

#[test]
fn ramp_columns_track_their_own_frame() {
    // One frame per column: column i must hold exactly sample i.
    let samples: Vec<f32> = (0..PEAK_COLUMNS)
        .map(|i| i as f32 / PEAK_COLUMNS as f32)
        .collect();
    let peaks = WaveformPeaks::from_interleaved(&samples, 1);
    for column in [0, 1, 1024, 2047] {
        let expected = column as f32 / PEAK_COLUMNS as f32;
        let peak = peaks.peak(column).unwrap();
        assert_eq!(peak.min, expected);
        assert_eq!(peak.max, expected);
        assert!((peak.rms - expected).abs() < 1e-6);
    }
}

#[test]
fn ramp_max_increases_and_ends_on_the_global_max() {
    // Two frames per column: no empty columns, so column maxes climb.
    let samples: Vec<f32> = (0..PEAK_COLUMNS * 2)
        .map(|i| i as f32 / (PEAK_COLUMNS * 2) as f32)
        .collect();
    let peaks = WaveformPeaks::from_interleaved(&samples, 1);
    let mut previous = f32::NEG_INFINITY;
    for column in 0..PEAK_COLUMNS {
        let max = peaks.peak(column).unwrap().max;
        assert!(max > previous, "column {column} max must strictly increase");
        previous = max;
    }
    let last = peaks.peak(2047).unwrap();
    assert_eq!(Some(last.max), samples.last().copied());
}

#[test]
fn sparse_ramp_keeps_the_global_max_in_the_last_column() {
    // Fewer frames than columns leaves empty columns; the tail bucket must
    // still cover the final (loudest) frame.
    let samples: Vec<f32> = (0..1000).map(|i| i as f32 / 1000.).collect();
    let peaks = WaveformPeaks::from_interleaved(&samples, 1);
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
