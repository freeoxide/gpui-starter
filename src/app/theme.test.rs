use std::rc::Rc;

use gpui_kit::SharedString;

// No App context: gpui test-support is a dev-dependency change, so drive
// load_themes_from_str and Theme::apply_config (the init.rs code paths).

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
    let restored = super::restore_embedded_themes(&mut registry);
    assert_eq!(restored, crate::app::assets::embedded_themes().len());
    for (file, names) in embedded_theme_names() {
        for name in names {
            assert!(
                registry.themes().contains_key(&name),
                "{file}: {name} missing after restore"
            );
        }
    }
    assert_eq!(
        super::restore_embedded_themes(&mut registry),
        0,
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

    assert_eq!(super::restore_embedded_themes(&mut registry), tail.len());
    for (file, names) in embedded_theme_names() {
        for name in names {
            assert!(
                registry.themes().contains_key(&name),
                "{file}: {name} missing after partial restore"
            );
        }
    }
}
