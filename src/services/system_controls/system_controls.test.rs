use super::*;
#[cfg(not(target_family = "wasm"))]
use std::sync::{Arc, Mutex};

#[cfg(not(target_family = "wasm"))]
use crate::app_state::{AppConfig, debounce_test_lock, force_save, install_for_tests};
#[cfg(not(target_family = "wasm"))]
use gpui_kit::TestAppContext;

fn ready_core() -> CoreControlsSnapshot {
    CoreControlsSnapshot {
        battery: ControlState::Ready(BatteryStatus {
            charge: 0.5,
            state: BatteryChargeState::Unknown,
            low_power_mode: None,
        }),
        volume: ControlState::Ready(VolumeStatus {
            level: 0.25,
            muted: false,
        }),
        brightness: ControlState::Ready(BrightnessStatus { level: 0.75 }),
    }
}

fn ready_bluetooth() -> ControlState<BluetoothStatus> {
    ControlState::Ready(BluetoothStatus {
        powered: Some(true),
        devices: Vec::new(),
    })
}

#[test]
fn bluetooth_access_from_read_maps_each_track_result() {
    let ready = ready_bluetooth();
    assert_eq!(BluetoothAccess::from_read(&ready), BluetoothAccess::Active);

    let unsupported = ControlState::<BluetoothStatus>::Unsupported {
        reason_key: REASON_WINDOWS_BLUETOOTH,
    };
    assert_eq!(
        BluetoothAccess::from_read(&unsupported),
        BluetoothAccess::Denied {
            reason: "no supported bluetooth backend for Windows".into(),
        }
    );

    let unavailable = ControlState::<BluetoothStatus>::Unavailable {
        error: "no bluetooth controller".into(),
    };
    assert_eq!(
        BluetoothAccess::from_read(&unavailable),
        BluetoothAccess::Denied {
            reason: "no bluetooth controller".into(),
        }
    );
}

#[test]
fn only_active_polls_the_bluetooth_track() {
    assert!(!BluetoothAccess::NotRequested.polls_bluetooth());
    assert!(!BluetoothAccess::Requesting.polls_bluetooth());
    assert!(BluetoothAccess::Active.polls_bluetooth());
    assert!(
        !BluetoothAccess::Denied {
            reason: "denied".into()
        }
        .polls_bluetooth()
    );
}

#[test]
fn snapshots_start_not_requested_and_web_denied() {
    assert_eq!(
        pending_snapshot().bluetooth_access,
        BluetoothAccess::NotRequested
    );
    assert_eq!(
        SystemControlsSnapshot::default().bluetooth_access,
        BluetoothAccess::NotRequested
    );
    assert_eq!(
        web_unsupported_snapshot().bluetooth_access,
        BluetoothAccess::Denied {
            reason: "not available on the web build".into()
        }
    );
}

#[test]
fn merge_refresh_replaces_core_and_preserves_the_gated_track() {
    let mut current = pending_snapshot();
    current.bluetooth_access = BluetoothAccess::Active;
    current.bluetooth = ready_bluetooth();

    let core_only = merge_refresh(&current, ready_core(), None);
    assert!(core_only.battery.is_ready());
    assert!(
        core_only.bluetooth.is_ready(),
        "an always-on refresh must not clobber the gated track"
    );
    assert_eq!(core_only.bluetooth_access, BluetoothAccess::Active);

    let failed = merge_refresh(
        &current,
        ready_core(),
        Some(ControlState::Unavailable {
            error: "dbus hiccup".into(),
        }),
    );
    assert!(!failed.bluetooth.is_ready());
    assert_eq!(
        failed.bluetooth_access,
        BluetoothAccess::Active,
        "a runtime poll failure degrades the row without stopping the poller"
    );
}

#[test]
fn bluetooth_capability_reflects_the_gate_not_just_readiness() {
    let supported = bluetooth_supported();

    let off = bluetooth_capability(&BluetoothAccess::NotRequested, &pending());
    assert_eq!(off.supported, supported);
    assert!(!off.enabled);
    assert!(!off.degraded);

    let requesting = bluetooth_capability(&BluetoothAccess::Requesting, &requesting_bluetooth());
    assert_eq!(requesting.supported, supported);
    assert!(requesting.enabled);

    let active = bluetooth_capability(&BluetoothAccess::Active, &ready_bluetooth());
    assert_eq!(
        active,
        crate::capabilities::CapabilityStatus::supported_enabled()
    );

    let degraded = bluetooth_capability(
        &BluetoothAccess::Active,
        &ControlState::Unavailable {
            error: "dbus hiccup".into(),
        },
    );
    assert!(degraded.degraded);

    let denied = bluetooth_capability(
        &BluetoothAccess::Denied {
            reason: "tcc denied".into(),
        },
        &pending(),
    );
    assert_eq!(denied.supported, supported);
    assert!(!denied.enabled);
    assert_eq!(
        denied.degraded, supported,
        "denied is terminal only where a reader exists"
    );
    assert_eq!(denied.reason.as_deref(), Some("tcc denied"));
}

#[cfg(not(target_family = "wasm"))]
fn mock_readers(bluetooth: ControlState<BluetoothStatus>) -> (Readers, Arc<Mutex<usize>>) {
    let calls = Arc::new(Mutex::new(0usize));
    let counted = calls.clone();
    let readers = Readers {
        read_core: Arc::new(ready_core),
        read_bluetooth: Arc::new(move || {
            *counted.lock().unwrap() += 1;
            bluetooth.clone()
        }),
    };
    (readers, calls)
}

#[cfg(not(target_family = "wasm"))]
fn current_snapshot(cx: &TestAppContext) -> SystemControlsSnapshot {
    cx.read(|cx| cx.global::<SystemControlsSnapshot>().clone())
}

#[cfg(not(target_family = "wasm"))]
#[gpui_kit::test]
async fn refresh_never_reads_bluetooth_before_the_gate_opens(cx: &TestAppContext) {
    cx.update(|cx| install_for_tests(AppConfig::default(), cx));
    let (readers, bluetooth_reads) = mock_readers(ready_bluetooth());
    cx.update(|cx| cx.set_global(pending_snapshot()));

    cx.update(|cx| refresh_with(readers, cx));
    cx.executor().run_until_parked();

    assert_eq!(
        *bluetooth_reads.lock().unwrap(),
        0,
        "an always-on refresh must not touch the bluetooth reader"
    );
    let snapshot = current_snapshot(cx);
    assert!(snapshot.battery.is_ready());
    assert!(!snapshot.bluetooth.is_ready());
    assert_eq!(snapshot.bluetooth_access, BluetoothAccess::NotRequested);
}

#[cfg(not(target_family = "wasm"))]
#[gpui_kit::test]
async fn enable_handshake_activates_polling_and_persists_the_optin(cx: &TestAppContext) {
    let _guard = debounce_test_lock();
    cx.update(|cx| install_for_tests(AppConfig::default(), cx));
    let (readers, bluetooth_reads) = mock_readers(ready_bluetooth());
    cx.update(|cx| cx.set_global(pending_snapshot()));

    cx.update(|cx| enable_with(readers.clone(), cx));
    cx.executor().run_until_parked();

    assert_eq!(
        *bluetooth_reads.lock().unwrap(),
        1,
        "exactly the handshake read"
    );
    let snapshot = current_snapshot(cx);
    assert_eq!(snapshot.bluetooth_access, BluetoothAccess::Active);
    assert!(snapshot.bluetooth.is_ready());
    assert!(
        cx.read(|cx| crate::app_state::config(cx).bluetooth_monitoring_enabled),
        "the opt-in must persist for the next launch"
    );

    cx.update(|cx| refresh_with(readers, cx));
    cx.executor().run_until_parked();
    assert_eq!(
        *bluetooth_reads.lock().unwrap(),
        2,
        "once Active the poller joins the bluetooth track"
    );
    cx.update(force_save);
}

#[cfg(not(target_family = "wasm"))]
#[gpui_kit::test]
async fn denied_handshake_never_repolls_automatically(cx: &TestAppContext) {
    let _guard = debounce_test_lock();
    cx.update(|cx| install_for_tests(AppConfig::default(), cx));
    let (readers, bluetooth_reads) = mock_readers(ControlState::Unavailable {
        error: "tcc denied".into(),
    });
    cx.update(|cx| cx.set_global(pending_snapshot()));

    cx.update(|cx| enable_with(readers.clone(), cx));
    cx.executor().run_until_parked();
    assert_eq!(
        current_snapshot(cx).bluetooth_access,
        BluetoothAccess::Denied {
            reason: "tcc denied".into()
        }
    );
    assert_eq!(*bluetooth_reads.lock().unwrap(), 1);

    cx.update(|cx| refresh_with(readers.clone(), cx));
    cx.update(|cx| refresh_with(readers.clone(), cx));
    cx.executor().run_until_parked();
    assert_eq!(
        *bluetooth_reads.lock().unwrap(),
        1,
        "a denied gate must not re-poll"
    );

    cx.update(|cx| enable_with(readers, cx));
    cx.executor().run_until_parked();
    assert_eq!(
        *bluetooth_reads.lock().unwrap(),
        2,
        "an explicit user retry is allowed"
    );
    cx.update(force_save);
}

#[cfg(not(target_family = "wasm"))]
#[gpui_kit::test]
async fn disable_returns_the_gate_to_not_requested(cx: &TestAppContext) {
    let _guard = debounce_test_lock();
    cx.update(|cx| install_for_tests(AppConfig::default(), cx));
    let (readers, _) = mock_readers(ready_bluetooth());
    cx.update(|cx| cx.set_global(pending_snapshot()));
    cx.update(|cx| enable_with(readers, cx));
    cx.executor().run_until_parked();

    cx.update(disable_bluetooth);

    let snapshot = current_snapshot(cx);
    assert_eq!(snapshot.bluetooth_access, BluetoothAccess::NotRequested);
    assert!(!snapshot.bluetooth.is_ready());
    assert!(!cx.read(|cx| crate::app_state::config(cx).bluetooth_monitoring_enabled));
    cx.update(force_save);
}

#[cfg(not(target_family = "wasm"))]
#[gpui_kit::test]
async fn launch_handshake_reruns_only_with_a_persisted_optin(cx: &TestAppContext) {
    let _guard = debounce_test_lock();
    cx.update(|cx| install_for_tests(AppConfig::default(), cx));
    let (readers, bluetooth_reads) = mock_readers(ready_bluetooth());

    cx.update(|cx| initialize_with(readers.clone(), cx));
    cx.executor().run_until_parked();
    assert_eq!(*bluetooth_reads.lock().unwrap(), 0);
    assert_eq!(
        current_snapshot(cx).bluetooth_access,
        BluetoothAccess::NotRequested
    );

    // Fresh launch with the persisted opt-in: the runtime gate resets to
    // NotRequested and the handshake redoes the single read.
    cx.update(|cx| {
        install_for_tests(
            AppConfig {
                bluetooth_monitoring_enabled: true,
                ..AppConfig::default()
            },
            cx,
        )
    });
    cx.update(|cx| initialize_with(readers, cx));
    cx.executor().run_until_parked();

    assert_eq!(
        *bluetooth_reads.lock().unwrap(),
        1,
        "exactly the handshake read"
    );
    assert_eq!(
        current_snapshot(cx).bluetooth_access,
        BluetoothAccess::Active
    );
    cx.update(force_save);
}

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
