use gpui_kit::{Bounds, Hsla, Pixels, Point, black, point, px, white};
use image::RgbaImage;

use super::{
    apply_filters, apply_filters_until, compose_export, fill_circle, hsla_to_rgba, rasterize_shape,
    stamp_line, stroke_ellipse,
};
use crate::features::pages::image_editor::document::FilterSettings;
use crate::ui::canvas::{CanvasShape, ShapeStyle, StrokeStyle};

fn solid(w: u32, h: u32, rgba: [u8; 4]) -> RgbaImage {
    RgbaImage::from_pixel(w, h, image::Rgba(rgba))
}

fn stroke(width: f32) -> StrokeStyle {
    StrokeStyle {
        width: px(width),
        color: white(),
        dash: None,
    }
}

#[test]
fn apply_filters_identity_returns_equal_pixels() {
    let base = solid(4, 4, [10, 20, 30, 255]);
    let out = apply_filters(&base, &FilterSettings::default());
    assert_eq!(out.as_raw(), base.as_raw());
}

#[test]
fn grayscale_makes_each_pixel_gray() {
    let mut base = solid(2, 1, [0, 0, 0, 255]);
    base.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
    base.put_pixel(1, 0, image::Rgba([0, 255, 0, 255]));
    let out = apply_filters(
        &base,
        &FilterSettings {
            grayscale: true,
            ..FilterSettings::default()
        },
    );
    for x in 0..2 {
        let p = out.get_pixel(x, 0);
        assert_eq!(p.0[0], p.0[1]);
        assert_eq!(p.0[1], p.0[2]);
    }
}

#[test]
fn brighten_raises_every_channel_with_saturation() {
    let base = solid(2, 1, [250, 100, 10, 255]);
    let out = apply_filters(
        &base,
        &FilterSettings {
            brightness: 10,
            ..FilterSettings::default()
        },
    );
    let p = out.get_pixel(0, 0);
    assert_eq!(p.0[0], 255);
    assert_eq!(p.0[1], 110);
    assert_eq!(p.0[2], 20);
    assert_eq!(p.0[3], 255);
}

#[test]
fn blur_zero_sigma_is_identity() {
    let base = solid(4, 4, [200, 100, 50, 255]);
    let out = apply_filters(
        &base,
        &FilterSettings {
            blur_sigma: 0.,
            ..FilterSettings::default()
        },
    );
    assert_eq!(out.as_raw(), base.as_raw());
}

#[test]
fn apply_filters_until_returns_none_when_superseded_immediately() {
    let base = solid(2, 1, [250, 100, 10, 255]);
    let out = apply_filters_until(
        &base,
        &FilterSettings {
            grayscale: true,
            ..FilterSettings::default()
        },
        &mut || true,
    );
    assert!(out.is_none());
}

#[test]
fn apply_filters_until_bails_between_stages() {
    let base = solid(2, 1, [250, 100, 10, 255]);
    let filters = FilterSettings {
        grayscale: true,
        brightness: 10,
        ..FilterSettings::default()
    };
    let mut checks = 0;
    let out = apply_filters_until(&base, &filters, &mut || {
        checks += 1;
        checks > 1
    });
    assert!(out.is_none());
    assert_eq!(checks, 2);
}

#[test]
fn apply_filters_until_with_never_superseded_matches_apply_filters() {
    let base = solid(2, 1, [250, 100, 10, 255]);
    let filters = FilterSettings {
        grayscale: true,
        brightness: 10,
        blur_sigma: 1.,
    };
    let expected = apply_filters(&base, &filters);
    let out = apply_filters_until(&base, &filters, &mut || false).unwrap();
    assert_eq!(out.as_raw(), expected.as_raw());
}

#[test]
fn blur_spreads_a_single_white_pixel() {
    let mut base = solid(9, 9, [0, 0, 0, 255]);
    base.put_pixel(4, 4, image::Rgba([255, 255, 255, 255]));
    let out = apply_filters(
        &base,
        &FilterSettings {
            blur_sigma: 1.,
            ..FilterSettings::default()
        },
    );
    assert!(out.get_pixel(3, 4).0[0] > 0);
    assert!(out.get_pixel(4, 5).0[0] > 0);
}

#[test]
fn fill_circle_sets_exactly_the_pixels_within_radius() {
    let mut buf = solid(9, 9, [0, 0, 0, 255]);
    fill_circle(&mut buf, (4., 4.), 2., [255, 255, 255, 255]);
    let mut painted = 0usize;
    for y in 0..9 {
        for x in 0..9 {
            let is_white = buf.get_pixel(x, y).0[0] == 255;
            let dx = x as f32 - 4.;
            let dy = y as f32 - 4.;
            assert_eq!(
                is_white,
                (dx * dx + dy * dy).sqrt() <= 2.,
                "pixel ({x},{y}) mismatch"
            );
            painted += is_white as usize;
        }
    }
    assert_eq!(painted, 13);
}

#[test]
fn stamp_line_covers_both_rows_along_the_segment() {
    let mut buf = solid(9, 3, [0, 0, 0, 255]);
    stamp_line(&mut buf, (0., 0.), (8., 0.), 1., [255, 255, 255, 255]);
    for x in 0..=8 {
        assert_eq!(buf.get_pixel(x, 0).0[0], 255, "x={x}");
        assert_eq!(buf.get_pixel(x, 1).0[0], 255, "x={x}");
    }
}

#[test]
fn stroke_ellipse_sets_extremes_and_leaves_center() {
    let mut buf = solid(9, 9, [0, 0, 0, 255]);
    stroke_ellipse(&mut buf, (4., 4.), 4., 4., 1., [255, 255, 255, 255]);
    for extreme in [(8u32, 4u32), (4, 8), (0, 4), (4, 0)] {
        assert_eq!(buf.get_pixel(extreme.0, extreme.1).0[0], 255);
    }
    assert_eq!(buf.get_pixel(4, 4).0[0], 0);
}

/// The semi-transparent case real annotations hit via hsla_to_rgba: each
/// channel is src*sa + dst*(1-sa), alpha is max, neighbors stay untouched.
#[test]
fn fill_circle_blends_semi_transparent_src_over_the_background() {
    let mut buf = solid(9, 9, [200, 100, 50, 40]);
    fill_circle(&mut buf, (4., 4.), 0., [255, 0, 0, 128]);
    let px = buf.get_pixel(4, 4).0;
    let sa: f32 = 128. / 255.;
    for (channel, src, dst) in [(0, 255., 200.), (1, 0., 100.), (2, 0., 50.)] {
        let expected = (src * sa + dst * (1. - sa)).round() as u8;
        assert_eq!(px[channel], expected, "channel {channel}");
    }
    assert_eq!(px[3], 128, "alpha is src.max(dst)");
    assert_eq!(
        buf.get_pixel(4, 3).0,
        [200, 100, 50, 40],
        "pixels outside the stamp keep the background"
    );
}

fn dot_at(p: Point<Pixels>, width: f32) -> CanvasShape {
    CanvasShape::path(vec![p], stroke(width))
}

#[test]
fn rasterized_single_point_path_paints_its_neighborhood() {
    let mut buf = solid(9, 9, [0, 0, 0, 255]);
    rasterize_shape(
        &mut buf,
        &dot_at(point(px(4.), px(4.)), 2.),
        [255, 255, 255, 255],
        2.,
    );
    for (x, y) in [(4u32, 4u32), (3, 4), (5, 4), (4, 3), (4, 5)] {
        assert_eq!(buf.get_pixel(x, y).0[0], 255);
    }
    assert_eq!(buf.get_pixel(3, 3).0[0], 0);
}

#[test]
fn compose_export_bakes_path_diagonal() {
    let base = solid(8, 8, [255, 255, 255, 255]);
    let annotations = [CanvasShape::path(
        vec![point(px(0.), px(0.)), point(px(7.), px(7.))],
        StrokeStyle {
            width: px(2.),
            color: black(),
            dash: None,
        },
    )];
    let out = compose_export(&base, &FilterSettings::default(), &annotations);
    assert_eq!(out.get_pixel(0, 0).0[0], 0);
    assert_eq!(out.get_pixel(7, 7).0[0], 0);
    assert_eq!(out.get_pixel(7, 0).0[0], 255);
}

#[test]
fn rasterize_rect_strokes_the_edges() {
    let mut buf = solid(8, 8, [0, 0, 0, 255]);
    let rect = CanvasShape::rect(
        Bounds::from_corners(point(px(2.), px(2.)), point(px(6.), px(6.))),
        ShapeStyle::stroked(stroke(2.)),
    );
    rasterize_shape(&mut buf, &rect, [255, 255, 255, 255], 2.);
    for x in 2..=6 {
        assert_eq!(buf.get_pixel(x, 2).0[0], 255, "top x={x}");
        assert_eq!(buf.get_pixel(x, 6).0[0], 255, "bottom x={x}");
    }
    for y in 2..=6 {
        assert_eq!(buf.get_pixel(2, y).0[0], 255, "left y={y}");
        assert_eq!(buf.get_pixel(6, y).0[0], 255, "right y={y}");
    }
    assert_eq!(buf.get_pixel(4, 4).0[0], 0, "interior untouched");
}

#[test]
fn rasterize_ellipse_strokes_the_ring() {
    let mut buf = solid(9, 9, [0, 0, 0, 255]);
    let ellipse = CanvasShape::ellipse(
        Bounds::from_corners(point(px(0.), px(0.)), point(px(8.), px(8.))),
        ShapeStyle::stroked(stroke(2.)),
    );
    rasterize_shape(&mut buf, &ellipse, [255, 255, 255, 255], 2.);
    assert_eq!(buf.get_pixel(4, 0).0[0], 255);
    assert_eq!(buf.get_pixel(4, 8).0[0], 255);
    assert_eq!(buf.get_pixel(4, 4).0[0], 0);
}

/// Apex at top-center, base corners at bottom-left/bottom-right: each edge
/// midpoint is painted while the interior and the space above the apex are not.
#[test]
fn rasterize_triangle_strokes_the_three_edges() {
    let mut buf = solid(17, 17, [0, 0, 0, 255]);
    let triangle = CanvasShape::triangle(
        Bounds::from_corners(point(px(2.), px(2.)), point(px(14.), px(14.))),
        ShapeStyle::stroked(stroke(2.)),
    );
    rasterize_shape(&mut buf, &triangle, [255, 255, 255, 255], 2.);
    for (x, y) in [(8u32, 2u32), (5, 8), (11, 8), (8, 14)] {
        assert_eq!(buf.get_pixel(x, y).0[0], 255, "edge point ({x},{y})");
    }
    assert_eq!(buf.get_pixel(8, 10).0[0], 0, "interior untouched");
    assert_eq!(buf.get_pixel(8, 0).0[0], 0, "above the apex untouched");
}

#[test]
fn hsla_to_rgba_converts_channels() {
    let rgba = hsla_to_rgba(Hsla {
        h: 0.,
        s: 1.,
        l: 0.5,
        a: 1.,
    });
    assert_eq!(rgba, [255, 0, 0, 255]);
}
