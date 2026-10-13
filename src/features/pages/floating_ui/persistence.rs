use std::collections::BTreeMap;

use gpui_kit::App;
use serde::{Deserialize, Serialize};

use super::state::PaletteId;

/// Matches `MAX_PLAUSIBLE_DIM` in the config store.
const MAX_PLAUSIBLE_DIM: f32 = 100_000.0;
const MAX_ENTRIES: usize = 16;

/// One persisted palette slot inside `AppConfig.floating_palettes`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PersistedPalette {
    pub id: String,
    pub open: bool,
    pub surface: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: Option<String>,
    pub note: Option<String>,
}

pub(crate) fn sanitize(raw: &[PersistedPalette]) -> BTreeMap<PaletteId, PersistedPalette> {
    let mut out: BTreeMap<PaletteId, PersistedPalette> = BTreeMap::new();
    for entry in raw.iter().take(MAX_ENTRIES) {
        let Some(id) = PaletteId::from_slug(&entry.id) else {
            continue;
        };
        if out.contains_key(&id) {
            continue;
        }
        if !entry.x.is_finite()
            || !entry.y.is_finite()
            || !entry.width.is_finite()
            || !entry.height.is_finite()
        {
            continue;
        }
        if entry.width <= 0.0
            || entry.height <= 0.0
            || entry.width > MAX_PLAUSIBLE_DIM
            || entry.height > MAX_PLAUSIBLE_DIM
        {
            continue;
        }
        let mut entry = entry.clone();
        if entry.surface != "window" {
            entry.surface = "panel".to_string();
        }
        out.insert(id, entry);
    }
    out
}

/// Sanitized slot for `id`, or `None` when it was never persisted.
pub fn slot(id: PaletteId, cx: &App) -> Option<PersistedPalette> {
    crate::app_state::with_config(cx, |config| {
        sanitize(&config.floating_palettes).get(&id).cloned()
    })
}

/// Replace `id`'s persisted slot. The one writer shape for this feature.
pub fn save_slot(entry: PersistedPalette, cx: &mut App) {
    crate::app_state::update_config(cx, |config| {
        let id = entry.id.clone();
        let v = &mut config.floating_palettes;
        v.retain(|p| p.id != id);
        v.push(entry.clone());
    });
}

/// Read-or-seed `id`'s slot, apply `f`, persist. Callers that only own some
/// fields must go through this so they cannot clobber the others.
pub(crate) fn update_slot(
    id: PaletteId,
    cx: &mut App,
    f: impl FnOnce(&mut PersistedPalette),
) -> PersistedPalette {
    let mut entry = slot(id, cx).unwrap_or_else(|| default_entry(id));
    f(&mut entry);
    save_slot(entry.clone(), cx);
    entry
}

pub(crate) fn default_entry(id: PaletteId) -> PersistedPalette {
    let size = id.default_size();
    PersistedPalette {
        id: id.slug().to_string(),
        open: false,
        surface: "panel".to_string(),
        x: 16.0,
        y: 16.0,
        width: f32::from(size.width),
        height: f32::from(size.height),
        color: None,
        note: None,
    }
}

#[cfg(test)]
#[path = "persistence.test.rs"]
mod persistence_test;
