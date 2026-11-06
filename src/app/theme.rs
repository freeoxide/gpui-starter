use gpui_kit::component::ActiveTheme as _;
use gpui_kit::{App, SharedString};

/// Runs unconditionally at startup, and again after a watcher reload (which
/// clears the registry); name-dedup keeps already-registered dev themes winning.
pub fn register_embedded_themes(cx: &mut App) {
    let registry = gpui_kit::component::ThemeRegistry::global_mut(cx);
    for (name, json) in crate::app::assets::embedded_themes() {
        if let Err(err) = registry.load_themes_from_str(&json) {
            tracing::warn!("invalid embedded theme {name}: {err}");
        }
    }
}

/// Re-register embedded themes a kit watcher reload dropped: `reload()` clears
/// the registry and re-reads only `themes_dir`, so the embedded set vanishes on
/// every watcher event. No-op when nothing is missing.
pub fn ensure_embedded_themes(cx: &mut App) {
    // global_mut notifies observers even when nothing changes; only take it
    // when a file is missing, or the observer that called us never terminates.
    let dropped = dropped_embedded_themes(gpui_kit::component::ThemeRegistry::global(cx));
    if dropped.is_empty() {
        return;
    }
    restore_embedded_themes(gpui_kit::component::ThemeRegistry::global_mut(cx), &dropped);
}

/// Embedded theme files carrying at least one theme name absent from `registry`.
fn dropped_embedded_themes(
    registry: &gpui_kit::component::ThemeRegistry,
) -> Vec<(SharedString, String)> {
    crate::app::assets::embedded_themes()
        .into_iter()
        .filter(|(file, json)| file_has_missing_theme(file, json, registry))
        .collect()
}

fn restore_embedded_themes(
    registry: &mut gpui_kit::component::ThemeRegistry,
    dropped: &[(SharedString, String)],
) {
    for (file, json) in dropped {
        if let Err(err) = registry.load_themes_from_str(json) {
            tracing::warn!("invalid embedded theme {file}: {err}");
        }
    }
}

fn file_has_missing_theme(
    file: &str,
    json: &str,
    registry: &gpui_kit::component::ThemeRegistry,
) -> bool {
    match serde_json::from_str::<gpui_kit::component::theme::ThemeSet>(json) {
        Ok(set) => set
            .themes
            .iter()
            .any(|theme| !registry.themes().contains_key(&theme.name)),
        Err(err) => {
            tracing::warn!("invalid embedded theme {file}: {err}");
            false
        }
    }
}

pub fn set_theme_mode(mode: gpui_kit::component::ThemeMode, cx: &mut App) {
    set_theme_mode_with_record(mode, true, cx);
}

pub fn set_theme_mode_with_record(
    mode: gpui_kit::component::ThemeMode,
    record: bool,
    cx: &mut App,
) {
    let before = cx.theme().mode;
    // change (not update) re-applies the mode's registered theme even when
    // the mode is unchanged, and refreshes all windows itself in kit 0.7.
    gpui_kit::component::Theme::change(mode, None, cx);
    if record {
        crate::undo_stack::record_theme_mode_change(before, mode, cx);
    }
}

#[cfg(test)]
#[path = "theme.test.rs"]
mod theme_test;
