// No `use super::*`: gpui's `test` proc-macro glob would make bare `#[test]`
// recurse infinitely. Import only what we need.
use super::CanvasDocument;
use super::CanvasShape;
use super::ShapeStyle;
use super::StrokeStyle;
use gpui_kit::{Bounds, Pixels, black, point, px, size};

fn rect_shape() -> CanvasShape {
    CanvasShape::rect(
        Bounds {
            origin: point(px(10.), px(20.)),
            size: size(px(30.), px(40.)),
        },
        ShapeStyle::default(),
    )
}

fn stroke(width: Pixels) -> StrokeStyle {
    StrokeStyle {
        width,
        ..StrokeStyle::default()
    }
}

fn assert_px_eq(actual: Pixels, expected: Pixels) {
    assert!(
        (actual.as_f32() - expected.as_f32()).abs() < 0.01,
        "expected ~{expected:?}, got {actual:?}"
    );
}

#[test]
fn doc_bounds_returns_rect_ellipse_and_image_bounds_verbatim() {
    let bounds = Bounds {
        origin: point(px(10.), px(20.)),
        size: size(px(30.), px(40.)),
    };
    assert_eq!(
        CanvasShape::rect(bounds, ShapeStyle::default()).doc_bounds(),
        bounds
    );
    assert_eq!(
        CanvasShape::ellipse(bounds, ShapeStyle::default()).doc_bounds(),
        bounds
    );
}

#[test]
fn doc_bounds_expands_path_by_half_stroke_width() {
    let shape = CanvasShape::path(
        vec![point(px(0.), px(0.)), point(px(10.), px(0.))],
        stroke(px(2.)),
    );
    let bounds = shape.doc_bounds();
    assert_px_eq(bounds.origin.x, px(-1.));
    assert_px_eq(bounds.origin.y, px(-1.));
    assert_px_eq(bounds.size.width, px(12.));
    assert_px_eq(bounds.size.height, px(2.));
}

#[test]
fn doc_bounds_uses_the_documented_text_approximation() {
    let shape = CanvasShape::text(point(px(5.), px(7.)), "hello", px(10.), black());
    let bounds = shape.doc_bounds();
    assert_px_eq(bounds.origin.x, px(5.));
    assert_px_eq(bounds.origin.y, px(7.));
    assert_px_eq(bounds.size.width, px(30.));
    assert_px_eq(bounds.size.height, px(12.5));
}

#[test]
fn translate_moves_every_variant() {
    let delta = point(px(5.), px(-3.));
    let mut rect = rect_shape();
    rect.translate(delta);
    let CanvasShape::Rect { bounds, .. } = &rect else {
        panic!("variant preserved");
    };
    assert_px_eq(bounds.origin.x, px(15.));
    assert_px_eq(bounds.origin.y, px(17.));

    let mut ellipse = CanvasShape::ellipse(
        Bounds {
            origin: point(px(0.), px(0.)),
            size: size(px(10.), px(10.)),
        },
        ShapeStyle::default(),
    );
    ellipse.translate(delta);
    let CanvasShape::Ellipse { bounds, .. } = &ellipse else {
        panic!("variant preserved");
    };
    assert_px_eq(bounds.origin.y, px(-3.));

    let mut path = CanvasShape::path(
        vec![point(px(1.), px(2.)), point(px(3.), px(4.))],
        stroke(px(1.)),
    );
    path.translate(delta);
    let CanvasShape::Path { points, .. } = &path else {
        panic!("variant preserved");
    };
    assert_px_eq(points[1].x, px(8.));
    assert_px_eq(points[1].y, px(1.));

    let mut text = CanvasShape::text(point(px(0.), px(0.)), "x", px(12.), black());
    text.translate(delta);
    let CanvasShape::Text { position, .. } = &text else {
        panic!("variant preserved");
    };
    assert_px_eq(position.x, px(5.));
}

// Image is the one variant needing a RenderImage, whose only constructor
// takes image::Frame; `image` sits in the linux/windows Cargo tables only.
#[cfg(any(target_os = "windows", target_os = "linux"))]
#[test]
fn translate_and_doc_bounds_cover_the_image_variant() {
    use std::sync::Arc;

    use gpui_kit::RenderImage;

    let buffer = image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]));
    let mut shape = CanvasShape::image(
        Bounds {
            origin: point(px(0.), px(0.)),
            size: size(px(8.), px(8.)),
        },
        Arc::new(RenderImage::new([image::Frame::new(buffer)])),
    );
    assert_px_eq(shape.doc_bounds().size.width, px(8.));
    shape.translate(point(px(2.), px(2.)));
    let CanvasShape::Image { bounds, .. } = &shape else {
        panic!("variant preserved");
    };
    assert_px_eq(bounds.origin.x, px(2.));
}

#[test]
fn rect_hit_test_accepts_within_tolerance_and_rejects_beyond() {
    let shape = CanvasShape::rect(
        Bounds {
            origin: point(px(0.), px(0.)),
            size: size(px(100.), px(50.)),
        },
        ShapeStyle::default(),
    );
    let tolerance = px(4.);
    assert!(shape.hit_test(point(px(100.), px(25.)), tolerance));
    assert!(shape.hit_test(point(px(103.5), px(25.)), tolerance));
    assert!(!shape.hit_test(point(px(105.), px(25.)), tolerance));
}

#[test]
fn ellipse_hit_test_uses_the_expanded_radii_formula() {
    let shape = CanvasShape::ellipse(
        Bounds {
            origin: point(px(0.), px(0.)),
            size: size(px(100.), px(50.)),
        },
        ShapeStyle::default(),
    );
    let tolerance = px(4.);
    assert!(shape.hit_test(point(px(100.), px(25.)), tolerance));
    assert!(!shape.hit_test(point(px(106.), px(25.)), tolerance));
    assert!(shape.hit_test(point(px(50.), px(-3.)), tolerance));
    assert!(!shape.hit_test(point(px(50.), px(-6.)), tolerance));
}

#[test]
fn path_hit_test_matches_any_segment_within_tolerance() {
    let shape = CanvasShape::path(
        vec![point(px(0.), px(0.)), point(px(100.), px(0.))],
        stroke(px(1.)),
    );
    let tolerance = px(4.);
    assert!(shape.hit_test(point(px(50.), px(3.)), tolerance));
    assert!(!shape.hit_test(point(px(50.), px(5.)), tolerance));
}

#[test]
fn path_hit_test_rejects_inside_the_bounds_but_off_the_stroke() {
    let shape = CanvasShape::path(
        vec![
            point(px(0.), px(0.)),
            point(px(100.), px(0.)),
            point(px(100.), px(100.)),
        ],
        stroke(px(1.)),
    );
    assert!(!shape.hit_test(point(px(10.), px(90.)), px(4.)));
}

#[test]
fn single_point_path_hits_as_a_dot_of_the_stroke_width() {
    let shape = CanvasShape::path(vec![point(px(20.), px(20.))], stroke(px(6.)));
    let tolerance = px(4.);
    assert!(shape.hit_test(point(px(25.), px(20.)), tolerance));
    assert!(!shape.hit_test(point(px(29.), px(20.)), tolerance));
}

#[test]
fn text_hit_test_uses_the_approximate_bounds() {
    let shape = CanvasShape::text(point(px(0.), px(0.)), "hello", px(10.), black());
    assert!(shape.hit_test(point(px(29.), px(10.)), px(0.)));
    assert!(!shape.hit_test(point(px(31.), px(10.)), px(0.)));
}

#[test]
fn ids_allocate_increasing_and_never_reuse() {
    let mut doc = CanvasDocument::new();
    let layer_a = doc.add_layer();
    let layer_b = doc.add_layer();
    assert!(layer_b.0 > layer_a.0);
    let shape_a = doc.add_shape(layer_a, rect_shape()).unwrap();
    let shape_b = doc.add_shape(layer_b, rect_shape()).unwrap();
    assert!(shape_b.0 > shape_a.0);
    doc.remove_shape(shape_a);
    let shape_c = doc.add_shape(layer_a, rect_shape()).unwrap();
    assert!(shape_c.0 > shape_b.0);
    assert_ne!(shape_c, shape_a);
}

#[test]
fn add_shape_to_unknown_layer_returns_none() {
    let mut doc = CanvasDocument::new();
    assert!(doc.add_shape(super::LayerId(999), rect_shape()).is_none());
}

#[test]
fn shapes_iterates_bottom_to_top_including_hidden_layers() {
    let mut doc = CanvasDocument::new();
    let bottom = doc.add_layer();
    let top = doc.add_layer();
    let bottom_shape = doc.add_shape(bottom, rect_shape()).unwrap();
    let top_shape = doc.add_shape(top, rect_shape()).unwrap();
    doc.set_layer_visible(bottom, false);
    let order: Vec<_> = doc
        .shapes()
        .map(|(layer, shape, _)| (layer, shape))
        .collect();
    assert_eq!(order, vec![(bottom, bottom_shape), (top, top_shape)]);
}

#[test]
fn update_shape_on_missing_id_returns_none() {
    let mut doc = CanvasDocument::new();
    assert_eq!(
        doc.update_shape(super::ShapeId(4242), |_| "unreachable"),
        None
    );
}

#[test]
fn clear_empties_layers_but_keeps_ids_monotonic() {
    let mut doc = CanvasDocument::new();
    let layer = doc.add_layer();
    let shape = doc.add_shape(layer, rect_shape()).unwrap();
    doc.clear();
    assert!(doc.layers().is_empty());
    let next_layer = doc.add_layer();
    let next_shape = doc.add_shape(next_layer, rect_shape()).unwrap();
    assert!(next_layer.0 > layer.0);
    assert!(next_shape.0 > shape.0);
}
