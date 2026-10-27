use super::*;

#[test]
fn migrates_legacy_config_to_current_version() {
    let legacy = AppConfig {
        version: 0,
        global_shortcut_enabled: false,
        ..AppConfig::default()
    };
    let migrated = migrate(legacy);
    assert_eq!(migrated.version, APP_STATE_VERSION);
    assert!(migrated.global_shortcut_enabled);
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
