use gpui_kit::component::ActiveTheme as _;
use gpui_kit::*;

use super::cache::{CanvasCaches, shape_key};
use super::geometry::CanvasTransform;
use super::scene::{CanvasDocument, CanvasShape, ShapeId, StrokeStyle};
use super::state::CanvasGrid;

const GRID_MINOR_OPACITY: f32 = 0.4;
const GRID_MAJOR_OPACITY: f32 = 0.8;
const GRID_MINOR_TARGET: f32 = 32.;
const GRID_MAJOR_TARGET: f32 = 64.;
const MAJOR_LINE_EVERY: i64 = 4;

pub(crate) fn grid_step(spacing: Pixels, zoom: f32) -> Pixels {
    let mut step = spacing.max(px(1.));
    while step * zoom < px(GRID_MINOR_TARGET) {
        step *= 2.;
    }
    while step * zoom >= px(GRID_MAJOR_TARGET) {
        step = px(step.as_f32() / 2.);
    }
    step
}

fn midpoint(a: Point<Pixels>, b: Point<Pixels>) -> Point<Pixels> {
    point(
        px((a.x.as_f32() + b.x.as_f32()) / 2.),
        px((a.y.as_f32() + b.y.as_f32()) / 2.),
    )
}

pub(crate) fn smoothed_segments(points: &[Point<Pixels>]) -> Vec<(Point<Pixels>, Point<Pixels>)> {
    let mut segments = Vec::new();
    if points.len() < 3 {
        return segments;
    }
    for i in 1..points.len() - 1 {
        segments.push((midpoint(points[i], points[i + 1]), points[i]));
    }
    segments
}

pub(crate) fn paint_grid(
    bounds: Bounds<Pixels>,
    grid: CanvasGrid,
    transform: CanvasTransform,
    window: &mut Window,
    cx: &mut App,
) {
    window.paint_quad(fill(bounds, cx.theme().background));
    if !grid.visible {
        return;
    }
    let local = Bounds {
        origin: Point::default(),
        size: bounds.size,
    };
    let doc_range = transform.to_doc_bounds(local);
    let step = grid_step(grid.spacing, transform.zoom);
    let minor = cx.theme().border.opacity(GRID_MINOR_OPACITY);
    let major = cx.theme().border.opacity(GRID_MAJOR_OPACITY);

    let doc_left = doc_range.origin.x.as_f32();
    let doc_right = doc_left + doc_range.size.width.as_f32();
    let mut k = (doc_left / step.as_f32()).floor() as i64;
    while k as f32 * step.as_f32() <= doc_right {
        let doc_x = k as f32 * step.as_f32();
        let viewport_x = (doc_x * transform.zoom + transform.pan.x.as_f32()).round();
        let color = if k.rem_euclid(MAJOR_LINE_EVERY) == 0 {
            major
        } else {
            minor
        };
        window.paint_quad(fill(
            Bounds {
                origin: bounds.origin + point(px(viewport_x), px(0.)),
                size: size(px(1.), bounds.size.height),
            },
            color,
        ));
        k += 1;
    }

    let doc_top = doc_range.origin.y.as_f32();
    let doc_bottom = doc_top + doc_range.size.height.as_f32();
    let mut k = (doc_top / step.as_f32()).floor() as i64;
    while k as f32 * step.as_f32() <= doc_bottom {
        let doc_y = k as f32 * step.as_f32();
        let viewport_y = (doc_y * transform.zoom + transform.pan.y.as_f32()).round();
        let color = if k.rem_euclid(MAJOR_LINE_EVERY) == 0 {
            major
        } else {
            minor
        };
        window.paint_quad(fill(
            Bounds {
                origin: bounds.origin + point(px(0.), px(viewport_y)),
                size: size(bounds.size.width, px(1.)),
            },
            color,
        ));
        k += 1;
    }
}

fn stroke_builder(stroke: &StrokeStyle, zoom: f32) -> PathBuilder {
    let builder = PathBuilder::stroke(stroke.width * zoom);
    match &stroke.dash {
        Some(dash) => {
            let screen_dash: Vec<Pixels> = dash.iter().map(|length| *length * zoom).collect();
            builder.dash_array(&screen_dash)
        }
        None => builder,
    }
}

fn rect_outline_into(builder: &mut PathBuilder, width: Pixels, height: Pixels) {
    builder.move_to(point(px(0.), px(0.)));
    builder.line_to(point(width, px(0.)));
    builder.line_to(point(width, height));
    builder.line_to(point(px(0.), height));
    builder.close();
}

fn ellipse_outline_into(builder: &mut PathBuilder, radius_x: Pixels, radius_y: Pixels) {
    let kappa = 0.552_284_8;
    let control_x = px(radius_x.as_f32() * kappa);
    let control_y = px(radius_y.as_f32() * kappa);
    builder.move_to(point(radius_x, px(0.)));
    builder.cubic_bezier_to(
        point(px(0.), radius_y),
        point(radius_x, control_y),
        point(control_x, radius_y),
    );
    builder.cubic_bezier_to(
        point(-radius_x, px(0.)),
        point(-control_x, radius_y),
        point(-radius_x, control_y),
    );
    builder.cubic_bezier_to(
        point(px(0.), -radius_y),
        point(-radius_x, -control_y),
        point(-control_x, -radius_y),
    );
    builder.cubic_bezier_to(
        point(radius_x, px(0.)),
        point(control_x, -radius_y),
        point(radius_x, -control_y),
    );
    builder.close();
}

fn paint_dot(center: Point<Pixels>, diameter: Pixels, color: Hsla, window: &mut Window) {
    let radius = diameter * 0.5;
    if radius <= px(0.) {
        return;
    }
    let mut builder = PathBuilder::fill();
    ellipse_outline_into(&mut builder, radius, radius);
    builder.translate(center);
    if let Ok(path) = builder.build() {
        window.paint_path(path, color);
    }
}

pub(crate) fn paint_shape(
    shape: &CanvasShape,
    transform: CanvasTransform,
    bounds: Bounds<Pixels>,
    caches: &mut CanvasCaches,
    stroke_slot: usize,
    fill_slot: usize,
    window: &mut Window,
    cx: &mut App,
) {
    let local = Bounds {
        origin: Point::default(),
        size: bounds.size,
    };
    let doc_bounds = shape.doc_bounds();
    if !transform.to_screen_bounds(doc_bounds).intersects(&local) {
        return;
    }
    let zoom = transform.zoom;
    match shape {
        CanvasShape::Rect {
            bounds: shape_bounds,
            style,
        } => {
            let screen = transform.to_screen_bounds(*shape_bounds);
            if let Some(fill_color) = style.fill() {
                window.paint_quad(fill(
                    Bounds {
                        origin: screen.origin + bounds.origin,
                        size: screen.size,
                    },
                    fill_color,
                ));
            }
            if let Some(stroke) = style.stroke() {
                let width = shape_bounds.size.width * zoom;
                let height = shape_bounds.size.height * zoom;
                let origin = bounds.origin + transform.to_screen(shape_bounds.origin);
                let key = shape_key(shape, zoom);
                if let Some(path) = caches.slot(stroke_slot).get(key, origin, || {
                    let mut builder = stroke_builder(stroke, zoom);
                    rect_outline_into(&mut builder, width, height);
                    builder.build().ok()
                }) {
                    window.paint_path(path, stroke.color);
                }
            }
        }
        CanvasShape::Ellipse {
            bounds: shape_bounds,
            style,
        } => {
            let radius_x = shape_bounds.size.width * zoom * 0.5;
            let radius_y = shape_bounds.size.height * zoom * 0.5;
            let center = bounds.origin + transform.to_screen(shape_bounds.center());
            let key = shape_key(shape, zoom);
            if let Some(fill_color) = style.fill() {
                if let Some(path) = caches.slot(fill_slot).get(key, center, || {
                    let mut builder = PathBuilder::fill();
                    ellipse_outline_into(&mut builder, radius_x, radius_y);
                    builder.build().ok()
                }) {
                    window.paint_path(path, fill_color);
                }
            }
            if let Some(stroke) = style.stroke() {
                if let Some(path) = caches.slot(stroke_slot).get(key, center, || {
                    let mut builder = stroke_builder(stroke, zoom);
                    ellipse_outline_into(&mut builder, radius_x, radius_y);
                    builder.build().ok()
                }) {
                    window.paint_path(path, stroke.color);
                }
            }
        }
        CanvasShape::Path { points, style } => {
            if points.is_empty() {
                return;
            }
            if points.len() == 1 {
                let center = bounds.origin + transform.to_screen(points[0]);
                paint_dot(center, style.width * zoom, style.color, window);
                return;
            }
            let doc_origin = doc_bounds.origin;
            let origin = bounds.origin + transform.to_screen(doc_origin);
            let key = shape_key(shape, zoom);
            if let Some(path) = caches.slot(stroke_slot).get(key, origin, || {
                let scaled: Vec<Point<Pixels>> =
                    points.iter().map(|p| (*p - doc_origin) * zoom).collect();
                let mut builder = stroke_builder(style, zoom);
                builder.move_to(scaled[0]);
                for (to, ctrl) in smoothed_segments(&scaled) {
                    builder.curve_to(to, ctrl);
                }
                builder.line_to(scaled[scaled.len() - 1]);
                builder.build().ok()
            }) {
                window.paint_path(path, style.color);
            }
        }
        CanvasShape::Text {
            position,
            text,
            font_size,
            color,
        } => {
            let screen_font = *font_size * zoom;
            let run = TextRun {
                len: text.len(),
                font: Font::default(),
                color: *color,
                background_color: None,
                underline: None,
                strikethrough: None,
            };
            let shaped = window
                .text_system()
                .shape_line(text.clone(), screen_font, &[run], None);
            let origin = bounds.origin + transform.to_screen(*position);
            if let Err(err) = shaped.paint(
                origin,
                screen_font * 1.25,
                TextAlign::Left,
                None,
                window,
                cx,
            ) {
                tracing::warn!("canvas text paint failed: {err}");
            }
        }
        CanvasShape::Image {
            bounds: image_bounds,
            image,
        } => {
            let screen = transform.to_screen_bounds(*image_bounds);
            let window_bounds = Bounds {
                origin: screen.origin + bounds.origin,
                size: screen.size,
            };
            if let Err(err) = window.paint_image(
                window_bounds,
                window_bounds,
                Corners::default(),
                image.clone(),
                0,
                false,
            ) {
                tracing::warn!("canvas image paint failed: {err}");
            }
        }
    }
}

pub(crate) fn paint_selection(
    document: &CanvasDocument,
    selection: &[ShapeId],
    transform: CanvasTransform,
    bounds: Bounds<Pixels>,
    window: &mut Window,
    cx: &mut App,
) {
    let color = cx.theme().primary;
    for id in selection {
        let Some(shape) = document.shape(*id) else {
            continue;
        };
        let screen = transform.to_screen_bounds(shape.doc_bounds());
        let origin = bounds.origin + screen.origin;
        let width = screen.size.width.max(px(1.));
        let height = screen.size.height.max(px(1.));
        let mut builder = PathBuilder::stroke(px(1.));
        rect_outline_into(&mut builder, width, height);
        builder.translate(origin);
        if let Ok(path) = builder.build() {
            window.paint_path(path, color);
        }
    }
}

#[cfg(test)]
#[path = "paint.test.rs"]
mod paint_test;
