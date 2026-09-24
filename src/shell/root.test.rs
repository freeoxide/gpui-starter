// No `use super::*`: gpui's `test` proc-macro glob would make bare `#[test]`
// recurse infinitely. Import the items under test explicitly.
use super::super::frame_time::is_slow_frame;
use super::persisted_bounds;
use crate::app_state::PersistedWindowBounds;
use gpui::{Bounds, point, px, size};

#[test]
fn is_slow_frame_below_threshold() {
    assert!(!is_slow_frame(3_999));
}

#[test]
fn is_slow_frame_at_threshold() {
    assert!(!is_slow_frame(4_000));
}

#[test]
fn is_slow_frame_above_threshold() {
    assert!(is_slow_frame(4_001));
}

#[test]
fn persisted_bounds_copies_pixel_geometry() {
    let bounds = Bounds {
        origin: point(px(24.0), px(-8.0)),
        size: size(px(1400.0), px(900.0)),
    };
    assert_eq!(
        persisted_bounds(bounds),
        PersistedWindowBounds {
            x: 24.0,
            y: -8.0,
            width: 1400.0,
            height: 900.0,
        }
    );
}

#[test]
fn persisted_bounds_keeps_fractional_dpi_values() {
    let bounds = Bounds {
        origin: point(px(10.5), px(0.0)),
        size: size(px(700.25), px(450.75)),
    };
    let persisted = persisted_bounds(bounds);
    assert_eq!((persisted.x, persisted.width), (10.5, 700.25));
    assert_eq!((persisted.y, persisted.height), (0.0, 450.75));
}
