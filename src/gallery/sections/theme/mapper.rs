//! Maps a `ThemeColor` serde key to the category and display name the color
//! viewer groups it under. Upstream ships the same bridge: `ThemeColor`
//! serializes snake_case while theme files use dot-notation keys.

pub(crate) struct ParsedKey {
    pub(crate) category: String,
    pub(crate) name: String,
    pub(crate) canonical_key: String,
}

/// Parses a theme key (either snake_case or dot-notation) into a logical
/// category and name; the canonical key is the dot-notation form used by
/// theme config files, so explicit-color lookups match either spelling.
pub(crate) fn parse_theme_key(key: &str) -> ParsedKey {
    if key.contains('.') {
        let parts: Vec<&str> = key.splitn(2, '.').collect();
        return ParsedKey {
            category: to_title_case_full(parts[0]),
            name: to_title_case_full(parts[1]),
            canonical_key: key.to_string(),
        };
    }

    // The flat ThemeColor struct loses the dot hierarchy, so this table
    // reconstructs it for the keys that carry one.
    let (category, name, canonical) = match key {
        // Accent
        "accent" => ("Accent", "Background", "accent.background"),
        "accent_foreground" => ("Accent", "Foreground", "accent.foreground"),

        // Primary
        "primary" => ("Primary", "Background", "primary.background"),
        "primary_active" => ("Primary", "Active Background", "primary.active.background"),
        "primary_foreground" => ("Primary", "Foreground", "primary.foreground"),
        "primary_hover" => ("Primary", "Hover Background", "primary.hover.background"),

        // Secondary
        "secondary" => ("Secondary", "Background", "secondary.background"),
        "secondary_active" => (
            "Secondary",
            "Active Background",
            "secondary.active.background",
        ),
        "secondary_foreground" => ("Secondary", "Foreground", "secondary.foreground"),
        "secondary_hover" => (
            "Secondary",
            "Hover Background",
            "secondary.hover.background",
        ),

        // Sidebar
        "sidebar" => ("Sidebar", "Background", "sidebar.background"),
        "sidebar_accent" => ("Sidebar", "Accent Background", "sidebar.accent.background"),
        "sidebar_accent_foreground" => {
            ("Sidebar", "Accent Foreground", "sidebar.accent.foreground")
        }
        "sidebar_border" => ("Sidebar", "Border", "sidebar.border"),
        "sidebar_foreground" => ("Sidebar", "Foreground", "sidebar.foreground"),
        "sidebar_primary" => (
            "Sidebar",
            "Primary Background",
            "sidebar.primary.background",
        ),
        "sidebar_primary_foreground" => (
            "Sidebar",
            "Primary Foreground",
            "sidebar.primary.foreground",
        ),

        // List
        "list" => ("List", "Background", "list.background"),
        "list_active" => ("List", "Active Background", "list.active.background"),
        "list_active_border" => ("List", "Active Border", "list.active.border"),
        "list_even" => ("List", "Even Background", "list.even.background"),
        "list_head" => ("List", "Head Background", "list.head.background"),
        "list_hover" => ("List", "Hover Background", "list.hover.background"),

        // Table
        "table" => ("Table", "Background", "table.background"),
        "table_active" => ("Table", "Active Background", "table.active.background"),
        "table_active_border" => ("Table", "Active Border", "table.active.border"),
        "table_even" => ("Table", "Even Background", "table.even.background"),
        "table_head" => ("Table", "Head Background", "table.head.background"),
        "table_head_foreground" => ("Table", "Head Foreground", "table.head.foreground"),
        "table_hover" => ("Table", "Hover Background", "table.hover.background"),
        "table_row_border" => ("Table", "Row Border", "table.row.border"),

        // Tabs
        "tab" => ("Tab", "Background", "tab.background"),
        "tab_active" => ("Tab", "Active Background", "tab.active.background"),
        "tab_active_foreground" => ("Tab", "Active Foreground", "tab.active.foreground"),
        "tab_bar" => ("Tab Bar", "Background", "tab_bar.background"),
        "tab_bar_segmented" => (
            "Tab Bar",
            "Segmented Background",
            "tab_bar.segmented.background",
        ),
        "tab_foreground" => ("Tab", "Foreground", "tab.foreground"),

        // Input
        "input" => ("Input", "Border", "input.border"),
        "caret" => ("Input", "Caret", "caret"),
        "selection" => ("Input", "Selection", "selection.background"),

        // Slider / Switch
        "slider_bar" => ("Slider", "Bar", "slider.background"),
        "slider_thumb" => ("Slider", "Thumb", "slider.thumb.background"),
        "switch" => ("Switch", "Background", "switch.background"),
        "switch_thumb" => ("Switch", "Thumb", "switch.thumb.background"),

        // Muted / Skeleton
        "muted" => ("Muted", "Background", "muted.background"),
        "muted_foreground" => ("Muted", "Foreground", "muted.foreground"),
        "skeleton" => ("Skeleton", "Background", "skeleton.background"),

        // Charts
        "chart_1" => ("Chart", "Color 1", "chart.1"),
        "chart_2" => ("Chart", "Color 2", "chart.2"),
        "chart_3" => ("Chart", "Color 3", "chart.3"),
        "chart_4" => ("Chart", "Color 4", "chart.4"),
        "chart_5" => ("Chart", "Color 5", "chart.5"),
        "chart_bullish" => ("Chart", "Bullish", "chart.bullish"),
        "chart_bearish" => ("Chart", "Bearish", "chart.bearish"),
        "chart_grid" => ("Chart", "Grid", "chart.grid"),

        // Danger / Success / Warning / Info
        "danger" => ("Danger", "Background", "danger.background"),
        "danger_active" => ("Danger", "Active", "danger.active.background"),
        "danger_foreground" => ("Danger", "Foreground", "danger.foreground"),
        "danger_hover" => ("Danger", "Hover", "danger.hover.background"),

        "success" => ("Success", "Background", "success.background"),
        "success_active" => ("Success", "Active", "success.active.background"),
        "success_foreground" => ("Success", "Foreground", "success.foreground"),
        "success_hover" => ("Success", "Hover", "success.hover.background"),

        "warning" => ("Warning", "Background", "warning.background"),
        "warning_active" => ("Warning", "Active", "warning.active.background"),
        "warning_foreground" => ("Warning", "Foreground", "warning.foreground"),
        "warning_hover" => ("Warning", "Hover", "warning.hover.background"),

        "info" => ("Info", "Background", "info.background"),
        "info_active" => ("Info", "Active", "info.active.background"),
        "info_foreground" => ("Info", "Foreground", "info.foreground"),
        "info_hover" => ("Info", "Hover", "info.hover.background"),

        // Base Colors
        "red" => ("Base", "Red", "base.red"),
        "red_light" => ("Base", "Red Light", "base.red.light"),
        "green" => ("Base", "Green", "base.green"),
        "green_light" => ("Base", "Green Light", "base.green.light"),
        "blue" => ("Base", "Blue", "base.blue"),
        "blue_light" => ("Base", "Blue Light", "base.blue.light"),
        "yellow" => ("Base", "Yellow", "base.yellow"),
        "yellow_light" => ("Base", "Yellow Light", "base.yellow.light"),
        "magenta" => ("Base", "Magenta", "base.magenta"),
        "magenta_light" => ("Base", "Magenta Light", "base.magenta.light"),
        "cyan" => ("Base", "Cyan", "base.cyan"),
        "cyan_light" => ("Base", "Cyan Light", "base.cyan.light"),

        // Everything else lands in Global, or splits on the first separator
        // when the key still carries one.
        _ => {
            if key.contains('_') {
                let parts: Vec<&str> = key.splitn(2, '_').collect();
                (parts[0], parts[1], key)
            } else {
                ("Global", key, key)
            }
        }
    };

    ParsedKey {
        category: to_title_case_full(category),
        name: to_title_case_full(name),
        canonical_key: canonical.to_string(),
    }
}

fn to_title_case(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

fn to_title_case_full(s: &str) -> String {
    s.split(['_', '.'])
        .map(to_title_case)
        .collect::<Vec<_>>()
        .join(" ")
}
