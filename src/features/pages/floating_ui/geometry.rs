use gpui_kit::{Pixels, Point, px};

#[cfg(not(target_family = "wasm"))]
use crate::ui::floating::{FloatAnchor, Placement9};
#[cfg(not(target_family = "wasm"))]
use gpui_kit::Bounds;

pub(crate) const SNAP_THRESHOLD: Pixels = px(24.);

pub(crate) fn clamp_origin(
    origin: Point<Pixels>,
    size: gpui_kit::Size<Pixels>,
    viewport: gpui_kit::Size<Pixels>,
) -> Point<Pixels> {
    let max_x = (viewport.width - size.width).max(px(0.));
    let max_y = (viewport.height - size.height).max(px(0.));
    gpui_kit::point(origin.x.clamp(px(0.), max_x), origin.y.clamp(px(0.), max_y))
}

pub(crate) fn snap_origin(
    origin: Point<Pixels>,
    size: gpui_kit::Size<Pixels>,
    viewport: gpui_kit::Size<Pixels>,
) -> Point<Pixels> {
    let clamped = clamp_origin(origin, size, viewport);
    let x = if clamped.x.abs() <= SNAP_THRESHOLD {
        px(0.)
    } else if (clamped.x - (viewport.width - size.width)).abs() <= SNAP_THRESHOLD {
        (viewport.width - size.width).max(px(0.))
    } else {
        clamped.x
    };
    gpui_kit::point(x, clamped.y)
}

// Wasm never spawns native windows, so the anchor math has no caller there.
#[cfg(not(target_family = "wasm"))]
pub(crate) fn restore_anchor(origin: Point<Pixels>, screen_visible: Bounds<Pixels>) -> FloatAnchor {
    FloatAnchor::Screen {
        display: None,
        placement: Placement9::TopLeft,
        offset: origin - screen_visible.origin,
    }
}

#[cfg(test)]
#[path = "geometry.test.rs"]
mod geometry_test;
