use serde::{Deserialize, Serialize};

use gpui_kit::{App, Pixels, Point, Size, point, px, size};

use crate::app_state;
use crate::ui::floating::Placement9;

/// Built-in widget kinds. `System` is the brightness/volume quick-sliders
/// widget. Stable string forms are the persistence keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum WidgetKind {
    Clock,
    Battery,
    System,
    Metrics,
}

impl WidgetKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            WidgetKind::Clock => "clock",
            WidgetKind::Battery => "battery",
            WidgetKind::System => "system",
            WidgetKind::Metrics => "metrics",
        }
    }

    #[expect(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "clock" => Some(WidgetKind::Clock),
            "battery" => Some(WidgetKind::Battery),
            "system" => Some(WidgetKind::System),
            "metrics" => Some(WidgetKind::Metrics),
            _ => None,
        }
    }

    /// i18n key for the display title (also the float title).
    pub fn title_key(&self) -> &'static str {
        match self {
            WidgetKind::Clock => "widgets_kind_clock",
            WidgetKind::Battery => "widgets_kind_battery",
            WidgetKind::System => "widgets_kind_system",
            WidgetKind::Metrics => "widgets_kind_metrics",
        }
    }
}

pub(crate) const fn system_controls_available() -> bool {
    true
}

pub(crate) const fn metrics_available() -> bool {
    true
}

/// Kinds this build offers. Data-plan-dependent kinds drop out in reduced
/// builds (the plan's fallback recipe); the clock never does.
pub fn available_kinds() -> Vec<WidgetKind> {
    [
        WidgetKind::Clock,
        WidgetKind::Battery,
        WidgetKind::System,
        WidgetKind::Metrics,
    ]
    .into_iter()
    .filter(|kind| match kind {
        WidgetKind::Clock => true,
        WidgetKind::Battery | WidgetKind::System => system_controls_available(),
        WidgetKind::Metrics => metrics_available(),
    })
    .collect()
}

pub(crate) struct KindDefaults {
    pub size: Size<Pixels>,
    pub placement: Placement9,
}

pub(crate) fn defaults(kind: WidgetKind) -> KindDefaults {
    match kind {
        WidgetKind::Clock => KindDefaults {
            size: size(px(220.), px(120.)),
            placement: Placement9::TopRight,
        },
        WidgetKind::Battery => KindDefaults {
            size: size(px(240.), px(140.)),
            placement: Placement9::TopLeft,
        },
        WidgetKind::System => KindDefaults {
            size: size(px(260.), px(180.)),
            placement: Placement9::BottomRight,
        },
        WidgetKind::Metrics => KindDefaults {
            size: size(px(280.), px(180.)),
            placement: Placement9::BottomLeft,
        },
    }
}

pub(crate) fn placement_to_str(placement: Placement9) -> &'static str {
    match placement {
        Placement9::TopLeft => "top_left",
        Placement9::Top => "top",
        Placement9::TopRight => "top_right",
        Placement9::Left => "left",
        Placement9::Center => "center",
        Placement9::Right => "right",
        Placement9::BottomLeft => "bottom_left",
        Placement9::Bottom => "bottom",
        Placement9::BottomRight => "bottom_right",
    }
}

pub(crate) fn placement_from_str(s: &str) -> Option<Placement9> {
    match s {
        "top_left" => Some(Placement9::TopLeft),
        "top" => Some(Placement9::Top),
        "top_right" => Some(Placement9::TopRight),
        "left" => Some(Placement9::Left),
        "center" => Some(Placement9::Center),
        "right" => Some(Placement9::Right),
        "bottom_left" => Some(Placement9::BottomLeft),
        "bottom" => Some(Placement9::Bottom),
        "bottom_right" => Some(Placement9::BottomRight),
        _ => None,
    }
}

/// Push the widget away from its anchor edge by the inset; 02 clamps into
/// visible bounds after applying it.
pub(crate) fn placement_offset(placement: Placement9) -> Point<Pixels> {
    let inset = px(16.);
    let x = match placement {
        Placement9::TopLeft | Placement9::Left | Placement9::BottomLeft => inset,
        Placement9::TopRight | Placement9::Right | Placement9::BottomRight => -inset,
        Placement9::Top | Placement9::Center | Placement9::Bottom => px(0.),
    };
    let y = match placement {
        Placement9::TopLeft | Placement9::Top | Placement9::TopRight => inset,
        Placement9::BottomLeft | Placement9::Bottom | Placement9::BottomRight => -inset,
        Placement9::Left | Placement9::Center | Placement9::Right => px(0.),
    };
    point(x, y)
}

/// Desired layout for one widget, persisted in AppConfig.widgets_layout.
/// Fields are pub because this is a serialized record (schema-pinned).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PersistedWidget {
    pub kind: String,
    pub enabled: bool,
    pub placement: String,
    pub always_on_top: bool,
    pub click_through: bool,
    pub layer: String,
}

pub(crate) fn sanitize(raw: &[PersistedWidget]) -> Vec<PersistedWidget> {
    let mut out: Vec<PersistedWidget> = Vec::new();
    for entry in raw.iter().take(8) {
        if WidgetKind::from_str(&entry.kind).is_none() || out.iter().any(|p| p.kind == entry.kind) {
            continue;
        }
        let placement = placement_from_str(&entry.placement)
            .map(placement_to_str)
            .unwrap_or("center");
        let layer = if entry.layer == "pinned" {
            "pinned"
        } else {
            "above"
        };
        out.push(PersistedWidget {
            kind: entry.kind.clone(),
            enabled: entry.enabled,
            placement: placement.to_string(),
            always_on_top: entry.always_on_top,
            click_through: entry.click_through,
            layer: layer.to_string(),
        });
    }
    out
}

/// Sanitized desired layout from config (sanitize: known kinds only, one
/// entry per kind with first-wins, placement/layer normalized, cap 8).
pub fn widget_layout(cx: &App) -> Vec<PersistedWidget> {
    app_state::with_config(cx, |c| sanitize(&c.widgets_layout))
}

pub(crate) fn save_slot(entry: PersistedWidget, cx: &mut App) {
    app_state::update_config(cx, |config| {
        let v = &mut config.widgets_layout;
        v.retain(|p| p.kind != entry.kind);
        v.push(entry.clone());
    });
}

pub(crate) fn slot(kind: WidgetKind, cx: &App) -> Option<PersistedWidget> {
    widget_layout(cx)
        .into_iter()
        .find(|p| p.kind == kind.as_str())
}

#[cfg(test)]
#[path = "catalog.test.rs"]
mod catalog_test;
