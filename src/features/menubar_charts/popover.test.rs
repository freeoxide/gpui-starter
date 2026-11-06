use gpui_kit::{Bounds, TestAppContext, WindowOptions, point, px, size};

use super::{POPOVER_ID, anchor, toggle_popover};
use crate::app::window::RootWindow;
use crate::ui::floating::{FloatAnchor, Placement9, floating_snapshot};

fn visible(origin_x: f32, origin_y: f32) -> Bounds<gpui_kit::Pixels> {
    Bounds {
        origin: point(px(origin_x), px(origin_y)),
        size: size(px(1440.), px(875.)),
    }
}

#[test]
fn anchor_converts_physical_to_logical_against_visible_bounds() {
    let FloatAnchor::Screen {
        display,
        placement,
        offset,
    } = anchor(400., 2.0, visible(0., 25.))
    else {
        panic!("expected a screen anchor");
    };
    assert_eq!(display, None);
    assert_eq!(placement, Placement9::TopLeft);
    assert_eq!(offset, point(px(200.), px(0.)));
}

#[test]
fn anchor_handles_unit_scale() {
    let FloatAnchor::Screen { offset, .. } = anchor(400., 1.0, visible(0., 25.)) else {
        panic!("expected a screen anchor");
    };
    assert_eq!(offset, point(px(400.), px(0.)));
}

#[test]
fn anchor_tolerates_a_left_of_visible_icon() {
    let FloatAnchor::Screen { offset, .. } = anchor(10., 2.0, visible(48., 25.)) else {
        panic!("expected a screen anchor");
    };
    assert_eq!(offset, point(px(5. - 48.), px(0.)));
}

#[gpui_kit::test]
async fn toggle_popover_round_trips_the_registered_float(cx: &TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(crate::ui::floating::init);
    let (handle, _) = cx
        .update(|cx| {
            gpui_kit::open_window(
                WindowOptions::default(),
                cx,
                super::super::panel::ChartsPanel::view,
            )
        })
        .expect("test window must open");
    cx.update(|cx| cx.set_global(RootWindow(handle)));

    assert!(cx.update(|cx| toggle_popover(400., cx)));
    assert!(
        cx.read(|cx| floating_snapshot(cx).iter().any(|e| e.id.0 == POPOVER_ID)),
        "open registers the popover float"
    );

    assert!(cx.update(|cx| toggle_popover(400., cx)));
    assert!(
        cx.read(|cx| floating_snapshot(cx)).is_empty(),
        "a second toggle closes the popover instead of stacking"
    );
}
