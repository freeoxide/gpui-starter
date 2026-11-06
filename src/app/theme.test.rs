use std::collections::HashSet;
use std::rc::Rc;

use gpui_kit::SharedString;

/// (file, theme names) of every theme inside every embedded themes/*.json.
fn embedded_theme_names() -> Vec<(SharedString, Vec<SharedString>)> {
    crate::app::assets::embedded_themes()
        .into_iter()
        .map(|(file, json)| {
            let names = serde_json::from_str::<gpui_kit::component::theme::ThemeSet>(&json)
                .unwrap_or_else(|err| panic!("{file}: {err}"))
                .themes
                .iter()
                .map(|theme| theme.name.clone())
                .collect::<Vec<_>>();
            (file, names)
        })
        .collect()
}

#[test]
fn embedded_theme_file_count_is_24() {
    // Tripwire: coverage in the tests below derives from the embed iterator;
    // this pins the shipped count so a dropped themes/*.json cannot slide by.
    assert_eq!(crate::app::assets::embedded_themes().len(), 24);
}

/// kit 0.7 adds `chart.grid` (ThemeColor::chart_grid); every embedded theme
/// must resolve a grid color through apply_config, with or without the token.
#[test]
fn apply_config_resolves_chart_grid_for_every_embedded_theme() {
    for (file, json) in crate::app::assets::embedded_themes() {
        let set = serde_json::from_str::<gpui_kit::component::theme::ThemeSet>(&json)
            .unwrap_or_else(|err| panic!("{file}: {err}"));
        for config in set.themes {
            let name = config.name.clone();
            let explicit = config.colors.chart_grid.clone();
            let mut theme = gpui_kit::component::Theme::default();
            theme.apply_config(&Rc::new(config));

            let expected = match explicit.as_deref() {
                // 0.6 dropped this key silently; 0.7 parses it natively.
                Some(value) => gpui_kit::component::theme::try_parse_color(value)
                    .unwrap_or_else(|err| panic!("{file} {name}: {value}: {err}")),
                // The documented 0.7 fallback for themes without chart.grid.
                None => theme.border.opacity(0.6),
            };
            assert_eq!(theme.chart_grid, expected, "{file}: {name}");
        }
    }
}

// Kit apply_config derives every chart/danger/success/warning/info/sidebar
// fallback from these keys; miss one and the family renders as kit Default.
const SEMANTIC_FALLBACK_FEEDS: &[&str] = &[
    "background",
    "border",
    "foreground",
    "primary.background",
    "secondary.background",
    "base.red",
    "base.green",
    "base.yellow",
    "base.blue",
    "base.cyan",
];

#[test]
fn every_embedded_theme_sets_the_palette_semantic_families_derive_from() {
    for (file, json) in crate::app::assets::embedded_themes() {
        let set = serde_json::from_str::<serde_json::Value>(&json)
            .unwrap_or_else(|err| panic!("{file}: {err}"));
        let themes = set
            .get("themes")
            .and_then(|themes| themes.as_array())
            .unwrap_or_else(|| panic!("{file}: no themes array"));
        for theme in themes {
            let name = theme
                .get("name")
                .and_then(|name| name.as_str())
                .unwrap_or("");
            let colors = theme
                .get("colors")
                .and_then(|colors| colors.as_object())
                .unwrap_or_else(|| panic!("{file} {name}: no colors object"));
            for key in SEMANTIC_FALLBACK_FEEDS {
                assert!(
                    colors.contains_key(*key),
                    "{file} {name}: {key} unset, chart/danger/success/warning/info/sidebar fall back to the kit Default palette"
                );
            }
        }
    }
}

#[test]
fn every_embedded_theme_registers_and_resolves_by_name() {
    let mut registry = gpui_kit::component::ThemeRegistry::default();
    for (file, json) in crate::app::assets::embedded_themes() {
        registry
            .load_themes_from_str(&json)
            .unwrap_or_else(|err| panic!("{file}: {err}"));
    }

    let mut expected = 0;
    for (file, names) in embedded_theme_names() {
        assert!(!names.is_empty(), "{file} carries no themes");
        expected += names.len();
        for name in names {
            assert!(
                registry.themes().contains_key(&name),
                "{file}: {name} missing from registry"
            );
        }
    }
    assert_eq!(
        registry.themes().len(),
        expected,
        "a duplicate theme name made load_themes_from_str silently skip it"
    );
}

#[test]
fn embedded_themes_do_not_collide_with_registry_defaults() {
    // load_themes_from_str skips already-present names; the real registry
    // boots with these two crate defaults, so an embedded name must differ.
    for (file, names) in embedded_theme_names() {
        for name in names {
            assert!(
                !matches!(name.as_str(), "Default Light" | "Default Dark"),
                "{file}: {name} collides with a registry default and would be skipped"
            );
        }
    }
}

#[test]
fn restore_reloads_every_embedded_file_into_a_cleared_registry() {
    // A kit watcher reload clears `themes` back to defaults + dir contents;
    // a bare registry is the post-clear state with an empty themes/ dir.
    let mut registry = gpui_kit::component::ThemeRegistry::default();
    let dropped = super::dropped_embedded_themes(&registry);
    assert_eq!(dropped.len(), crate::app::assets::embedded_themes().len());
    super::restore_embedded_themes(&mut registry, &dropped);
    for (file, names) in embedded_theme_names() {
        for name in names {
            assert!(
                registry.themes().contains_key(&name),
                "{file}: {name} missing after restore"
            );
        }
    }
    assert!(
        super::dropped_embedded_themes(&registry).is_empty(),
        "a full registry must not be touched again"
    );
}

#[test]
fn restore_reloads_only_files_with_missing_themes() {
    let mut registry = gpui_kit::component::ThemeRegistry::default();
    let files = crate::app::assets::embedded_themes();
    let (head, tail) = files.split_at(files.len() / 2);
    for (file, json) in head {
        registry
            .load_themes_from_str(json)
            .unwrap_or_else(|err| panic!("{file}: {err}"));
    }

    let dropped = super::dropped_embedded_themes(&registry);
    assert_eq!(dropped.len(), tail.len());
    super::restore_embedded_themes(&mut registry, &dropped);
    for (file, names) in embedded_theme_names() {
        for name in names {
            assert!(
                registry.themes().contains_key(&name),
                "{file}: {name} missing after partial restore"
            );
        }
    }
}

/// Serde drops unknown `colors` keys and apply_config silently falls back
/// to the default color, so a bad key or value in a shipped theme never shows.
#[test]
fn every_embedded_theme_color_is_a_known_key_and_parses() {
    let default_config = gpui_kit::component::theme::ThemeConfigColors::default();
    let known: HashSet<String> = serde_json::to_value(default_config)
        .unwrap()
        .as_object()
        .unwrap()
        .keys()
        .cloned()
        .collect();
    assert!(!known.is_empty(), "an empty key set would pass every file");

    for (file, json) in crate::app::assets::embedded_themes() {
        let set = serde_json::from_str::<serde_json::Value>(&json)
            .unwrap_or_else(|err| panic!("{file}: {err}"));
        let themes = set
            .get("themes")
            .and_then(|themes| themes.as_array())
            .unwrap_or_else(|| panic!("{file}: no themes array"));
        for theme in themes {
            let name = theme
                .get("name")
                .and_then(|name| name.as_str())
                .unwrap_or("");
            let colors = theme
                .get("colors")
                .and_then(|colors| colors.as_object())
                .unwrap_or_else(|| panic!("{file} {name}: no colors object"));
            for (key, value) in colors {
                assert!(
                    known.contains(key),
                    "{file} {name}: colors key {key} matches no ThemeConfigColors field"
                );
                let color = value
                    .as_str()
                    .unwrap_or_else(|| panic!("{file} {name}: {key} is not a string"));
                assert!(
                    gpui_kit::component::theme::try_parse_color(color).is_ok(),
                    "{file} {name}: {key} {color} is not a color"
                );
            }
        }
    }
}
