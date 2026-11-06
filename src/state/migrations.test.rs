use super::*;

#[test]
fn migrates_legacy_config_to_current_version() {
    let legacy = AppConfig {
        version: 0,
        ..AppConfig::default()
    };
    let migrated = migrate(legacy);
    assert_eq!(migrated.version, APP_STATE_VERSION);
}

#[test]
fn migration_keeps_an_explicitly_disabled_global_shortcut() {
    // A missing key already defaults on; an explicit false is a user choice.
    let legacy = AppConfig {
        version: 0,
        global_shortcut_enabled: false,
        ..AppConfig::default()
    };
    let migrated = migrate(legacy);
    assert!(!migrated.global_shortcut_enabled);
}

#[test]
fn minimal_v0_state_file_loads_and_migrates() {
    // Only the keys a real v0 file carried; every post-v0 field must stay
    // absent so the serde defaults alone carry the load.
    let legacy_v0 = r#"{
        "version": 0,
        "theme": "Gruvbox Dark",
        "active_route": { "Page": "Settings" },
        "sidebar_collapsed": true,
        "native_notifications_enabled": true,
        "first_run_completed": true,
        "notification_inbox": []
    }"#;
    let config: AppConfig = serde_json::from_str(legacy_v0)
        .expect("a v0 state file must deserialize through the serde defaults");
    let migrated = migrate(config);
    assert_eq!(migrated.version, APP_STATE_VERSION);
    assert_eq!(migrated.theme, "Gruvbox Dark");
    assert!(migrated.global_shortcut_enabled);
}

#[test]
fn migrate_never_stamps_a_newer_version_down() {
    let newer = AppConfig {
        version: APP_STATE_VERSION + 1,
        ..AppConfig::default()
    };
    assert_eq!(migrate(newer).version, APP_STATE_VERSION + 1);
}

#[test]
fn migrated_legacy_config_leaves_the_bluetooth_optin_off() {
    let legacy = AppConfig {
        version: 0,
        ..AppConfig::default()
    };
    let migrated = migrate(legacy);
    assert_eq!(migrated.version, APP_STATE_VERSION);
    assert!(!migrated.bluetooth_monitoring_enabled);
}

#[test]
fn migrated_legacy_config_leaves_update_checks_off() {
    let legacy = AppConfig {
        version: 0,
        ..AppConfig::default()
    };
    let migrated = migrate(legacy);
    assert_eq!(migrated.version, APP_STATE_VERSION);
    assert!(!migrated.update_check_on_startup);
}
