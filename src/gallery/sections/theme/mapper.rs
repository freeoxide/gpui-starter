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

    // The flat ThemeColor struct loses the dot hierarchy; this table restores
    // it from ThemeConfigColors serde renames, which no rule derives
    // (title_bar keeps its underscore, progress_bar does not).
    let (category, name, canonical) = match key {
        // Accordion
        "accordion" => ("Accordion", "Background", "accordion.background"),

        // Accent
        "accent" => ("Accent", "Background", "accent.background"),
        "accent_foreground" => ("Accent", "Foreground", "accent.foreground"),

        // Button
        "button" => ("Button", "Background", "button.background"),
        "button_active" => ("Button", "Active Background", "button.active.background"),
        "button_foreground" => ("Button", "Foreground", "button.foreground"),
        "button_hover" => ("Button", "Hover Background", "button.hover.background"),
        "button_danger" => ("Button", "Danger Background", "button.danger.background"),
        "button_danger_active" => (
            "Button",
            "Danger Active Background",
            "button.danger.active.background",
        ),
        "button_danger_foreground" => ("Button", "Danger Foreground", "button.danger.foreground"),
        "button_danger_hover" => (
            "Button",
            "Danger Hover Background",
            "button.danger.hover.background",
        ),
        "button_info" => ("Button", "Info Background", "button.info.background"),
        "button_info_active" => (
            "Button",
            "Info Active Background",
            "button.info.active.background",
        ),
        "button_info_foreground" => ("Button", "Info Foreground", "button.info.foreground"),
        "button_info_hover" => (
            "Button",
            "Info Hover Background",
            "button.info.hover.background",
        ),
        "button_primary" => ("Button", "Primary Background", "button.primary.background"),
        "button_primary_active" => (
            "Button",
            "Primary Active Background",
            "button.primary.active.background",
        ),
        "button_primary_foreground" => {
            ("Button", "Primary Foreground", "button.primary.foreground")
        }
        "button_primary_hover" => (
            "Button",
            "Primary Hover Background",
            "button.primary.hover.background",
        ),
        "button_secondary" => (
            "Button",
            "Secondary Background",
            "button.secondary.background",
        ),
        "button_secondary_active" => (
            "Button",
            "Secondary Active Background",
            "button.secondary.active.background",
        ),
        "button_secondary_foreground" => (
            "Button",
            "Secondary Foreground",
            "button.secondary.foreground",
        ),
        "button_secondary_hover" => (
            "Button",
            "Secondary Hover Background",
            "button.secondary.hover.background",
        ),
        "button_success" => ("Button", "Success Background", "button.success.background"),
        "button_success_active" => (
            "Button",
            "Success Active Background",
            "button.success.active.background",
        ),
        "button_success_foreground" => {
            ("Button", "Success Foreground", "button.success.foreground")
        }
        "button_success_hover" => (
            "Button",
            "Success Hover Background",
            "button.success.hover.background",
        ),
        "button_warning" => ("Button", "Warning Background", "button.warning.background"),
        "button_warning_active" => (
            "Button",
            "Warning Active Background",
            "button.warning.active.background",
        ),
        "button_warning_foreground" => {
            ("Button", "Warning Foreground", "button.warning.foreground")
        }
        "button_warning_hover" => (
            "Button",
            "Warning Hover Background",
            "button.warning.hover.background",
        ),

        // Description List
        "description_list_label" => (
            "Description List",
            "Label Background",
            "description_list.label.background",
        ),
        "description_list_label_foreground" => (
            "Description List",
            "Label Foreground",
            "description_list.label.foreground",
        ),

        // Drag / Drop Target
        "drag_border" => ("Drag", "Border", "drag.border"),
        "drop_target" => ("Drop Target", "Background", "drop_target.background"),

        // Group Box
        "group_box" => ("Group Box", "Background", "group_box.background"),
        "group_box_foreground" => ("Group Box", "Foreground", "group_box.foreground"),

        // Link
        "link" => ("Link", "Color", "link"),
        "link_active" => ("Link", "Active", "link.active"),
        "link_hover" => ("Link", "Hover", "link.hover"),

        // Popover / Progress
        "popover" => ("Popover", "Background", "popover.background"),
        "popover_foreground" => ("Popover", "Foreground", "popover.foreground"),
        "progress_bar" => ("Progress", "Bar Background", "progress.bar.background"),

        // Scrollbar
        "scrollbar" => ("Scrollbar", "Background", "scrollbar.background"),
        "scrollbar_thumb" => (
            "Scrollbar",
            "Thumb Background",
            "scrollbar.thumb.background",
        ),
        "scrollbar_thumb_hover" => (
            "Scrollbar",
            "Thumb Hover Background",
            "scrollbar.thumb.hover.background",
        ),

        // Status Bar / Title Bar / Window
        "status_bar" => ("Status Bar", "Background", "status_bar.background"),
        "status_bar_border" => ("Status Bar", "Border", "status_bar.border"),
        "title_bar" => ("Title Bar", "Background", "title_bar.background"),
        "title_bar_border" => ("Title Bar", "Border", "title_bar.border"),
        "window_border" => ("Window", "Border", "window.border"),

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
        "table_foot" => ("Table", "Foot Background", "table.foot.background"),
        "table_foot_foreground" => ("Table", "Foot Foreground", "table.foot.foreground"),
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

#[cfg(test)]
#[path = "mapper.test.rs"]
mod mapper_test;
