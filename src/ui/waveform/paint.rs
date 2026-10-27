use gpui_kit::*;

use super::history::ColumnAgg;

pub(crate) fn snap_to_device(v: f32, scale: f32) -> f32 {
    (v * scale).round() / scale
}

pub(crate) fn nice_time_step(window_secs: f32, target_lines: usize) -> f32 {
    for k in -3i32..=2 {
        for mantissa in [1., 2., 5.] {
            let step = mantissa * 10f32.powi(k);
            if window_secs / step <= target_lines as f32 {
                return step;
            }
        }
    }
    500.
}

pub(crate) fn time_gridline_xs(
    bounds: Bounds<Pixels>,
    window_secs: f32,
    step: f32,
    scale: f32,
) -> Vec<Pixels> {
    let width = bounds.size.width.as_f32();
    let left = bounds.left().as_f32();
    let right = bounds.right().as_f32();
    let mut xs = Vec::new();
    let mut k = 1;
    loop {
        let x = snap_to_device(right - k as f32 * step / window_secs * width, scale);
        if x <= left {
            break;
        }
        xs.push(px(x));
        k += 1;
    }
    xs
}

pub(crate) fn amplitude_y(value: f32, lane: Bounds<Pixels>) -> Pixels {
    lane.center().y - lane.size.height * 0.5 * value.clamp(-1., 1.)
}

fn column_x_span(lane: Bounds<Pixels>, ix: usize, count: usize, scale: f32) -> (f32, f32) {
    let left = lane.origin.x.as_f32();
    let width = lane.size.width.as_f32();
    let x0 = snap_to_device(left + ix as f32 * width / count as f32, scale);
    let x1 = snap_to_device(left + (ix + 1) as f32 * width / count as f32, scale);
    (x0, x1)
}

pub(crate) fn envelope_bounds(
    agg: &ColumnAgg,
    col_ix: usize,
    columns: usize,
    lane: Bounds<Pixels>,
    scale: f32,
) -> Option<(Bounds<Pixels>, Bounds<Pixels>)> {
    if !agg.populated {
        return None;
    }
    let (x0, x1) = column_x_span(lane, col_ix, columns, scale);
    let floor = 1. / scale;
    let width = px((x1 - x0).max(0.));
    let peak_top = amplitude_y(agg.max, lane);
    let peak_height = (amplitude_y(agg.min, lane) - peak_top).max(px(floor));
    let rms_top = amplitude_y(agg.rms, lane);
    let rms_height = (amplitude_y(-agg.rms, lane) - rms_top).max(px(floor));
    Some((
        Bounds {
            origin: point(px(x0), peak_top),
            size: size(width, peak_height),
        },
        Bounds {
            origin: point(px(x0), rms_top),
            size: size(width, rms_height),
        },
    ))
}

pub(crate) fn oscilloscope_line_points(
    samples: &[f32],
    window_samples: usize,
    lane: Bounds<Pixels>,
    scale: f32,
) -> Vec<Point<Pixels>> {
    let m = samples.len();
    if m == 0 {
        return Vec::new();
    }
    let window = window_samples.max(m).max(1);
    let left = lane.origin.x.as_f32();
    let width = lane.size.width.as_f32();
    (0..m)
        .map(|i| {
            let x = snap_to_device(
                left + width * ((window - m + i) as f32) / window as f32,
                scale,
            );
            point(px(x), amplitude_y(samples[i], lane))
        })
        .collect()
}

pub(crate) fn bar_bounds(
    height_norm: f32,
    bar_ix: usize,
    bars: usize,
    lane: Bounds<Pixels>,
    scale: f32,
) -> Bounds<Pixels> {
    let gap = 1. / scale;
    let (span_x0, span_x1) = column_x_span(lane, bar_ix, bars, scale);
    let x0 = span_x0 + gap;
    let x1 = (span_x1 - gap).max(x0);
    let height = lane.size.height * height_norm.clamp(0., 1.);
    Bounds {
        origin: point(px(x0), lane.bottom() - height),
        size: size(px(x1 - x0), height),
    }
}

#[cfg(test)]
#[path = "paint.test.rs"]
mod paint_test;
