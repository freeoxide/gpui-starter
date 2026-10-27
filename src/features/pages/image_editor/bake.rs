use std::f32::consts::TAU;

use gpui_kit::{Hsla, Pixels, Point, Rgba};
use image::{DynamicImage, RgbaImage};

use super::document::FilterSettings;
use crate::ui::canvas::CanvasShape;

pub(crate) fn apply_filters(base: &RgbaImage, filters: &FilterSettings) -> RgbaImage {
    let mut img = DynamicImage::ImageRgba8(base.clone());
    if filters.grayscale {
        img = img.grayscale();
    }
    if filters.brightness != 0 {
        img = img.brighten(filters.brightness);
    }
    if filters.blur_sigma > 0. {
        img = img.blur(filters.blur_sigma);
    }
    img.to_rgba8()
}

fn blend_pixel(buf: &mut RgbaImage, x: u32, y: u32, src: [u8; 4]) {
    let mut px = *buf.get_pixel(x, y);
    let sa = src[3] as f32 / 255.;
    for (dst, channel) in px.0.iter_mut().zip(src).take(3) {
        let blended = channel as f32 * sa + *dst as f32 * (1. - sa);
        *dst = blended.round() as u8;
    }
    px.0[3] = src[3].max(px.0[3]);
    buf.put_pixel(x, y, px);
}

pub(crate) fn fill_circle(buf: &mut RgbaImage, center: (f32, f32), radius: f32, rgba: [u8; 4]) {
    let (w, h) = buf.dimensions();
    let x0 = (center.0 - radius).floor().max(0.) as i32;
    let x1 = (center.0 + radius).ceil().min(w as f32 - 1.) as i32;
    let y0 = (center.1 - radius).floor().max(0.) as i32;
    let y1 = (center.1 + radius).ceil().min(h as f32 - 1.) as i32;
    for y in y0.max(0)..=y1 {
        for x in x0.max(0)..=x1 {
            let dx = x as f32 - center.0;
            let dy = y as f32 - center.1;
            if (dx * dx + dy * dy).sqrt() <= radius {
                blend_pixel(buf, x as u32, y as u32, rgba);
            }
        }
    }
}

pub(crate) fn stamp_line(
    buf: &mut RgbaImage,
    a: (f32, f32),
    b: (f32, f32),
    radius: f32,
    rgba: [u8; 4],
) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = (dx * dx + dy * dy).sqrt();
    let steps = ((len / (radius.max(1.) / 2.)).ceil() as usize).max(1);
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        fill_circle(buf, (a.0 + dx * t, a.1 + dy * t), radius, rgba);
    }
}

pub(crate) fn stroke_ellipse(
    buf: &mut RgbaImage,
    center: (f32, f32),
    rx: f32,
    ry: f32,
    radius: f32,
    rgba: [u8; 4],
) {
    let samples = ((TAU * rx.max(ry)).ceil() as usize).max(12);
    for i in 0..samples {
        let theta = TAU * i as f32 / samples as f32;
        fill_circle(
            buf,
            (center.0 + rx * theta.cos(), center.1 + ry * theta.sin()),
            radius,
            rgba,
        );
    }
}

fn hsla_to_rgba(color: Hsla) -> [u8; 4] {
    let rgba = Rgba::from(color);
    [
        (rgba.r * 255.).round() as u8,
        (rgba.g * 255.).round() as u8,
        (rgba.b * 255.).round() as u8,
        (rgba.a * 255.).round() as u8,
    ]
}

fn path_points(shape: &CanvasShape) -> Option<(&[Point<Pixels>], [u8; 4], f32)> {
    match shape {
        CanvasShape::Path { points, style } => Some((
            points,
            hsla_to_rgba(style.color),
            style.width.as_f32().max(1.),
        )),
        _ => None,
    }
}

pub(crate) fn rasterize_shape(
    buf: &mut RgbaImage,
    shape: &CanvasShape,
    fallback_rgba: [u8; 4],
    fallback_width: f32,
) {
    match shape {
        CanvasShape::Path { .. } => {
            let Some((points, rgba, width)) = path_points(shape) else {
                return;
            };
            match points {
                [p] => fill_circle(buf, (p.x.as_f32(), p.y.as_f32()), width / 2., rgba),
                _ => {
                    for pair in points.windows(2) {
                        stamp_line(
                            buf,
                            (pair[0].x.as_f32(), pair[0].y.as_f32()),
                            (pair[1].x.as_f32(), pair[1].y.as_f32()),
                            width / 2.,
                            rgba,
                        );
                    }
                }
            }
        }
        CanvasShape::Rect { bounds, style } => {
            let (rgba, width) = match style.stroke() {
                Some(stroke) => (hsla_to_rgba(stroke.color), stroke.width.as_f32().max(1.)),
                None => (fallback_rgba, fallback_width),
            };
            let (x0, y0) = (bounds.origin.x.as_f32(), bounds.origin.y.as_f32());
            let (x1, y1) = (
                x0 + bounds.size.width.as_f32(),
                y0 + bounds.size.height.as_f32(),
            );
            for (a, b) in [
                ((x0, y0), (x1, y0)),
                ((x1, y0), (x1, y1)),
                ((x1, y1), (x0, y1)),
                ((x0, y1), (x0, y0)),
            ] {
                stamp_line(buf, a, b, width / 2., rgba);
            }
        }
        CanvasShape::Ellipse { bounds, style } => {
            let (rgba, width) = match style.stroke() {
                Some(stroke) => (hsla_to_rgba(stroke.color), stroke.width.as_f32().max(1.)),
                None => (fallback_rgba, fallback_width),
            };
            stroke_ellipse(
                buf,
                (
                    bounds.origin.x.as_f32() + bounds.size.width.as_f32() / 2.,
                    bounds.origin.y.as_f32() + bounds.size.height.as_f32() / 2.,
                ),
                bounds.size.width.as_f32() / 2.,
                bounds.size.height.as_f32() / 2.,
                width / 2.,
                rgba,
            );
        }
        CanvasShape::Text { .. } | CanvasShape::Image { .. } => {}
    }
}

pub(crate) fn compose_export(
    base: &RgbaImage,
    filters: &FilterSettings,
    annotations: &[CanvasShape],
) -> RgbaImage {
    let mut out = apply_filters(base, filters);
    for shape in annotations {
        rasterize_shape(&mut out, shape, [0, 0, 0, 255], 2.);
    }
    out
}

#[cfg(test)]
#[path = "bake.test.rs"]
mod bake_test;
