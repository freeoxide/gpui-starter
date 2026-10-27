use gpui_kit::{Bounds, Pixels, point, px, size};

use super::*;

fn viewport() -> gpui_kit::Size<Pixels> {
    size(px(800.), px(600.))
}

fn panel() -> gpui_kit::Size<Pixels> {
    size(px(200.), px(120.))
}

#[test]
fn snap_left_at_zero_ten_and_twenty_four_px() {
    for x in [0., 10., 24.] {
        let snapped = snap_origin(point(px(x), px(50.)), panel(), viewport());
        assert_eq!(
            snapped,
            point(px(0.), px(50.)),
            "x={x} must snap flush left"
        );
    }
}

#[test]
fn no_snap_beyond_the_threshold() {
    let snapped = snap_origin(point(px(25.), px(50.)), panel(), viewport());
    assert_eq!(snapped, point(px(25.), px(50.)));
}

#[test]
fn snap_right_within_the_threshold_of_the_right_edge() {
    for x in [580., 590., 600.] {
        let snapped = snap_origin(point(px(x), px(50.)), panel(), viewport());
        assert_eq!(snapped.x, px(600.), "x={x} must snap flush right");
    }
}

#[test]
fn both_edges_qualifying_snaps_left() {
    let narrow = size(px(210.), px(600.));
    let snapped = snap_origin(point(px(5.), px(50.)), panel(), narrow);
    assert_eq!(snapped.x, px(0.), "left edge wins when both qualify");
}

#[test]
fn clamp_pulls_each_axis_into_the_viewport() {
    let clamped = clamp_origin(point(px(-40.), px(700.)), panel(), viewport());
    assert_eq!(clamped, point(px(0.), px(480.)));
}

#[test]
fn oversized_panel_pins_to_the_origin() {
    let big = size(px(1000.), px(2000.));
    let clamped = clamp_origin(point(px(123.), px(45.)), big, viewport());
    assert_eq!(clamped, point(px(0.), px(0.)));
}

#[test]
fn restore_anchor_reproduces_the_origin_for_various_visible_bounds() {
    let cases = [
        (
            point(px(100.), px(80.)),
            Bounds {
                origin: point(px(0.), px(0.)),
                size: size(px(1920.), px(1080.)),
            },
        ),
        (
            point(px(1960.), px(120.)),
            Bounds {
                origin: point(px(1512.), px(0.)),
                size: size(px(1512.), px(982.)),
            },
        ),
        (
            point(px(-200.), px(40.)),
            Bounds {
                origin: point(px(-300.), px(0.)),
                size: size(px(1440.), px(900.)),
            },
        ),
    ];
    for (origin, visible) in cases {
        let FloatAnchor::Screen {
            display,
            placement,
            offset,
        } = restore_anchor(origin, visible)
        else {
            panic!("restore_anchor must be screen-anchored");
        };
        assert!(display.is_none());
        assert_eq!(placement, crate::ui::floating::Placement9::TopLeft);
        assert_eq!(visible.origin + offset, origin);
    }
}
