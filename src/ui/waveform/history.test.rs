use super::*;

fn approx(a: f32, b: f32) {
    assert!((a - b).abs() < 1e-4, "expected ~{b}, got {a}");
}

fn push_ramp(history: &mut ChannelHistory, up_to: usize) {
    let from = history.total_pushed as usize;
    for i in from..up_to {
        history.push(i as f32);
    }
}

fn sum_of_squares(n: usize) -> f32 {
    (0..n).map(|i| (i * i) as f32).sum()
}

#[test]
fn pyramid_append_feeds_level_0_in_strides() {
    let mut history = ChannelHistory::new(RAW_RING_SAMPLES);
    push_ramp(&mut history, 28);
    assert_eq!(history.raw.len(), 28);
    assert_eq!(history.levels.len(), 4);
    assert_eq!(history.levels[0].entries.len(), 3);
    assert_eq!(history.acc.count, 4);
    let entry = history.levels[0].entries[0];
    approx(entry.min, 0.);
    approx(entry.max, 7.);
    approx(entry.rms, (140_f32 / 8.).sqrt());
}

#[test]
fn pyramid_cascades_into_level_1_at_factor_64() {
    let mut history = ChannelHistory::new(RAW_RING_SAMPLES);
    push_ramp(&mut history, 64);
    assert_eq!(history.levels[1].factor, 64);
    assert_eq!(history.levels[1].entries.len(), 1);
    let entry = history.levels[1].entries[0];
    approx(entry.min, 0.);
    approx(entry.max, 63.);
    approx(entry.rms, (sum_of_squares(64) / 64.).sqrt());
}

#[test]
fn eviction_caps_raw_ring_and_level_entries() {
    let mut history = ChannelHistory::new(64);
    push_ramp(&mut history, RAW_RING_SAMPLES + 200);
    assert_eq!(history.raw.len(), RAW_RING_SAMPLES);
    approx(*history.raw.front().unwrap(), 200.);
    approx(
        *history.raw.back().unwrap(),
        (RAW_RING_SAMPLES + 199) as f32,
    );
    assert!(history.levels[0].entries.len() <= 8);
    assert_eq!(history.levels[0].cap, 8);
    assert_eq!(history.levels[1].entries.len(), 1);
    assert_eq!(history.levels[1].cap, 1);
}

#[test]
fn provisional_tracks_the_incomplete_group() {
    let mut history = ChannelHistory::new(RAW_RING_SAMPLES);
    push_ramp(&mut history, 28);
    let p = history.provisional(0).unwrap();
    approx(p.min, 24.);
    approx(p.max, 27.);
    approx(p.rms, ((576. + 625. + 676. + 729.) / 4_f32).sqrt());
    push_ramp(&mut history, 32);
    assert!(history.provisional(0).is_none());
}

#[test]
fn query_raw_path_maps_newest_suffix_to_columns() {
    let mut history = ChannelHistory::new(RAW_RING_SAMPLES);
    push_ramp(&mut history, 100);
    let cols = history.query(100, 10);
    assert!(cols.iter().all(|c| c.populated));
    approx(cols[0].min, 0.);
    approx(cols[0].max, 9.);
    approx(cols[9].min, 90.);
    approx(cols[9].max, 99.);
}

#[test]
fn query_raw_path_marks_empty_ranges_unpopulated() {
    let mut history = ChannelHistory::new(RAW_RING_SAMPLES);
    push_ramp(&mut history, 100);
    let cols = history.query(100, 200);
    for (c, col) in cols.iter().enumerate() {
        assert_eq!(col.populated, c % 2 == 1, "column {c}");
    }
}

#[test]
fn query_level_path_appends_provisional_entry() {
    let mut history = ChannelHistory::new(RAW_RING_SAMPLES + 800);
    assert_eq!(history.levels.len(), 4);
    push_ramp(&mut history, 17_184);
    let cols = history.query(17_184, 10);
    assert!(cols.iter().all(|c| c.populated));
    approx(cols[0].min, 0.);
    approx(cols[0].max, 4095.);
    approx(cols[9].min, 16384.);
    approx(cols[9].max, 17183.);
}

#[test]
fn query_level0_boundary_includes_provisional() {
    let mut history = ChannelHistory::new(RAW_RING_SAMPLES + 8);
    push_ramp(&mut history, RAW_RING_SAMPLES + 4);
    let cols = history.query(RAW_RING_SAMPLES + 4, 2049);
    assert_eq!(cols.len(), 2049);
    approx(cols[0].max, 7.);
    approx(cols[2047].max, 16383.);
    approx(cols[2048].max, 16387.);
}

#[test]
fn last_samples_copies_newest_slice() {
    let mut history = ChannelHistory::new(RAW_RING_SAMPLES);
    push_ramp(&mut history, 100);
    let mut out = Vec::new();
    history.last_samples(8, &mut out);
    assert_eq!(out, (92..100).map(|i| i as f32).collect::<Vec<_>>());
    history.last_samples(1000, &mut out);
    assert_eq!(out, (0..100).map(|i| i as f32).collect::<Vec<_>>());
}
