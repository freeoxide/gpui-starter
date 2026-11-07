use super::*;
#[cfg(not(target_family = "wasm"))]
use crate::platform::system_controls::SystemOp;
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
            reason: crate::i18n::localize(REASON_WINDOWS_BLUETOOTH).into(),
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
            reason: crate::i18n::localize(REASON_WEB).into()
        }
    );
}

#[test]
fn merge_refresh_replaces_core_and_preserves_the_gated_track() {
    let mut current = pending_snapshot();
    current.bluetooth_access = BluetoothAccess::Active;
    current.bluetooth = ready_bluetooth();

    let core_only = merge_refresh(&current, 0, 0, ready_core(), None);
    assert!(core_only.battery.is_ready());
    assert!(
        core_only.bluetooth.is_ready(),
        "an always-on refresh must not clobber the gated track"
    );
    assert_eq!(core_only.bluetooth_access, BluetoothAccess::Active);

    let failed = merge_refresh(
        &current,
        0,
        0,
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
fn a_superseded_read_cannot_revert_a_fresh_write() {
    let mut current = pending_snapshot();
    current.volume = ControlState::Ready(VolumeStatus {
        level: 0.25,
        muted: false,
    });
    // The write landed after the read was spawned: mirror plus bump.
    let mut written = current.clone();
    written.volume = ControlState::Ready(VolumeStatus {
        level: 0.8,
        muted: false,
    });
    let stale = ready_core();

    let dropped = merge_refresh(&written, 0, 1, stale, None);
    assert_eq!(
        dropped.volume, written.volume,
        "a read superseded by a write must be dropped, not merged"
    );

    let reconciled = ControlState::Ready(VolumeStatus {
        level: 0.9,
        muted: false,
    });
    let mut fresh = ready_core();
    fresh.volume = reconciled.clone();
    let merged = merge_refresh(&written, 1, 1, fresh, None);
    assert_eq!(merged.volume, reconciled);
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
        apply_op: Arc::new(|_| Ok(())),
    };
    (readers, calls)
}

#[cfg(not(target_family = "wasm"))]
fn current_snapshot(cx: &TestAppContext) -> SystemControlsSnapshot {
    cx.read(|cx| cx.global::<SystemControlsSnapshot>().clone())
}

#[cfg(not(target_family = "wasm"))]
fn ready_snapshot() -> SystemControlsSnapshot {
    let core = ready_core();
    SystemControlsSnapshot {
        battery: core.battery,
        bluetooth: ControlState::Ready(BluetoothStatus {
            powered: Some(false),
            devices: Vec::new(),
        }),
        volume: core.volume,
        brightness: core.brightness,
        bluetooth_access: BluetoothAccess::Active,
    }
}

#[cfg(not(target_family = "wasm"))]
fn write_seam(
    core: CoreControlsSnapshot,
    result: Result<(), String>,
) -> (Readers, Arc<Mutex<Vec<SystemOp>>>) {
    let ops = Arc::new(Mutex::new(Vec::new()));
    let recorded = ops.clone();
    let readers = Readers {
        read_core: Arc::new(move || core.clone()),
        read_bluetooth: Arc::new(ready_bluetooth),
        apply_op: Arc::new(move |op| {
            recorded.lock().unwrap().push(op);
            result.clone()
        }),
    };
    (readers, ops)
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
async fn inflight_handshake_read_cannot_reactivate_a_disabled_gate(cx: &TestAppContext) {
    let _guard = debounce_test_lock();
    cx.update(|cx| install_for_tests(AppConfig::default(), cx));
    let (readers, bluetooth_reads) = mock_readers(ready_bluetooth());
    cx.update(|cx| cx.set_global(pending_snapshot()));

    cx.update(|cx| enable_with(readers, cx));
    cx.update(disable_bluetooth);
    cx.executor().run_until_parked();

    assert_eq!(
        *bluetooth_reads.lock().unwrap(),
        1,
        "the read spawned by the handshake completed after the disable"
    );
    let gated = current_snapshot(cx);
    assert_eq!(gated.bluetooth_access, BluetoothAccess::NotRequested);
    assert!(
        !gated.bluetooth.is_ready(),
        "a disabled gate must not end up Active with a Ready row"
    );

    cx.update(|cx| set_bluetooth_powered(true, cx));
    assert_eq!(
        current_snapshot(cx),
        gated,
        "the powered write must stay unreachable while the gate is off"
    );
    cx.update(force_save);
}

#[cfg(not(target_family = "wasm"))]
#[gpui_kit::test]
async fn the_newest_handshake_owns_the_gate_after_a_reenable(cx: &TestAppContext) {
    let _guard = debounce_test_lock();
    cx.update(|cx| install_for_tests(AppConfig::default(), cx));
    let reads = Arc::new(Mutex::new(0usize));
    let counted = reads.clone();
    let readers = Readers {
        read_core: Arc::new(ready_core),
        read_bluetooth: Arc::new(move || {
            let call = {
                let mut count = counted.lock().unwrap();
                *count += 1;
                *count
            };
            // The first prompt fails; the re-enable's prompt passes.
            if call == 1 {
                ControlState::Unavailable {
                    error: "first prompt denied".into(),
                }
            } else {
                ready_bluetooth()
            }
        }),
        apply_op: Arc::new(|_| Ok(())),
    };
    cx.update(|cx| cx.set_global(pending_snapshot()));

    cx.update(|cx| enable_with(readers.clone(), cx));
    cx.update(disable_bluetooth);
    cx.update(|cx| enable_with(readers, cx));
    cx.executor().run_until_parked();

    assert_eq!(*reads.lock().unwrap(), 2, "both handshakes ran their read");
    let snapshot = current_snapshot(cx);
    assert_eq!(
        snapshot.bluetooth_access,
        BluetoothAccess::Active,
        "the newer handshake's result owns the gate, not whichever read lands first"
    );
    assert!(snapshot.bluetooth.is_ready());
    cx.update(force_save);
}

#[cfg(not(target_family = "wasm"))]
#[gpui_kit::test]
async fn inflight_poll_read_cannot_restore_the_gated_bluetooth_row(cx: &TestAppContext) {
    let _guard = debounce_test_lock();
    cx.update(|cx| install_for_tests(AppConfig::default(), cx));
    let (readers, bluetooth_reads) = mock_readers(ready_bluetooth());
    cx.update(|cx| cx.set_global(pending_snapshot()));

    cx.update(|cx| enable_with(readers.clone(), cx));
    cx.executor().run_until_parked();
    assert_eq!(*bluetooth_reads.lock().unwrap(), 1);

    cx.update(|cx| refresh_with(readers, cx));
    cx.update(disable_bluetooth);
    cx.executor().run_until_parked();

    let snapshot = current_snapshot(cx);
    assert_eq!(snapshot.bluetooth_access, BluetoothAccess::NotRequested);
    assert!(
        snapshot.battery.is_ready(),
        "the always-on tracks still merge behind the gate"
    );
    assert!(
        !snapshot.bluetooth.is_ready(),
        "a poll that was in flight during the disable must not commit its track"
    );
    assert_eq!(
        *bluetooth_reads.lock().unwrap(),
        2,
        "handshake read plus the one in-flight poll"
    );
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

#[cfg(not(target_family = "wasm"))]
#[gpui_kit::test]
async fn set_volume_clamps_mirrors_then_reconciles(cx: &TestAppContext) {
    cx.update(|cx| install_for_tests(AppConfig::default(), cx));
    let reconciled = CoreControlsSnapshot {
        volume: ControlState::Ready(VolumeStatus {
            level: 0.6,
            muted: true,
        }),
        ..ready_core()
    };
    let (readers, ops) = write_seam(reconciled, Ok(()));
    cx.update(|cx| {
        cx.set_global(readers);
        cx.set_global(ready_snapshot());
    });

    cx.update(|cx| set_volume(1.7, cx));
    assert_eq!(
        current_snapshot(cx).volume,
        ControlState::Ready(VolumeStatus {
            level: 1.0,
            muted: false,
        }),
        "the optimistic mirror writes the clamped level before the apply lands"
    );
    assert_eq!(
        cx.read(|cx| cx.global::<ControlGate>().refresh_generation),
        1,
        "every mirror write bumps the refresh generation"
    );
    cx.executor().run_until_parked();

    let recorded = ops.lock().unwrap().clone();
    assert_eq!(recorded.len(), 1);
    assert!(matches!(recorded[0], SystemOp::SetVolume { level } if level == 1.0));
    assert_eq!(
        current_snapshot(cx).volume,
        ControlState::Ready(VolumeStatus {
            level: 0.6,
            muted: true,
        }),
        "the post-apply refresh reconciles from the re-read"
    );
}

#[cfg(not(target_family = "wasm"))]
#[gpui_kit::test]
async fn set_volume_applies_even_without_a_ready_mirror(cx: &TestAppContext) {
    cx.update(|cx| install_for_tests(AppConfig::default(), cx));
    let (readers, ops) = write_seam(ready_core(), Ok(()));
    cx.update(|cx| {
        cx.set_global(readers);
        let mut snapshot = ready_snapshot();
        snapshot.volume = ControlState::Unavailable {
            error: "pending first read".into(),
        };
        cx.set_global(snapshot);
    });

    cx.update(|cx| set_volume(0.5, cx));
    cx.executor().run_until_parked();

    let recorded = ops.lock().unwrap().clone();
    assert_eq!(recorded.len(), 1, "volume applies unconditionally");
    assert!(matches!(recorded[0], SystemOp::SetVolume { level } if level == 0.5));
    assert!(
        current_snapshot(cx).volume.is_ready(),
        "the reconciling read brings the track back Ready"
    );
}

#[cfg(not(target_family = "wasm"))]
#[gpui_kit::test]
async fn set_volume_muted_mirrors_the_switch_and_applies(cx: &TestAppContext) {
    cx.update(|cx| install_for_tests(AppConfig::default(), cx));
    let (readers, ops) = write_seam(ready_core(), Ok(()));
    cx.update(|cx| {
        cx.set_global(readers);
        cx.set_global(ready_snapshot());
    });

    cx.update(|cx| set_volume_muted(true, cx));
    assert_eq!(
        current_snapshot(cx).volume,
        ControlState::Ready(VolumeStatus {
            level: 0.25,
            muted: true,
        })
    );
    cx.executor().run_until_parked();

    let recorded = ops.lock().unwrap().clone();
    assert_eq!(recorded.len(), 1);
    assert!(matches!(
        recorded[0],
        SystemOp::SetVolumeMuted { muted: true }
    ));
}

#[cfg(not(target_family = "wasm"))]
#[gpui_kit::test]
async fn set_brightness_waits_for_a_ready_control(cx: &TestAppContext) {
    cx.update(|cx| install_for_tests(AppConfig::default(), cx));
    let (readers, ops) = write_seam(ready_core(), Ok(()));
    cx.update(|cx| {
        cx.set_global(readers);
        let mut snapshot = ready_snapshot();
        snapshot.brightness = ControlState::Unavailable {
            error: "pending first read".into(),
        };
        cx.set_global(snapshot);
    });

    cx.update(|cx| set_brightness(0.9, cx));
    cx.executor().run_until_parked();
    assert!(
        ops.lock().unwrap().is_empty(),
        "no apply while the control is not ready"
    );
    assert!(!current_snapshot(cx).brightness.is_ready());

    cx.update(|cx| cx.set_global(ready_snapshot()));
    cx.update(|cx| set_brightness(1.2, cx));
    assert_eq!(
        current_snapshot(cx).brightness,
        ControlState::Ready(BrightnessStatus { level: 1.0 }),
        "the mirror clamps before the apply lands"
    );
    cx.executor().run_until_parked();

    let recorded = ops.lock().unwrap().clone();
    assert_eq!(recorded.len(), 1);
    assert!(matches!(recorded[0], SystemOp::SetBrightness { level } if level == 1.0));
    assert_eq!(
        current_snapshot(cx).brightness,
        ControlState::Ready(BrightnessStatus { level: 0.75 }),
        "the post-apply refresh reconciles from the re-read"
    );
}

#[cfg(not(target_family = "wasm"))]
#[gpui_kit::test]
async fn set_bluetooth_powered_applies_through_an_open_gate(cx: &TestAppContext) {
    cx.update(|cx| install_for_tests(AppConfig::default(), cx));
    let (readers, ops) = write_seam(ready_core(), Ok(()));
    cx.update(|cx| {
        cx.set_global(readers);
        cx.set_global(ready_snapshot());
    });

    cx.update(|cx| set_bluetooth_powered(true, cx));
    assert_eq!(
        current_snapshot(cx).bluetooth,
        ControlState::Ready(BluetoothStatus {
            powered: Some(true),
            devices: Vec::new(),
        })
    );
    cx.executor().run_until_parked();

    let recorded = ops.lock().unwrap().clone();
    assert_eq!(recorded.len(), 1);
    assert!(matches!(
        recorded[0],
        SystemOp::SetBluetoothPowered { powered: true }
    ));
}

#[cfg(not(target_family = "wasm"))]
#[gpui_kit::test]
async fn a_failed_apply_reports_to_the_error_surface(cx: &TestAppContext) {
    cx.update(|cx| install_for_tests(AppConfig::default(), cx));
    cx.update(crate::error_surface::initialize);
    let (readers, ops) = write_seam(ready_core(), Err("no mixer".to_string()));
    cx.update(|cx| {
        cx.set_global(readers);
        cx.set_global(ready_snapshot());
    });

    cx.update(|cx| set_volume(0.5, cx));
    cx.executor().run_until_parked();

    assert_eq!(ops.lock().unwrap().len(), 1);
    let message = cx
        .read(crate::error_surface::latest_message)
        .expect("the failed apply reaches the error surface");
    assert!(
        message.contains("no mixer"),
        "the OS error text survives into the message: {message}"
    );
    assert_eq!(
        current_snapshot(cx).volume,
        ControlState::Ready(VolumeStatus {
            level: 0.25,
            muted: false,
        }),
        "the trailing refresh still reconciles after a failure"
    );
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
