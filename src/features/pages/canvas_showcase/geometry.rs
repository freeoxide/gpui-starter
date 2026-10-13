use gpui_kit::*;

pub(super) const HANDLE_SIDE: Pixels = px(8.);
pub(super) const MIN_SHAPE_SIZE: Pixels = px(8.);

pub(super) fn square_at(center: Point<Pixels>, side: Pixels) -> Bounds<Pixels> {
    let half = side.as_f32() / 2.;
    Bounds {
        origin: center - point(px(half), px(half)),
        size: size(side, side),
    }
}

pub(super) fn handle_positions(b: Bounds<Pixels>) -> [Point<Pixels>; 8] {
    let left = b.origin.x;
    let top = b.origin.y;
    let right = b.origin.x + b.size.width;
    let bottom = b.origin.y + b.size.height;
    let mid_x = px((left.as_f32() + right.as_f32()) / 2.);
    let mid_y = px((top.as_f32() + bottom.as_f32()) / 2.);
    [
        point(left, top),
        point(mid_x, top),
        point(right, top),
        point(right, mid_y),
        point(right, bottom),
        point(mid_x, bottom),
        point(left, bottom),
        point(left, mid_y),
    ]
}

pub(super) fn resize_bounds(
    start: Bounds<Pixels>,
    handle_ix: usize,
    pointer: Point<Pixels>,
) -> Bounds<Pixels> {
    let mut left = start.origin.x;
    let mut top = start.origin.y;
    let mut right = start.origin.x + start.size.width;
    let mut bottom = start.origin.y + start.size.height;
    let moves_left = matches!(handle_ix, 0 | 6 | 7);
    let moves_right = matches!(handle_ix, 2..=4);
    let moves_top = matches!(handle_ix, 0..=2);
    let moves_bottom = matches!(handle_ix, 4..=6);
    match handle_ix {
        0 => {
            left = pointer.x;
            top = pointer.y;
        }
        1 => top = pointer.y,
        2 => {
            right = pointer.x;
            top = pointer.y;
        }
        3 => right = pointer.x,
        4 => {
            right = pointer.x;
            bottom = pointer.y;
        }
        5 => bottom = pointer.y,
        6 => {
            left = pointer.x;
            bottom = pointer.y;
        }
        7 => left = pointer.x,
        _ => {}
    }
    if moves_left && right - left < MIN_SHAPE_SIZE {
        left = right - MIN_SHAPE_SIZE;
    } else if moves_right && right - left < MIN_SHAPE_SIZE {
        right = left + MIN_SHAPE_SIZE;
    }
    if moves_top && bottom - top < MIN_SHAPE_SIZE {
        top = bottom - MIN_SHAPE_SIZE;
    } else if moves_bottom && bottom - top < MIN_SHAPE_SIZE {
        bottom = top + MIN_SHAPE_SIZE;
    }
    Bounds {
        origin: point(left, top),
        size: size(right - left, bottom - top),
    }
}

pub(super) fn scale_points(
    points: &[Point<Pixels>],
    old: Bounds<Pixels>,
    new: Bounds<Pixels>,
) -> Vec<Point<Pixels>> {
    let old_w = old.size.width.as_f32();
    let old_h = old.size.height.as_f32();
    if old_w == 0. || old_h == 0. {
        let delta = new.origin - old.origin;
        return points.iter().map(|p| *p + delta).collect();
    }
    let sx = new.size.width.as_f32() / old_w;
    let sy = new.size.height.as_f32() / old_h;
    points
        .iter()
        .map(|p| {
            point(
                px(new.origin.x.as_f32() + (p.x - old.origin.x).as_f32() * sx),
                px(new.origin.y.as_f32() + (p.y - old.origin.y).as_f32() * sy),
            )
        })
        .collect()
}

pub(super) fn rect_from_corners(a: Point<Pixels>, b: Point<Pixels>) -> Bounds<Pixels> {
    let left = a.x.min(b.x);
    let right = a.x.max(b.x);
    let top = a.y.min(b.y);
    let bottom = a.y.max(b.y);
    Bounds {
        origin: point(left, top),
        size: size(right - left, bottom - top),
    }
}

pub(super) fn sample_cubic_bezier(
    p0: Point<Pixels>,
    c1: Point<Pixels>,
    c2: Point<Pixels>,
    p1: Point<Pixels>,
    segments: usize,
) -> Vec<Point<Pixels>> {
    (0..=segments)
        .map(|i| {
            let t = i as f32 / segments as f32;
            let u = 1. - t;
            let w0 = u * u * u;
            let w1 = 3. * u * u * t;
            let w2 = 3. * u * t * t;
            let w3 = t * t * t;
            point(
                px(w0 * p0.x.as_f32()
                    + w1 * c1.x.as_f32()
                    + w2 * c2.x.as_f32()
                    + w3 * p1.x.as_f32()),
                px(w0 * p0.y.as_f32()
                    + w1 * c1.y.as_f32()
                    + w2 * c2.y.as_f32()
                    + w3 * p1.y.as_f32()),
            )
        })
        .collect()
}

#[cfg(test)]
#[path = "geometry.test.rs"]
mod geometry_test;
