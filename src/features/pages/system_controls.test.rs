use gpui_kit::prelude::*;
use gpui_kit::{SharedString, TestAppContext};

use super::{BluetoothPanel, SystemControlsPage, bluetooth_panel};
use crate::system_controls::{
    BluetoothAccess, BluetoothStatus, ControlState, REASON_WINDOWS_BLUETOOTH,
    SystemControlsSnapshot,
};

fn pending<T>() -> ControlState<T> {
    ControlState::Unavailable {
        error: SharedString::from("pending first read"),
    }
}

fn ready_bluetooth() -> ControlState<BluetoothStatus> {
    ControlState::Ready(BluetoothStatus {
        powered: Some(true),
        devices: Vec::new(),
    })
}

fn snapshot_for(
    access: BluetoothAccess,
    bluetooth: ControlState<BluetoothStatus>,
) -> SystemControlsSnapshot {
    SystemControlsSnapshot {
        battery: pending(),
        bluetooth,
        volume: pending(),
        brightness: pending(),
        bluetooth_access: access,
    }
}

#[test]
fn not_requested_and_requesting_map_to_the_enable_affordance() {
    let not_requested = snapshot_for(BluetoothAccess::NotRequested, pending());
    assert_eq!(
        bluetooth_panel(&not_requested),
        BluetoothPanel::Enable { busy: false }
    );

    let requesting = snapshot_for(BluetoothAccess::Requesting, pending());
    assert_eq!(
        bluetooth_panel(&requesting),
        BluetoothPanel::Enable { busy: true }
    );
}

#[test]
fn active_maps_to_live_values_even_when_a_poll_degrades() {
    let live = snapshot_for(BluetoothAccess::Active, ready_bluetooth());
    assert_eq!(bluetooth_panel(&live), BluetoothPanel::Live);

    let degraded = snapshot_for(
        BluetoothAccess::Active,
        ControlState::Unavailable {
            error: SharedString::from("dbus hiccup"),
        },
    );
    assert_eq!(
        bluetooth_panel(&degraded),
        BluetoothPanel::Live,
        "the stop affordance must stay reachable while a poll is degraded"
    );
}

#[test]
fn denied_maps_to_guidance_with_the_reason() {
    let denied = snapshot_for(
        BluetoothAccess::Denied {
            reason: SharedString::from("tcc denied"),
        },
        ControlState::Unavailable {
            error: SharedString::from("tcc denied"),
        },
    );
    assert_eq!(
        bluetooth_panel(&denied),
        BluetoothPanel::Denied {
            reason: SharedString::from("tcc denied"),
        }
    );
}

#[test]
fn denied_on_an_unsupported_platform_maps_to_the_platform_row() {
    let windows = snapshot_for(
        BluetoothAccess::Denied {
            reason: SharedString::from("no supported bluetooth backend for Windows"),
        },
        ControlState::Unsupported {
            reason_key: REASON_WINDOWS_BLUETOOTH,
        },
    );
    assert_eq!(
        bluetooth_panel(&windows),
        BluetoothPanel::UnsupportedPlatform
    );
}

#[gpui_kit::test]
fn page_shows_the_enable_affordance_while_not_requested(cx: &mut TestAppContext) {
    cx.update(|cx| {
        cx.set_global(snapshot_for(BluetoothAccess::NotRequested, pending()));
    });
    cx.update(gpui_kit::component::theme::init);
    cx.add_window(|window, cx| SystemControlsPage::new(window, cx));
    cx.read(|cx| {
        let snapshot = crate::system_controls::snapshot(cx);
        assert_eq!(
            bluetooth_panel(&snapshot),
            BluetoothPanel::Enable { busy: false }
        );
        assert!(
            !snapshot.bluetooth.is_ready(),
            "values must stay hidden before the opt-in"
        );
    });
}

#[gpui_kit::test]
fn page_shows_live_values_while_active(cx: &mut TestAppContext) {
    cx.update(|cx| {
        cx.set_global(snapshot_for(BluetoothAccess::Active, ready_bluetooth()));
    });
    cx.update(gpui_kit::component::theme::init);
    cx.add_window(|window, cx| SystemControlsPage::new(window, cx));
    cx.read(|cx| {
        let snapshot = crate::system_controls::snapshot(cx);
        assert_eq!(bluetooth_panel(&snapshot), BluetoothPanel::Live);
        assert!(snapshot.bluetooth.is_ready());
    });
}

#[gpui_kit::test]
fn page_shows_guidance_and_retry_while_denied(cx: &mut TestAppContext) {
    cx.update(|cx| {
        cx.set_global(snapshot_for(
            BluetoothAccess::Denied {
                reason: SharedString::from("tcc denied"),
            },
            ControlState::Unavailable {
                error: SharedString::from("tcc denied"),
            },
        ));
    });
    cx.update(gpui_kit::component::theme::init);
    cx.add_window(|window, cx| SystemControlsPage::new(window, cx));
    cx.read(|cx| {
        let snapshot = crate::system_controls::snapshot(cx);
        assert_eq!(
            bluetooth_panel(&snapshot),
            BluetoothPanel::Denied {
                reason: SharedString::from("tcc denied"),
            },
            "a denied gate must show guidance plus a manual retry, never auto-enable"
        );
        assert!(!snapshot.bluetooth.is_ready());
    });
}

#[test]
fn bluetooth_gate_keys_resolve_in_the_catalogs() {
    let keys = [
        "system_controls_bluetooth_enable",
        "system_controls_bluetooth_enable_hint",
        "system_controls_bluetooth_stop",
        "system_controls_bluetooth_retry",
        "system_controls_bluetooth_denied_hint",
    ];
    for key in keys {
        let text = crate::i18n::localize(key);
        assert_ne!(text, key, "missing catalog entry for {key}");
    }
}
