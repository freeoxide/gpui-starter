// No `use super::*`: gpui's `test` proc-macro glob would make bare `#[test]`
// recurse infinitely. Import only what we need.
use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::{Path, PathBuilder, Pixels, Point, black, point, px};

use super::ShapeCache;
use crate::ui::canvas::{CanvasShape, ShapeStyle, StrokeStyle};

fn diagonal(width: Pixels) -> Option<Path<Pixels>> {
    let mut builder = PathBuilder::stroke(width);
    builder.move_to(point(px(0.), px(0.)));
    builder.line_to(point(px(10.), px(10.)));
    builder.build().ok()
}

fn assert_px_eq(actual: Pixels, expected: Pixels) {
    assert!(
        (actual.as_f32() - expected.as_f32()).abs() < 0.01,
        "expected ~{expected:?}, got {actual:?}"
    );
}

#[test]
fn path_builds_once_per_generation_and_not_on_origin_change() {
    let mut cache = ShapeCache::default();
    let builds = Rc::new(Cell::new(0));
    let hashes = Rc::new(Cell::new(0));
    let (build, key) = {
        let (builds, hashes) = (builds.clone(), hashes.clone());
        let content = CanvasShape::rect(
            gpui_kit::Bounds {
                origin: point(px(0.), px(0.)),
                size: gpui_kit::size(px(10.), px(10.)),
            },
            ShapeStyle::default(),
        );
        let zoom = 1.;
        (
            move || {
                builds.set(builds.get() + 1);
                diagonal(px(2.))
            },
            move || {
                hashes.set(hashes.get() + 1);
                super::shape_key(&content, zoom)
            },
        )
    };
    assert!(
        cache
            .get_path(1, &key, point(px(5.), px(5.)), &build)
            .is_some()
    );
    cache
        .get_path(1, &key, point(px(50.), px(50.)), &build)
        .unwrap();
    cache.get_path(1, &key, Point::default(), &build).unwrap();
    assert_eq!(builds.get(), 1);
    assert_eq!(hashes.get(), 1, "same generation never rehashes");

    // New generation, unchanged content: revalidated without rebuilding.
    cache.get_path(2, &key, Point::default(), &build).unwrap();
    assert_eq!(builds.get(), 1);
    assert_eq!(hashes.get(), 2);
}

#[test]
fn path_rebuilds_when_generation_and_content_both_change() {
    let mut cache = ShapeCache::default();
    let content = CanvasShape::rect(
        gpui_kit::Bounds {
            origin: point(px(0.), px(0.)),
            size: gpui_kit::size(px(10.), px(10.)),
        },
        ShapeStyle::default(),
    );
    let moved = CanvasShape::rect(
        gpui_kit::Bounds {
            origin: point(px(4.), px(0.)),
            size: gpui_kit::size(px(10.), px(10.)),
        },
        ShapeStyle::default(),
    );
    let builds = Rc::new(Cell::new(0));
    let build = {
        let builds = builds.clone();
        move || {
            builds.set(builds.get() + 1);
            diagonal(px(2.))
        }
    };
    cache
        .get_path(
            1,
            || super::shape_key(&content, 1.),
            Point::default(),
            &build,
        )
        .unwrap();
    cache
        .get_path(2, || super::shape_key(&moved, 1.), Point::default(), &build)
        .unwrap();
    assert_eq!(builds.get(), 2);
}

#[test]
fn returned_vertices_differ_by_exactly_the_origin_delta() {
    let mut cache = ShapeCache::default();
    let content = CanvasShape::rect(
        gpui_kit::Bounds {
            origin: point(px(0.), px(0.)),
            size: gpui_kit::size(px(10.), px(10.)),
        },
        ShapeStyle::default(),
    );
    let stored = cache
        .get_path(
            7,
            || super::shape_key(&content, 1.),
            Point::default(),
            || diagonal(px(2.)),
        )
        .unwrap();
    let moved = cache
        .get_path(
            7,
            || super::shape_key(&content, 1.),
            point(px(10.), px(4.)),
            || diagonal(px(2.)),
        )
        .unwrap();
    assert_eq!(stored.vertices.len(), moved.vertices.len());
    for (stored_vertex, moved_vertex) in stored.vertices.iter().zip(&moved.vertices) {
        assert_px_eq(
            moved_vertex.xy_position.x - stored_vertex.xy_position.x,
            px(10.),
        );
        assert_px_eq(
            moved_vertex.xy_position.y - stored_vertex.xy_position.y,
            px(4.),
        );
    }
}

#[test]
fn text_key_changes_with_content_and_color() {
    let base = CanvasShape::text(point(px(1.), px(2.)), "alpha", px(12.), black());
    let base_key = super::shape_key(&base, 1.);
    assert_ne!(super::shape_key(&base, 2.), base_key);
    assert_ne!(
        super::shape_key(
            &CanvasShape::text(point(px(1.), px(2.)), "beta", px(12.), black()),
            1.
        ),
        base_key
    );
    assert_ne!(
        super::shape_key(
            &CanvasShape::text(point(px(1.), px(2.)), "alpha", px(12.), gpui_kit::white()),
            1.
        ),
        base_key
    );
    assert_eq!(
        super::shape_key(
            &CanvasShape::text(point(px(1.), px(2.)), "alpha", px(12.), black()),
            1.
        ),
        base_key
    );
}

#[test]
fn shape_key_changes_with_geometry_style_and_zoom() {
    let stroke = StrokeStyle {
        width: px(2.),
        color: black(),
        dash: None,
    };
    let base = CanvasShape::path(
        vec![point(px(0.), px(0.)), point(px(10.), px(0.))],
        stroke.clone(),
    );
    let base_key = super::shape_key(&base, 1.);
    assert_ne!(super::shape_key(&base, 2.), base_key);
    let moved_point =
        CanvasShape::path(vec![point(px(0.), px(0.)), point(px(10.), px(1.))], stroke);
    assert_ne!(super::shape_key(&moved_point, 1.), base_key);
    let wider_stroke = CanvasShape::path(
        vec![point(px(0.), px(0.)), point(px(10.), px(0.))],
        StrokeStyle {
            width: px(4.),
            color: black(),
            dash: None,
        },
    );
    assert_ne!(super::shape_key(&wider_stroke, 1.), base_key);
    let dashed = CanvasShape::path(
        vec![point(px(0.), px(0.)), point(px(10.), px(0.))],
        StrokeStyle {
            width: px(2.),
            color: black(),
            dash: Some(vec![px(2.), px(2.)]),
        },
    );
    assert_ne!(super::shape_key(&dashed, 1.), base_key);
}

#[test]
fn shape_key_is_stable_for_identical_input() {
    let shape = CanvasShape::rect(
        gpui_kit::Bounds {
            origin: point(px(1.), px(2.)),
            size: gpui_kit::size(px(30.), px(40.)),
        },
        ShapeStyle::default(),
    );
    assert_eq!(super::shape_key(&shape, 1.), super::shape_key(&shape, 1.));
}

#[test]
fn slot_stays_pointer_sized_for_the_shaped_line() {
    // An inline ShapedLine (SmallVec<[DecorationRun; 32]> storage) measures
    // ~3 KB, which at 100k+ slots is OOM class; it must stay boxed.
    assert!(std::mem::size_of::<super::ShapeCache>() <= 256);
}
