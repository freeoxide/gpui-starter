use gpui_kit::{App, Bounds, Pixels, point, px, size};

use super::IconRect;
use crate::ui::floating::{
    FloatAnchor, FloatLevel, FloatingSpec, FloatingWindowId, Placement9, close_floating,
    floating_snapshot, spawn_floating,
};

const POPOVER_ID: &str = "menubar-charts";

pub(crate) fn anchor(rect: IconRect, scale: f32, visible: Bounds<Pixels>) -> FloatAnchor {
    let logical_x = px((rect.x / scale as f64) as f32);
    let offset = point(logical_x - visible.origin.x, px(0.));
    FloatAnchor::Screen {
        display: None,
        placement: Placement9::TopLeft,
        offset,
    }
}

/// macOS path: toggle the anchored popover. Closes it if our id is in
/// `floating_snapshot`; otherwise spawns at `FloatLevel::AlwaysOnTop`,
/// `focus(false)`, anchored under `rect`. Returns false when the anchor or
/// spawn fails (no root window for the scale factor, or `spawn_floating`
/// errored).
pub fn toggle_popover(rect: IconRect, cx: &mut App) -> bool {
    if let Some(entry) = floating_snapshot(cx)
        .into_iter()
        .find(|entry| entry.id.0 == POPOVER_ID)
    {
        close_floating(&entry.handle, cx);
        return true;
    }
    let Some(window) = crate::app::window::root_window(cx) else {
        tracing::warn!(
            target: "gpui_starter::menubar_charts",
            "no root window recorded; charts popover ignored"
        );
        return false;
    };
    let scale = window.update(cx, |_, window, _| window.scale_factor());
    let Ok(scale) = scale else {
        tracing::warn!(
            target: "gpui_starter::menubar_charts",
            "root window unavailable; charts popover ignored"
        );
        return false;
    };
    let Some(visible) = cx.primary_display().map(|display| display.visible_bounds()) else {
        tracing::warn!(
            target: "gpui_starter::menubar_charts",
            "no primary display; charts popover ignored"
        );
        return false;
    };
    let spec = FloatingSpec::new(
        FloatingWindowId(POPOVER_ID.into()),
        size(px(360.), px(540.)),
    )
    .title(crate::i18n::localize("menubar_charts_title"))
    .level(FloatLevel::AlwaysOnTop)
    .anchor(anchor(rect, scale, visible))
    .focus(false);
    match spawn_floating(
        spec,
        |window, cx| super::panel::ChartsPanel::view(window, cx).into(),
        cx,
    ) {
        Ok(_) => true,
        Err(err) => {
            tracing::warn!(
                target: "gpui_starter::menubar_charts",
                error = %err,
                "charts popover spawn failed"
            );
            false
        }
    }
}

#[cfg(test)]
#[path = "popover.test.rs"]
mod popover_test;
