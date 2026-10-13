// No `use super::*`: gpui's `test` proc-macro glob would make bare `#[test]`
// recurse infinitely. Import only what we need.
use gpui_kit::{Pixels, point, px};

use super::grid_step;
use super::smoothed_segments;

fn assert_px_eq(actual: Pixels, expected: Pixels) {
    assert!(
        (actual.as_f32() - expected.as_f32()).abs() < 0.01,
        "expected ~{expected:?}, got {actual:?}"
    );
}

#[test]
fn grid_step_doubles_until_it_reaches_the_minor_target() {
    assert_px_eq(grid_step(px(16.), 1.), px(32.));
}

#[test]
fn grid_step_doubles_repeatedly_under_low_zoom() {
    assert_px_eq(grid_step(px(16.), 0.25), px(128.));
}

#[test]
fn grid_step_halves_once_under_high_zoom() {
    assert_px_eq(grid_step(px(16.), 4.), px(8.));
}

#[test]
fn grid_step_keeps_the_screen_step_between_the_targets_across_zooms() {
    for zoom in [0.05, 0.1, 0.25, 0.5, 1., 2., 4., 8., 32.] {
        let screen_step = grid_step(px(16.), zoom) * zoom;
        assert!(
            screen_step.as_f32() >= 32. && screen_step.as_f32() < 64.,
            "zoom {zoom} produced screen step {screen_step:?}"
        );
    }
}

#[test]
fn smoothed_segments_return_midpoint_targets_with_source_controls() {
    let points = vec![
        point(px(0.), px(0.)),
        point(px(10.), px(0.)),
        point(px(10.), px(10.)),
    ];
    let segments = smoothed_segments(&points);
    assert_eq!(segments.len(), 1);
    let (to, ctrl) = segments[0];
    assert_px_eq(to.x, px(10.));
    assert_px_eq(to.y, px(5.));
    assert_px_eq(ctrl.x, px(10.));
    assert_px_eq(ctrl.y, px(0.));
}

#[test]
fn smoothed_segments_skip_short_paths() {
    assert!(smoothed_segments(&[]).is_empty());
    assert!(smoothed_segments(&[point(px(1.), px(1.))]).is_empty());
    let two = smoothed_segments(&[point(px(1.), px(1.)), point(px(5.), px(5.))]);
    assert!(two.is_empty());
}
