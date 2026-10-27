use gpui_kit::{App, Global, Pixels, SharedString, Size, px, rgba, size};
use serde::{Deserialize, Serialize};

use crate::ui::floating::FloatingWindowId;

/// Stable palette identity; serde lower-case slug ("color" | "assets" | "note").
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PaletteId {
    Color,
    Assets,
    Note,
}

impl PaletteId {
    pub const ALL: &'static [PaletteId] = &[PaletteId::Color, PaletteId::Assets, PaletteId::Note];

    pub fn slug(self) -> &'static str {
        match self {
            PaletteId::Color => "color",
            PaletteId::Assets => "assets",
            PaletteId::Note => "note",
        }
    }

    pub fn from_slug(slug: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|id| id.slug() == slug)
    }

    /// Plan 02 window id, e.g. "floating-ui-color".
    pub fn window_id(self) -> FloatingWindowId {
        FloatingWindowId(format!("floating-ui-{}", self.slug()).into())
    }

    /// Localized palette title (catalog key per id).
    pub fn localized_title(self) -> String {
        crate::i18n::localize(match self {
            PaletteId::Color => "floating_ui_palette_color",
            PaletteId::Assets => "floating_ui_palette_assets",
            PaletteId::Note => "floating_ui_palette_note",
        })
    }

    /// Default pane/window size per palette (logical px).
    pub fn default_size(self) -> Size<Pixels> {
        match self {
            PaletteId::Color => size(px(260.), px(320.)),
            PaletteId::Assets => size(px(300.), px(260.)),
            PaletteId::Note => size(px(260.), px(220.)),
        }
    }
}

/// Swatch colors are data, not theme tokens: the value IS the content.
pub(crate) const COLOR_PRESETS: &[(&str, u32)] = &[
    ("red", 0xE5484DFF),
    ("orange", 0xF76B15FF),
    ("amber", 0xFFB224FF),
    ("yellow", 0xF2F23FFF),
    ("green", 0x30A46CFF),
    ("teal", 0x12A594FF),
    ("blue", 0x0090FFFF),
    ("indigo", 0x3E63DDFF),
    ("purple", 0x8E4EC6FF),
    ("pink", 0xD6409FFF),
    ("black", 0x000000FF),
    ("white", 0xFFFFFFFF),
];

pub(crate) const ASSET_PRESETS: &[&str] = &[
    "🚀", "🎨", "📌", "✅", "⭐", "🔥", "💡", "🧪", "📦", "🗂️", "🔒", "⏱️", "📊", "🧭", "✏️", "🗑️",
    "👀", "🎯", "⚙️", "🔗", "💬", "🏷️", "📍", "🧩",
];

/// Materialized cross-window palette state. Written only by [`apply_change`]
/// and seeded by [`init`](super::init); read by observers in any window.
#[derive(Clone, Debug, Default)]
pub struct PaletteSyncState {
    pub color_hex: String,
    pub last_asset: Option<SharedString>,
}

impl Global for PaletteSyncState {}

/// Interpret a drained `AppEventKind::PaletteChanged`; called from AppRoot only.
/// Unknown palette slug is a logged no-op.
pub fn apply_change(palette: &str, value: &str, cx: &mut App) {
    let Some(id) = PaletteId::from_slug(palette) else {
        tracing::warn!(
            target: "gpui_starter::floating_ui",
            palette,
            "palette change for unknown palette slug"
        );
        return;
    };
    let mut next = cx
        .try_global::<PaletteSyncState>()
        .cloned()
        .unwrap_or_default();
    match id {
        PaletteId::Color => next.color_hex = value.to_string(),
        PaletteId::Assets => next.last_asset = Some(value.into()),
        PaletteId::Note => {
            tracing::debug!(
                target: "gpui_starter::floating_ui",
                "note palette carries no cross-window state"
            );
        }
    }
    cx.set_global(next);
}

/// "#RRGGBB"/"#RRGGBBAA" -> Rgba; None on malformed input.
pub(crate) fn hex_to_rgba(hex: &str) -> Option<gpui_kit::Rgba> {
    let digits = hex.strip_prefix('#')?;
    if digits.len() != 6 && digits.len() != 8 {
        return None;
    }
    let value = u32::from_str_radix(digits, 16).ok()?;
    if digits.len() == 6 {
        let [_, r, g, b] = value.to_be_bytes().map(|b| (b as f32) / 255.0);
        return Some(gpui_kit::Rgba { r, g, b, a: 1.0 });
    }
    Some(rgba(value))
}

/// Rgba -> "#RRGGBB"; alpha below 1.0 rounds into the channels it covers.
pub(crate) fn rgba_to_hex(color: gpui_kit::Rgba) -> String {
    let channel = |c: f32| (c * 255.).round().clamp(0., 255.) as u8;
    format!(
        "#{:02X}{:02X}{:02X}",
        channel(color.r),
        channel(color.g),
        channel(color.b)
    )
}

#[cfg(test)]
#[path = "state.test.rs"]
mod state_test;
