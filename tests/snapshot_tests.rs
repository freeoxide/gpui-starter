//! Snapshot tests (insta, inline snapshots) for core serialisable structures.
//! `cargo insta review` rewrites the inline bodies after intentional changes.

#[allow(dead_code)]
mod common;

use std::collections::HashSet;

use common::{manifest_path, read_file};
use gpui_starter::app_state::{AppConfig, PersistedWindowBounds, PersistedWindowState};
use gpui_starter::page::Page;
use gpui_starter::routes::AppRoute;
use gpui_starter::services::telemetry::TelemetryMode;
use gpui_starter::services::updater::{
    PlatformAsset, UpdateManifest, UpdateSnapshot, UpdateStatus,
};

#[test]
fn test_config_default_serialization() {
    let config = AppConfig::default();
    insta::assert_yaml_snapshot!(
            &config,
    @r#"
        version: 1
        theme: Default Light
        scrollbar_show: ~
        locale: ~
        active_route:
          Page: Home
        sidebar_collapsed: false
        native_notifications_enabled: true
        global_shortcut_enabled: true
        first_run_completed: false
        notification_inbox: []
        window_bounds: ~
        granted_permissions: []
        denied_permissions: []
        update_channel: stable
        last_update_check: ~
        show_frame_time: true
        floating_palettes: []
        mic_device_id: ~
        mic_gain_db: 0
        widgets_layout: []
        bluetooth_monitoring_enabled: false
        update_check_on_startup: false
        telemetry_mode: Disabled
        "#
        );
}

#[test]
fn test_config_roundtrip() {
    let original = AppConfig {
        version: 1,
        theme: "Gruvbox Dark".to_string(),
        scrollbar_show: None,
        locale: Some("en".to_string()),
        active_route: AppRoute::page(Page::Settings),
        sidebar_collapsed: true,
        native_notifications_enabled: false,
        global_shortcut_enabled: true,
        first_run_completed: true,
        notification_inbox: Vec::new(),
        window_bounds: Some(PersistedWindowBounds {
            x: 100.0,
            y: 200.0,
            width: 800.0,
            height: 600.0,
            display_id: Some("ab12cd34-56ef-4c78-9abc-def012345678".to_string()),
            state: PersistedWindowState::Maximized,
        }),
        granted_permissions: HashSet::from(["camera".to_string(), "microphone".to_string()]),
        denied_permissions: HashSet::new(),
        update_channel: "stable".to_string(),
        last_update_check: None,
        show_frame_time: false,
        floating_palettes: Vec::new(),
        mic_device_id: Some("coreaudio:Built-in Microphone".to_string()),
        mic_gain_db: 4.5,
        widgets_layout: Vec::new(),
        bluetooth_monitoring_enabled: false,
        update_check_on_startup: false,
        telemetry_mode: TelemetryMode::LocalOnly,
    };

    let json = serde_json::to_string(&original).expect("serialize config");
    let restored: AppConfig = serde_json::from_str(&json).expect("deserialize config");
    assert_eq!(original, restored, "round-tripped config must be identical");

    let mut snapshot = serde_json::to_value(&original).expect("config to value");
    for key in ["granted_permissions", "denied_permissions"] {
        if let Some(values) = snapshot.get_mut(key).and_then(|value| value.as_array_mut()) {
            values.sort_by(|left, right| left.as_str().cmp(&right.as_str()));
        }
    }
    insta::assert_yaml_snapshot!(
            &snapshot,
    @r#"
        version: 1
        theme: Gruvbox Dark
        scrollbar_show: ~
        locale: en
        active_route:
          Page: Settings
        sidebar_collapsed: true
        native_notifications_enabled: false
        global_shortcut_enabled: true
        first_run_completed: true
        notification_inbox: []
        window_bounds:
          x: 100
          y: 200
          width: 800
          height: 600
          display_id: ab12cd34-56ef-4c78-9abc-def012345678
          state: maximized
        granted_permissions:
          - camera
          - microphone
        denied_permissions: []
        update_channel: stable
        last_update_check: ~
        show_frame_time: false
        floating_palettes: []
        mic_device_id: "coreaudio:Built-in Microphone"
        mic_gain_db: 4.5
        widgets_layout: []
        bluetooth_monitoring_enabled: false
        update_check_on_startup: false
        telemetry_mode: LocalOnly
        "#
        );
}

#[test]
fn test_config_minimal_v1_json_deserializes_with_defaults() {
    // A v1 file written before any serde(default) field existed; later fields
    // must fill from their serde defaults instead of quarantining the file.
    let json = r#"{
        "version": 1,
        "theme": "Gruvbox Dark",
        "scrollbar_show": null,
        "locale": "en",
        "active_route": { "Page": "Settings" },
        "sidebar_collapsed": true,
        "native_notifications_enabled": false,
        "first_run_completed": true,
        "notification_inbox": [],
        "window_bounds": null
    }"#;
    let config: AppConfig = serde_json::from_str(json).expect("minimal v1 config must deserialize");
    let expected = AppConfig {
        theme: "Gruvbox Dark".to_string(),
        locale: Some("en".to_string()),
        active_route: AppRoute::page(Page::Settings),
        sidebar_collapsed: true,
        native_notifications_enabled: false,
        first_run_completed: true,
        ..AppConfig::default()
    };
    assert_eq!(
        config, expected,
        "later fields must load their documented defaults"
    );
}

#[test]
fn test_window_bounds_old_schema_deserializes_with_defaults() {
    let json = r#"{"x":100.0,"y":200.0,"width":800.0,"height":600.0}"#;
    let bounds: PersistedWindowBounds = serde_json::from_str(json).expect("old-schema bounds");
    assert_eq!(bounds.display_id, None);
    assert_eq!(bounds.state, PersistedWindowState::Windowed);
}

#[test]
fn test_theme_structure() {
    let theme_path = manifest_path("themes/gruvbox.json");
    let raw = read_file(&theme_path);
    let value: serde_json::Value =
        serde_json::from_str(&raw).expect("theme file must be valid JSON");

    // Names and counts only, so color-value tweaks don't churn the snapshot.
    #[derive(serde::Serialize)]
    struct ThemeSummary {
        name: String,
        author: String,
        theme_count: usize,
        theme_names: Vec<String>,
        theme_modes: Vec<String>,
        color_key_count_per_theme: Vec<usize>,
        highlight_key_count_per_theme: Vec<usize>,
    }

    let themes = value.get("themes").and_then(|t| t.as_array()).unwrap();
    let summary = ThemeSummary {
        name: value
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string(),
        author: value
            .get("author")
            .and_then(|v| v.as_str())
            .unwrap()
            .to_string(),
        theme_count: themes.len(),
        theme_names: themes
            .iter()
            .map(|t| t.get("name").and_then(|v| v.as_str()).unwrap().to_string())
            .collect(),
        theme_modes: themes
            .iter()
            .map(|t| t.get("mode").and_then(|v| v.as_str()).unwrap().to_string())
            .collect(),
        color_key_count_per_theme: themes
            .iter()
            .map(|t| {
                t.get("colors")
                    .and_then(|c| c.as_object())
                    .map(|o| o.len())
                    .unwrap_or(0)
            })
            .collect(),
        highlight_key_count_per_theme: themes
            .iter()
            .map(|t| {
                t.get("highlight")
                    .and_then(|h| h.as_object())
                    .map(|o| o.len())
                    .unwrap_or(0)
            })
            .collect(),
    };

    insta::assert_yaml_snapshot!(
            &summary,
    @r#"
        name: Gruvbox
        author: Pavel Pertsev
        theme_count: 2
        theme_names:
          - Gruvbox Light
          - Gruvbox Dark
        theme_modes:
          - light
          - dark
        color_key_count_per_theme:
          - 45
          - 41
        highlight_key_count_per_theme:
          - 21
          - 21
        "#
        );
}

#[test]
fn test_update_manifest_serialization() {
    let mut platforms = std::collections::HashMap::new();
    platforms.insert(
        "macos-aarch64".to_string(),
        PlatformAsset {
            url: "https://releases.example.com/app-2.0.0.tar.gz".to_string(),
            signature: "deadbeef".to_string(),
            size: 42_000_000,
        },
    );
    platforms.insert(
        "linux-x86_64".to_string(),
        PlatformAsset {
            url: "https://releases.example.com/app-2.0.0-linux.tar.gz".to_string(),
            signature: "cafebabe".to_string(),
            size: 39_000_000,
        },
    );
    let manifest = UpdateManifest {
        version: "2.0.0".to_string(),
        release_notes: "Major release with new features.".to_string(),
        platforms,
    };

    // HashMap iteration order varies per process; sorted maps keep the
    // snapshot stable no matter how many platforms the manifest carries.
    insta::with_settings!({sort_maps => true}, {
        insta::assert_yaml_snapshot!(
                &manifest,
        @r#"
        version: 2.0.0
        release_notes: Major release with new features.
        platforms:
          linux-x86_64:
            url: "https://releases.example.com/app-2.0.0-linux.tar.gz"
            signature: cafebabe
            size: 39000000
          macos-aarch64:
            url: "https://releases.example.com/app-2.0.0.tar.gz"
            signature: deadbeef
            size: 42000000
        "#
        );
    });
}

#[test]
fn test_update_status_serialization() {
    #[derive(serde::Serialize)]
    struct StatusVariants {
        idle: UpdateStatus,
        checking: UpdateStatus,
        up_to_date: UpdateStatus,
        available: UpdateStatus,
        error: UpdateStatus,
    }

    let variants = StatusVariants {
        idle: UpdateStatus::Idle,
        checking: UpdateStatus::Checking,
        up_to_date: UpdateStatus::UpToDate,
        available: UpdateStatus::Available {
            version: "2.0.0".to_string(),
            notes: "Bug fixes".to_string(),
        },
        error: UpdateStatus::Error("network timeout".to_string()),
    };

    insta::assert_yaml_snapshot!(
            &variants,
    @r#"
        idle: Idle
        checking: Checking
        up_to_date: UpToDate
        available:
          Available:
            version: 2.0.0
            notes: Bug fixes
        error:
          Error: network timeout
        "#
        );
}

#[test]
fn test_update_snapshot_serialization() {
    let snapshot = UpdateSnapshot {
        status: UpdateStatus::Available {
            version: "2.1.0".to_string(),
            notes: "New features".to_string(),
        },
        current_version: "2.0.0".to_string(),
        last_check: Some("2026-06-04T12:00:00Z".to_string()),
        update_channel: "stable".to_string(),
        check_retry_count: 0,
        cached_manifest: None,
    };

    insta::assert_yaml_snapshot!(
            &snapshot,
    @r#"
        status:
          Available:
            version: 2.1.0
            notes: New features
        current_version: 2.0.0
        last_check: "2026-06-04T12:00:00Z"
        update_channel: stable
        check_retry_count: 0
        "#
        );
}
