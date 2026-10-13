use gpui_kit::{Bounds, point, px, size};

use super::{
    MIN_SHAPE_SIZE, handle_positions, rect_from_corners, resize_bounds, sample_cubic_bezier,
    scale_points, square_at,
};

#[test]
fn handle_positions_list_all_eight_anchors() {
    let bounds = Bounds {
        origin: point(px(10.), px(20.)),
        size: size(px(100.), px(50.)),
    };
    let positions = handle_positions(bounds);
    let expected = [
        point(px(10.), px(20.)),
        point(px(60.), px(20.)),
        point(px(110.), px(20.)),
        point(px(110.), px(45.)),
        point(px(110.), px(70.)),
        point(px(60.), px(70.)),
        point(px(10.), px(70.)),
        point(px(10.), px(45.)),
    ];
    assert_eq!(positions, expected);
}

#[test]
fn square_at_centers_on_the_input_point() {
    let bounds = square_at(point(px(10.), px(20.)), px(8.));
    assert_eq!(bounds.origin, point(px(6.), px(16.)));
    assert_eq!(bounds.size, size(px(8.), px(8.)));
}

#[test]
fn resize_bounds_moves_only_the_anchors_edges() {
    let start = Bounds {
        origin: point(px(10.), px(20.)),
        size: size(px(100.), px(50.)),
    };
    let tl = resize_bounds(start, 0, point(px(30.), px(40.)));
    assert_eq!(tl.origin, point(px(30.), px(40.)));
    assert_eq!(
        point(tl.origin.x + tl.size.width, tl.origin.y + tl.size.height),
        point(px(110.), px(70.)),
        "TL resize keeps the bottom-right corner fixed"
    );
    let br = resize_bounds(start, 4, point(px(80.), px(50.)));
    assert_eq!(br.origin, point(px(10.), px(20.)));
    assert_eq!(br.size, size(px(70.), px(30.)));
    let mr = resize_bounds(start, 3, point(px(40.), px(0.)));
    assert_eq!(mr.origin, point(px(10.), px(20.)));
    assert_eq!(mr.size, size(px(30.), px(50.)));
}

#[test]
fn resize_bounds_clamps_to_minimum_size() {
    let start = Bounds {
        origin: point(px(10.), px(20.)),
        size: size(px(100.), px(50.)),
    };
    let tl = resize_bounds(start, 0, point(px(200.), px(100.)));
    assert_eq!(tl.origin, point(px(102.), px(62.)));
    assert_eq!(tl.size, size(px(8.), px(8.)));
}

#[test]
fn resize_bounds_never_flips_across_the_anchored_edge() {
    let start = Bounds {
        origin: point(px(10.), px(20.)),
        size: size(px(100.), px(50.)),
    };
    let mr = resize_bounds(start, 3, point(px(0.), px(45.)));
    assert_eq!(mr.origin, point(px(10.), px(20.)));
    assert_eq!(mr.size.width, MIN_SHAPE_SIZE);
}

#[test]
fn scale_points_maps_corners_and_midpoint() {
    let old = Bounds {
        origin: point(px(0.), px(0.)),
        size: size(px(100.), px(100.)),
    };
    let new = Bounds {
        origin: point(px(10.), px(10.)),
        size: size(px(200.), px(100.)),
    };
    let points = vec![
        point(px(0.), px(0.)),
        point(px(100.), px(0.)),
        point(px(100.), px(100.)),
        point(px(0.), px(100.)),
        point(px(50.), px(50.)),
    ];
    let scaled = scale_points(&points, old, new);
    let expected = vec![
        point(px(10.), px(10.)),
        point(px(210.), px(10.)),
        point(px(210.), px(110.)),
        point(px(10.), px(110.)),
        point(px(110.), px(60.)),
    ];
    assert_eq!(scaled, expected);
}

#[test]
fn scale_points_translates_when_the_old_bounds_are_flat() {
    let old = Bounds {
        origin: point(px(5.), px(5.)),
        size: size(px(0.), px(40.)),
    };
    let new = Bounds {
        origin: point(px(10.), px(10.)),
        size: size(px(100.), px(100.)),
    };
    let scaled = scale_points(&[point(px(5.), px(5.))], old, new);
    assert_eq!(scaled, vec![point(px(10.), px(10.))]);
}

#[test]
fn rect_from_corners_normalizes_reversed_drag_corners() {
    let bounds = rect_from_corners(point(px(30.), px(10.)), point(px(10.), px(40.)));
    assert_eq!(bounds.origin, point(px(10.), px(10.)));
    assert_eq!(bounds.size, size(px(20.), px(30.)));
}

#[test]
fn cubic_sampling_hits_both_endpoints_exactly() {
    let p0 = point(px(120.), px(260.));
    let c1 = point(px(200.), px(80.));
    let c2 = point(px(360.), px(440.));
    let p1 = point(px(460.), px(260.));
    let samples = sample_cubic_bezier(p0, c1, c2, p1, 24);
    assert_eq!(samples.len(), 25);
    assert_eq!(samples.first().copied(), Some(p0));
    assert_eq!(samples.last().copied(), Some(p1));
}

#[test]
fn cubic_sampling_matches_the_curve_formula_at_the_midpoint() {
    let p0 = point(px(0.), px(0.));
    let c1 = point(px(100.), px(0.));
    let c2 = point(px(100.), px(100.));
    let p1 = point(px(0.), px(100.));
    let samples = sample_cubic_bezier(p0, c1, c2, p1, 4);
    let mid = samples[2];
    assert!((mid.x.as_f32() - 75.).abs() < 0.01, "got {}", mid.x);
    assert!((mid.y.as_f32() - 50.).abs() < 0.01, "got {}", mid.y);
}

#[test]
fn cubic_sampling_stays_linear_for_collinear_controls() {
    let p0 = point(px(0.), px(0.));
    let c1 = point(px(100.), px(0.));
    let c2 = point(px(200.), px(0.));
    let p1 = point(px(300.), px(0.));
    let samples = sample_cubic_bezier(p0, c1, c2, p1, 4);
    let x_at_quarter = samples[1].x.as_f32();
    assert!((x_at_quarter - 75.).abs() < 0.01, "got {x_at_quarter}");
    for sample in &samples {
        assert_eq!(sample.y, px(0.));
    }
}

#[test]
fn cubic_sampling_with_one_segment_yields_both_endpoints() {
    let samples = sample_cubic_bezier(
        point(px(0.), px(0.)),
        point(px(10.), px(10.)),
        point(px(20.), px(0.)),
        point(px(30.), px(10.)),
        1,
    );
    assert_eq!(samples.len(), 2);
}
