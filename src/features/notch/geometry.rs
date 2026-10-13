//! Pure placement math for the notch floater window geometry.

use gpui_kit::{Bounds, Pixels, point, px, size};

#[cfg(test)]
#[path = "geometry.test.rs"]
mod geometry_test;

#[derive(Clone, Copy)]
pub(crate) struct NotchLayout {
    pub collapsed: Bounds<Pixels>,
    pub expanded: Bounds<Pixels>,
}

// px() here is the audited exception to the rem-scale rule: this geometry
// must match the physical screen (design guides "Zoom, base font, and rem").
pub(crate) fn notch_layout(
    screen: Bounds<Pixels>,
    safe_top: Option<Pixels>,
    housing: Option<Bounds<Pixels>>,
) -> NotchLayout {
    let pill_h = px(safe_top.map(|p| p.as_f32()).unwrap_or(24.).clamp(20., 40.));
    let collapsed_w = px(housing
        .map(|h| h.size.width.as_f32())
        .unwrap_or(240.)
        .clamp(160., (screen.size.width * 0.6).as_f32()));
    let center_x = screen.center().x;
    let collapsed = Bounds::new(
        point(center_x - collapsed_w * 0.5, px(0.)),
        size(collapsed_w, pill_h),
    );
    let expanded_w = px(720f32.min((screen.size.width * 0.9).as_f32()));
    let expanded_h = px(200f32
        .min((screen.size.height - pill_h).as_f32())
        .max(pill_h.as_f32()));
    let expanded = Bounds::new(
        point(center_x - expanded_w * 0.5, px(0.)),
        size(expanded_w, expanded_h),
    );
    NotchLayout {
        collapsed,
        expanded,
    }
}
