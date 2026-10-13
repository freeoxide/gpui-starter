use gpui_kit::{Bounds, point, px, size};

use super::{grid_cell_bounds, grid_dims, shader_hsv};

fn bounds_of(width: f32, height: f32) -> Bounds<gpui_kit::Pixels> {
    Bounds {
        origin: point(px(0.), px(0.)),
        size: size(px(width), px(height)),
    }
}

#[test]
fn shader_hsv_stays_in_hsv_ranges_across_a_sweep() {
    let coords = [0., 0.25, 0.5, 0.75, 1.];
    let times = [0., 1.7, 5., 100., 100000.];
    for u in coords {
        for v in coords {
            for t in times {
                let color = shader_hsv(u, v, t);
                assert!(
                    (0f32..1.).contains(&color.h),
                    "hue {u},{v},{t}: {}",
                    color.h
                );
                assert_eq!(color.s, 0.85);
                assert!(
                    (0.08..=0.92).contains(&color.l),
                    "lightness {u},{v},{t}: {}",
                    color.l
                );
                assert_eq!(color.a, 1.);
            }
        }
    }
}

#[test]
fn shader_hsv_varies_with_time() {
    assert_ne!(shader_hsv(0.5, 0.5, 0.), shader_hsv(0.5, 0.5, 5.));
}

#[test]
fn grid_dims_ceil_divides() {
    let bounds = bounds_of(400., 280.);
    assert_eq!(grid_dims(bounds, px(6.)), (67, 47));
}

#[test]
fn grid_dims_doubles_the_cell_until_the_cap_fits() {
    let bounds = bounds_of(400., 280.);
    assert_eq!(grid_dims(bounds, px(1.)), (50, 35));
}

#[test]
fn grid_cell_bounds_tiles_without_gaps_or_overrun() {
    let bounds = Bounds {
        origin: point(px(10.), px(20.)),
        size: size(px(100.), px(80.)),
    };
    let cell = px(30.);
    assert_eq!(grid_cell_bounds(bounds, cell, 0, 0).origin, bounds.origin);
    let last_column = grid_cell_bounds(bounds, cell, 3, 0);
    assert_eq!(last_column.right(), bounds.right());
    let mut previous_right = bounds.origin.x;
    for gx in 0..4 {
        let cell_bounds = grid_cell_bounds(bounds, cell, gx, 0);
        assert_eq!(cell_bounds.origin.x, previous_right);
        previous_right = cell_bounds.right();
    }
    assert_eq!(previous_right, bounds.right());
}

#[test]
fn grid_cell_bounds_clamps_the_last_row() {
    let bounds = Bounds {
        origin: point(px(0.), px(0.)),
        size: size(px(80.), px(70.)),
    };
    let cell = px(30.);
    let last_row = grid_cell_bounds(bounds, cell, 0, 2);
    assert_eq!(last_row.origin.y, px(60.));
    assert_eq!(last_row.bottom(), bounds.bottom());
}
