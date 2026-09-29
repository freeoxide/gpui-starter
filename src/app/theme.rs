use gpui::App;
use gpui_component::ActiveTheme as _;

/// Runs unconditionally at startup, and again after a watcher reload (which
/// clears the registry); name-dedup keeps already-registered dev themes winning.
pub fn register_embedded_themes(cx: &mut App) {
    let registry = gpui_component::ThemeRegistry::global_mut(cx);
    for (name, json) in crate::app::assets::embedded_themes() {
        if let Err(err) = registry.load_themes_from_str(&json) {
            tracing::warn!("invalid embedded theme {name}: {err}");
        }
    }
}

/// Restore embedded themes a kit watcher reload dropped: kit `reload()` clears
/// the registry and only re-reads `themes_dir`, so anything not on disk
/// (the embedded set) vanishes on every watcher event, file deletion included.
/// No-op when nothing is missing.
pub fn ensure_embedded_themes(cx: &mut App) {
    // global_mut notifies observers even when nothing changes, so check
    // first: mutating from the observer that called us must terminate.
    if embedded_files_missing(gpui_component::ThemeRegistry::global(cx)) == 0 {
        return;
    }
    restore_embedded_themes(gpui_component::ThemeRegistry::global_mut(cx));
}

/// Load every embedded theme file carrying at least one name missing from
/// `registry`; returns how many files were (re)loaded.
fn restore_embedded_themes(registry: &mut gpui_component::ThemeRegistry) -> usize {
    let dropped: Vec<_> = crate::app::assets::embedded_themes()
        .into_iter()
        .filter(|(file, json)| file_has_missing_theme(file, json, registry))
        .collect();
    let count = dropped.len();
    for (file, json) in &dropped {
        if let Err(err) = registry.load_themes_from_str(json) {
            tracing::warn!("invalid embedded theme {file}: {err}");
        }
    }
    count
}

/// Number of embedded theme files whose parsed theme names are not all
/// present in `registry`.
fn embedded_files_missing(registry: &gpui_component::ThemeRegistry) -> usize {
    crate::app::assets::embedded_themes()
        .iter()
        .filter(|(file, json)| file_has_missing_theme(file, json, registry))
        .count()
}

fn file_has_missing_theme(
    file: &str,
    json: &str,
    registry: &gpui_component::ThemeRegistry,
) -> bool {
    match serde_json::from_str::<gpui_component::theme::ThemeSet>(json) {
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

pub fn set_theme_mode(mode: gpui_component::ThemeMode, cx: &mut App) {
    set_theme_mode_with_record(mode, true, cx);
}

pub fn set_theme_mode_with_record(mode: gpui_component::ThemeMode, record: bool, cx: &mut App) {
    let before = cx.theme().mode;
    // change (not update) re-applies the mode's registered theme even when
    // the mode is unchanged, and refreshes all windows itself in kit 0.7.
    gpui_component::Theme::change(mode, None, cx);
    if record {
        crate::undo_stack::record_theme_mode_change(before, mode, cx);
    }
}

#[cfg(test)]
#[path = "theme.test.rs"]
mod theme_test;
