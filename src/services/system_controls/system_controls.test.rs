use super::*;

#[test]
fn clamp01_clamps_out_of_range() {
    assert_eq!(clamp01(-0.5), 0.0);
    assert_eq!(clamp01(1.7), 1.0);
    assert_eq!(clamp01(0.42), 0.42);
}

#[test]
fn pending_snapshot_marks_every_control_unavailable() {
    let snap = pending_snapshot();
    assert!(matches!(
        snap.battery,
        ControlState::<BatteryStatus>::Unavailable { .. }
    ));
    assert!(matches!(
        snap.bluetooth,
        ControlState::<BluetoothStatus>::Unavailable { .. }
    ));
    assert!(matches!(
        snap.volume,
        ControlState::<VolumeStatus>::Unavailable { .. }
    ));
    assert!(matches!(
        snap.brightness,
        ControlState::<BrightnessStatus>::Unavailable { .. }
    ));
    assert_eq!(SystemControlsSnapshot::default(), snap);
}

#[test]
fn web_unsupported_snapshot_uses_the_web_reason_key() {
    let snap = web_unsupported_snapshot();
    assert_eq!(
        snap.battery,
        ControlState::Unsupported {
            reason_key: REASON_WEB
        }
    );
    assert_eq!(
        snap.bluetooth,
        ControlState::Unsupported {
            reason_key: REASON_WEB
        }
    );
    assert_eq!(
        snap.volume,
        ControlState::Unsupported {
            reason_key: REASON_WEB
        }
    );
    assert_eq!(
        snap.brightness,
        ControlState::Unsupported {
            reason_key: REASON_WEB
        }
    );
}

#[test]
fn control_state_reason_and_readiness_follow_the_variant() {
    let unsupported: ControlState<()> = ControlState::Unsupported {
        reason_key: REASON_WEB,
    };
    assert_eq!(unsupported.reason().as_deref(), Some(REASON_WEB));
    assert!(!unsupported.is_ready());

    let unavailable: ControlState<()> = ControlState::Unavailable {
        error: "no battery".into(),
    };
    assert_eq!(unavailable.reason().as_deref(), Some("no battery"));
    assert!(!unavailable.is_ready());

    let ready: ControlState<()> = ControlState::Ready(());
    assert_eq!(ready.reason(), None);
    assert!(ready.is_ready());
}

#[test]
fn localization_keys_resolve_in_the_catalogs() {
    let keys = [
        REASON_WEB,
        REASON_MACOS_BRIGHTNESS,
        REASON_WINDOWS_BLUETOOTH,
        REASON_MACOS_BLUETOOTH_POWER,
        "system_controls_title",
        "system_controls_battery",
        "system_controls_charge",
        "system_controls_state_charging",
        "system_controls_state_discharging",
        "system_controls_state_full",
        "system_controls_state_empty",
        "system_controls_state_paused",
        "system_controls_state_unknown",
        "system_controls_low_power_mode",
        "system_controls_on",
        "system_controls_off",
        "system_controls_not_exposed",
        "system_controls_bluetooth",
        "system_controls_powered",
        "system_controls_devices",
        "system_controls_connected",
        "system_controls_not_connected",
        "system_controls_no_devices",
        "system_controls_volume",
        "system_controls_mute",
        "system_controls_brightness",
        "system_controls_unsupported_detail",
        "system_controls_unavailable_detail",
        "system_controls_set_failed",
    ];
    for key in keys {
        let text = crate::i18n::localize(key);
        assert_ne!(text, key, "missing catalog entry for {key}");
    }
}
