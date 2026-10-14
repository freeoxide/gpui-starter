use std::rc::Rc;

use gpui_kit::{AnyView, App, Window};

use crate::ui::floating::{
    FloatAnchor, FloatLevel, FloatingSpec, FloatingWindowId, LayerShellSpec, Placement9,
    WidgetLayer, close_floating, floating_snapshot, set_floating_always_on_top,
    set_floating_click_through, spawn_floating, spawn_floating_after_displays,
};

use super::battery::BatteryWidget;
use super::catalog::{self, PersistedWidget, WidgetKind};
use super::clock::ClockWidget;
use super::metrics::MetricsWidget;
use super::system_sliders::SystemSlidersWidget;

const LOG: &str = "gpui_starter::features::desktop_widgets";

/// The stable float id for a kind ("widget-clock" etc.), shared by spawn,
/// the manager rows, and tests.
pub fn widget_id(kind: WidgetKind) -> FloatingWindowId {
    FloatingWindowId(format!("widget-{}", kind.as_str()).into())
}

pub(crate) fn widget_spec(kind: WidgetKind, persisted: &PersistedWidget, cx: &App) -> FloatingSpec {
    let defaults = catalog::defaults(kind);
    let placement = catalog::placement_from_str(&persisted.placement).unwrap_or(defaults.placement);
    let mut spec = FloatingSpec::new(widget_id(kind), defaults.size)
        .title(crate::i18n::localize(kind.title_key()))
        .level(if persisted.always_on_top {
            FloatLevel::AlwaysOnTop
        } else {
            FloatLevel::Floating
        })
        .anchor(FloatAnchor::Screen {
            display: None,
            placement,
            offset: catalog::placement_offset(placement),
        })
        .focus(false)
        .click_through(persisted.click_through);
    if cx.compositor_name() == "Wayland" {
        let layer = if persisted.layer == "pinned" {
            WidgetLayer::Background
        } else {
            WidgetLayer::Top
        };
        spec = spec.layer_shell(LayerShellSpec::new().layer(layer));
    }
    spec
}

pub(crate) fn widget_view_factory(
    kind: WidgetKind,
) -> Rc<dyn Fn(&mut Window, &mut App) -> AnyView> {
    match kind {
        WidgetKind::Clock => Rc::new(|window, cx| ClockWidget::view(window, cx).into()),
        WidgetKind::Battery => Rc::new(|window, cx| BatteryWidget::view(window, cx).into()),
        WidgetKind::System => Rc::new(|window, cx| SystemSlidersWidget::view(window, cx).into()),
        WidgetKind::Metrics => Rc::new(|window, cx| MetricsWidget::view(window, cx).into()),
    }
}

pub(crate) fn default_entry(kind: WidgetKind) -> PersistedWidget {
    let defaults = catalog::defaults(kind);
    PersistedWidget {
        kind: kind.as_str().to_string(),
        enabled: false,
        placement: catalog::placement_to_str(defaults.placement).to_string(),
        always_on_top: true,
        click_through: false,
        layer: "above".to_string(),
    }
}

fn current_entry(kind: WidgetKind, cx: &App) -> PersistedWidget {
    catalog::slot(kind, cx).unwrap_or_else(|| default_entry(kind))
}

fn open_handle(kind: WidgetKind, cx: &App) -> Option<crate::ui::floating::FloatingHandle> {
    floating_snapshot(cx)
        .into_iter()
        .find(|e| e.id == widget_id(kind))
        .map(|e| e.handle)
}

fn spawn_open(kind: WidgetKind, cx: &mut App) -> bool {
    let entry = current_entry(kind, cx);
    let spec = widget_spec(kind, &entry, cx);
    let factory = widget_view_factory(kind);
    match spawn_floating(spec, move |window, cx| factory(window, cx), cx) {
        Ok(_) => true,
        Err(err) => {
            tracing::warn!(target: LOG, error = %err, kind = kind.as_str(), "widget spawn failed");
            false
        }
    }
}

fn respawn(kind: WidgetKind, cx: &mut App) -> bool {
    let Some(handle) = open_handle(kind, cx) else {
        return true;
    };
    close_floating(&handle, cx);
    spawn_open(kind, cx)
}

/// Show/hide on the desktop. Persists the desired state; `on` spawns the
/// float (spawn errors log at warn and return false), `off` closes it through
/// plan 02. Returns false when the spawn/close did not complete.
pub fn set_widget_enabled(kind: WidgetKind, on: bool, cx: &mut App) -> bool {
    let mut entry = current_entry(kind, cx);
    entry.enabled = on;
    catalog::save_slot(entry, cx);
    if on {
        spawn_open(kind, cx)
    } else {
        open_handle(kind, cx)
            .map(|handle| close_floating(&handle, cx))
            .unwrap_or(false)
    }
}

/// Change the screen position: persists, then (if open) close + respawn at
/// the new anchor. 02 has no move API; respawn is the path.
pub fn set_widget_placement(kind: WidgetKind, placement: Placement9, cx: &mut App) -> bool {
    let mut entry = current_entry(kind, cx);
    entry.placement = catalog::placement_to_str(placement).to_string();
    catalog::save_slot(entry, cx);
    respawn(kind, cx)
}

/// Runtime always-on-top: persists, applies 25's toggle; when the runtime
/// apply returns false (Wayland: z-order is compositor-owned) the widget is
/// respawned at the new FloatLevel, which is creation-time on every platform.
pub fn set_widget_always_on_top(kind: WidgetKind, on: bool, cx: &mut App) -> bool {
    let mut entry = current_entry(kind, cx);
    entry.always_on_top = on;
    catalog::save_slot(entry, cx);
    let Some(handle) = open_handle(kind, cx) else {
        return true;
    };
    let applied = set_floating_always_on_top(&handle, on, cx);
    if applied {
        return true;
    }
    respawn(kind, cx)
}

/// Runtime click-through: persists, applies 25's toggle. Returns the shim's
/// bool (false on wasm overlay floats, test platform, gone handles).
pub fn set_widget_click_through(kind: WidgetKind, on: bool, cx: &mut App) -> bool {
    let mut entry = current_entry(kind, cx);
    entry.click_through = on;
    catalog::save_slot(entry, cx);
    let Some(handle) = open_handle(kind, cx) else {
        return true;
    };
    set_floating_click_through(&handle, on, cx)
}

/// Wayland layer choice ("above" = WidgetLayer::Top, "pinned" =
/// WidgetLayer::Background). Persists; if open, close + respawn so the new
/// LayerShellSpec applies (layer is creation-only). Off Wayland the
/// persisted value is inert.
pub fn set_widget_layer(kind: WidgetKind, layer: &str, cx: &mut App) -> bool {
    if layer != "above" && layer != "pinned" {
        return false;
    }
    let mut entry = current_entry(kind, cx);
    entry.layer = layer.to_string();
    catalog::save_slot(entry, cx);
    respawn(kind, cx)
}

/// Respawn every enabled widget that is not already open. Uses 26's
/// spawn_floating_after_displays (zed#46378-safe). Native boot path; the
/// wasm page calls it on first open instead.
pub fn spawn_persisted(cx: &mut App) {
    let open: Vec<FloatingWindowId> = floating_snapshot(cx).into_iter().map(|e| e.id).collect();
    for entry in catalog::widget_layout(cx) {
        let Some(kind) = WidgetKind::from_str(&entry.kind) else {
            continue;
        };
        if !entry.enabled || !catalog::available_kinds().contains(&kind) {
            continue;
        }
        let id = widget_id(kind);
        if open.contains(&id) {
            continue;
        }
        let spec = widget_spec(kind, &entry, cx);
        let factory = widget_view_factory(kind);
        spawn_floating_after_displays(spec, move |window, cx| factory(window, cx), cx);
    }
}

#[cfg(test)]
#[path = "spawn.test.rs"]
mod spawn_test;
