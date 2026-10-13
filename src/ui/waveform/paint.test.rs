// No `use super::*`: gpui's `test` proc-macro glob would make bare `#[test]`
// recurse infinitely. Import only what we need.
use gpui_kit::{Bounds, Pixels, point, px, size};

use super::ColumnAgg;
use super::{
    amplitude_y, bar_bounds, envelope_bounds, nice_time_step, oscilloscope_line_points,
    snap_to_device, time_gridline_xs,
};

fn lane() -> Bounds<Pixels> {
    Bounds {
        origin: point(px(0.), px(0.)),
        size: size(px(100.), px(100.)),
    }
}

fn agg(min: f32, max: f32, rms: f32) -> ColumnAgg {
    ColumnAgg {
        min,
        max,
        rms,
        populated: true,
    }
}

fn unpopulated() -> ColumnAgg {
    ColumnAgg {
        min: 0.,
        max: 0.,
        rms: 0.,
        populated: false,
    }
}

fn assert_px_eq(actual: Pixels, expected: Pixels) {
    assert!(
        (actual.as_f32() - expected.as_f32()).abs() < 1e-3,
        "expected ~{expected:?}, got {actual:?}"
    );
}

#[test]
fn snap_to_device_rounds_to_device_grid() {
    assert_eq!(snap_to_device(10.25, 2.0), 10.5);
    assert_eq!(snap_to_device(10.24, 2.0), 10.0);
    assert_eq!(snap_to_device(10.3, 1.0), 10.0);
}

#[test]
fn nice_time_step_picks_1_2_5_progression() {
    assert_eq!(nice_time_step(5., 6), 1.);
    let step = nice_time_step(0.05, 6);
    assert!((step - 0.01).abs() < 1e-6, "got {step}");
    assert_eq!(nice_time_step(0.4, 6), 0.1);
    assert_eq!(nice_time_step(120., 6), 20.);
}

#[test]
fn gridlines_march_left_from_right_at_step_spacing() {
    let bounds = Bounds {
        origin: point(px(0.), px(0.)),
        size: size(px(100.), px(50.)),
    };
    let xs = time_gridline_xs(bounds, 5., 1., 1.0);
    assert_eq!(xs.len(), 4);
    assert!(xs.windows(2).all(|w| w[0] > w[1]), "descending: {xs:?}");
    assert!(
        xs.iter()
            .all(|&x| x.as_f32() > bounds.left().as_f32() && x.as_f32() < bounds.right().as_f32())
    );
    let spacing = 1. / 5. * 100.;
    for w in xs.windows(2) {
        assert!((w[0] - w[1]).as_f32() - spacing < 1.0);
    }
}

#[test]
fn envelope_bounds_place_peak_and_rms_quads() {
    let (peak, rms) = envelope_bounds(&agg(-0.5, 0.5, 0.25), 2, 10, lane(), 1.0).unwrap();
    assert_px_eq(peak.origin.x, px(20.));
    assert_px_eq(peak.origin.y, px(25.));
    assert_px_eq(peak.size.width, px(10.));
    assert_px_eq(peak.size.height, px(50.));
    assert_px_eq(rms.origin.y, px(37.5));
    assert_px_eq(rms.size.height, px(25.));
    assert_px_eq(rms.origin.x, px(20.));

    let (peak2, rms2) = envelope_bounds(&agg(-0.5, 0.5, 0.25), 2, 10, lane(), 2.0).unwrap();
    assert_px_eq(peak2.origin.x, px(20.));
    assert_px_eq(peak2.origin.y, px(25.));
    assert_px_eq(rms2.origin.y, px(37.5));
    assert_px_eq(rms2.size.height, px(25.));
    assert!(envelope_bounds(&unpopulated(), 2, 10, lane(), 1.0).is_none());
}

#[test]
fn flat_aggs_floor_at_one_device_pixel() {
    let (peak, rms) = envelope_bounds(&agg(0.2, 0.2, 0.2), 0, 10, lane(), 1.0).unwrap();
    assert_px_eq(peak.size.height, px(1.));
    assert_px_eq(peak.origin.y, amplitude_y(0.2, lane()));
    assert_px_eq(rms.size.height, px(20.));

    let (peak2, _) = envelope_bounds(&agg(0.2, 0.2, 0.2), 0, 10, lane(), 2.0).unwrap();
    assert_px_eq(peak2.size.height, px(0.5));
}

#[test]
fn line_points_right_align_partial_windows() {
    let samples = [0.5f32, -0.5];
    let full = oscilloscope_line_points(&samples, 2, lane(), 1.0);
    assert_px_eq(full[0].x, px(0.));
    assert_px_eq(full[1].x, px(50.));
    assert_px_eq(full[0].y, amplitude_y(0.5, lane()));
    assert_px_eq(full[1].y, amplitude_y(-0.5, lane()));

    let inset = oscilloscope_line_points(&samples, 10, lane(), 1.0);
    assert_px_eq(inset[0].x, px(80.));
    assert_px_eq(inset[1].x, px(90.));

    let single = oscilloscope_line_points(&[0.25], 5, lane(), 1.0);
    assert_eq!(single.len(), 1);
    assert_px_eq(single[0].x, px(80.));
    assert!(oscilloscope_line_points(&[], 5, lane(), 1.0).is_empty());
}

#[test]
fn bar_bounds_tile_with_one_device_pixel_gaps() {
    let bars: Vec<_> = (0..4)
        .map(|ix| bar_bounds(1.0, ix, 4, lane(), 1.0))
        .collect();
    for (ix, bar) in bars.iter().enumerate() {
        assert_px_eq(bar.origin.x, px(25. * ix as f32 + 1.));
        assert_px_eq(bar.size.width, px(23.));
        assert_px_eq(bar.origin.y, px(0.));
        assert_px_eq(bar.size.height, px(100.));
    }
    for w in bars.windows(2) {
        let gap = (w[1].origin.x - (w[0].origin.x + w[0].size.width)).as_f32();
        assert!((gap - 2.0).abs() < 1e-3, "gap {gap}");
    }

    let half = bar_bounds(0.5, 0, 4, lane(), 1.0);
    assert_px_eq(half.size.height, px(50.));
    assert_px_eq(half.origin.y, px(50.));
}
