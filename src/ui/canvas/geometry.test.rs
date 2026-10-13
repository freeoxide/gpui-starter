// No `use super::*`: gpui's `test` proc-macro glob would make bare `#[test]`
// recurse infinitely. Import only what we need.
use super::CanvasTransform;
use super::segment_distance;
use gpui_kit::{Bounds, Pixels, Point, point, px, size};

fn assert_px_eq(actual: Pixels, expected: Pixels) {
    assert!(
        (actual.as_f32() - expected.as_f32()).abs() < 0.01,
        "expected ~{expected:?}, got {actual:?}"
    );
}

fn assert_point_eq(actual: Point<Pixels>, expected: Point<Pixels>) {
    assert_px_eq(actual.x, expected.x);
    assert_px_eq(actual.y, expected.y);
}

#[test]
fn identity_roundtrips_sample_points() {
    for doc in [
        point(px(0.), px(0.)),
        point(px(10.), px(20.)),
        point(px(-31.5), px(400.25)),
    ] {
        assert_point_eq(
            CanvasTransform::IDENTITY.to_doc(CanvasTransform::IDENTITY.to_screen(doc)),
            doc,
        );
    }
}

#[test]
fn to_screen_scales_then_pans() {
    let transform = CanvasTransform::new(point(px(5.), px(7.)), 2.);
    assert_point_eq(
        transform.to_screen(point(px(10.), px(20.))),
        point(px(25.), px(47.)),
    );
}

#[test]
fn non_identity_roundtrips_points() {
    let transform = CanvasTransform::new(point(px(-40.), px(120.)), 0.25);
    for doc in [
        point(px(0.), px(0.)),
        point(px(100.), px(-3.)),
        point(px(1024.), px(768.)),
    ] {
        assert_point_eq(transform.to_doc(transform.to_screen(doc)), doc);
    }
}

#[test]
fn bounds_conversions_map_and_inverse() {
    let transform = CanvasTransform::new(point(px(11.), px(-9.)), 2.5);
    let doc = Bounds {
        origin: point(px(4.), px(6.)),
        size: size(px(20.), px(30.)),
    };
    let screen = transform.to_screen_bounds(doc);
    assert_point_eq(screen.origin, point(px(21.), px(6.)));
    assert_px_eq(screen.size.width, px(50.));
    assert_px_eq(screen.size.height, px(75.));
    let roundtrip = transform.to_doc_bounds(screen);
    assert_point_eq(roundtrip.origin, doc.origin);
    assert_px_eq(roundtrip.size.width, doc.size.width);
    assert_px_eq(roundtrip.size.height, doc.size.height);
}

#[test]
fn scale_length_multiplies_by_zoom() {
    let transform = CanvasTransform::new(Point::default(), 0.5);
    assert_px_eq(transform.scale_length(px(10.)), px(5.));
}

#[test]
fn segment_distance_hits_midpoint_exactly() {
    let a = point(px(0.), px(0.));
    let b = point(px(10.), px(0.));
    assert_px_eq(segment_distance(point(px(5.), px(3.)), a, b), px(3.));
}

#[test]
fn segment_distance_past_both_ends_measures_to_the_endpoint() {
    let a = point(px(0.), px(0.));
    let b = point(px(10.), px(0.));
    assert_px_eq(segment_distance(point(px(-3.), px(4.)), a, b), px(5.));
    assert_px_eq(segment_distance(point(px(13.), px(4.)), a, b), px(5.));
}

#[test]
fn segment_distance_on_degenerate_segment_measures_to_the_point() {
    let a = point(px(2.), px(2.));
    assert_px_eq(segment_distance(point(px(5.), px(6.)), a, a), px(5.));
}
