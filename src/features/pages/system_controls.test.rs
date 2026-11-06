use gpui_kit::component::slider::{SliderEvent, SliderValue};
use gpui_kit::{SharedString, TestAppContext};

use super::{BluetoothPanel, SystemControlsPage, bluetooth_panel};
use crate::system_controls::{
    BluetoothAccess, BluetoothStatus, BrightnessStatus, ControlState, REASON_WINDOWS_BLUETOOTH,
    SystemControlsSnapshot, VolumeStatus,
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
async fn sync_sliders_moves_each_entity_only_while_its_track_is_ready(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let mut entity = None;
    cx.add_window(|window, cx| {
        let page = SystemControlsPage::new(window, cx);
        entity = Some(cx.entity());
        page
    });
    let page = entity.expect("page entity");
    cx.read(|cx| {
        let page = page.read(cx);
        assert_eq!(page.volume.read(cx).value().start(), 0.);
        assert_eq!(page.brightness.read(cx).value().start(), 0.);
    });

    let mut ready = snapshot_for(BluetoothAccess::NotRequested, pending());
    ready.volume = ControlState::Ready(VolumeStatus {
        level: 0.3,
        muted: false,
    });
    ready.brightness = ControlState::Ready(BrightnessStatus { level: 0.8 });
    cx.update(|cx| cx.set_global(ready));
    cx.read(|cx| {
        let page = page.read(cx);
        assert_eq!(page.volume.read(cx).value().start(), 0.3);
        assert_eq!(page.brightness.read(cx).value().start(), 0.8);
    });

    let mut degraded = snapshot_for(BluetoothAccess::NotRequested, pending());
    degraded.volume = pending();
    degraded.brightness = ControlState::Ready(BrightnessStatus { level: 0.9 });
    cx.update(|cx| cx.set_global(degraded));
    cx.read(|cx| {
        let page = page.read(cx);
        assert_eq!(
            page.volume.read(cx).value().start(),
            0.3,
            "a degraded track must not stomp its slider"
        );
        assert_eq!(page.brightness.read(cx).value().start(), 0.9);
    });
}

#[gpui_kit::test]
async fn a_dragging_slider_is_not_yanked_by_a_stale_readback(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let mut entity = None;
    let handle = cx.add_window(|window, cx| {
        let page = SystemControlsPage::new(window, cx);
        entity = Some(cx.entity());
        page
    });
    let page = entity.expect("page entity");
    let volume = cx.read(|cx| page.read(cx).volume.clone());
    let seeded = || {
        let mut snapshot = snapshot_for(BluetoothAccess::NotRequested, pending());
        snapshot.volume = ControlState::Ready(VolumeStatus {
            level: 0.3,
            muted: false,
        });
        snapshot
    };
    cx.update(|cx| cx.set_global(seeded()));
    cx.read(|cx| assert_eq!(volume.read(cx).value().start(), 0.3));

    // Drag: the kit emits Change before notifying, which arms the window.
    cx.update(|cx| {
        handle
            .update(cx, |_, window, cx| {
                volume.update(cx, |state, cx| {
                    cx.emit(SliderEvent::Change(SliderValue::Single(0.5)));
                    state.set_value(0.5, window, cx);
                });
            })
            .unwrap();
    });
    cx.update(|cx| cx.set_global(seeded()));
    cx.read(|cx| {
        assert_eq!(
            volume.read(cx).value().start(),
            0.5,
            "a stale readback must not yank a dragged slider"
        );
    });

    // Once the window passes, the poll owns the thumb again.
    cx.update(|cx| {
        page.update(cx, |page, _| {
            assert!(
                page.suppress_until.is_some(),
                "the drag must arm the window"
            );
            page.suppress_until = None;
        })
    });
    let mut fresh = snapshot_for(BluetoothAccess::NotRequested, pending());
    fresh.volume = ControlState::Ready(VolumeStatus {
        level: 0.6,
        muted: false,
    });
    cx.update(|cx| cx.set_global(fresh));
    cx.read(|cx| assert_eq!(volume.read(cx).value().start(), 0.6));
}

#[gpui_kit::test]
async fn an_a11y_value_change_commits_and_survives_the_next_sync(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    cx.update(crate::error_surface::initialize);
    let mut entity = None;
    let handle = cx.add_window(|window, cx| {
        let page = SystemControlsPage::new(window, cx);
        entity = Some(cx.entity());
        page
    });
    let page = entity.expect("page entity");
    let volume = cx.read(|cx| page.read(cx).volume.clone());
    let mut ready = snapshot_for(BluetoothAccess::NotRequested, pending());
    ready.volume = ControlState::Ready(VolumeStatus {
        level: 0.3,
        muted: false,
    });
    cx.update(|cx| cx.set_global(ready));
    cx.read(|cx| assert_eq!(volume.read(cx).value().start(), 0.3));

    // The kit's a11y Increment/Decrement call set_value with no SliderEvent.
    cx.update(|cx| {
        handle
            .update(cx, |_, window, cx| {
                volume.update(cx, |state, cx| state.set_value(0.42, window, cx));
            })
            .unwrap();
    });
    cx.read(|cx| {
        assert_eq!(
            crate::system_controls::snapshot(cx).volume,
            ControlState::Ready(VolumeStatus {
                level: 0.42,
                muted: false
            }),
            "the a11y adjustment must reach the control"
        );
        assert_eq!(volume.read(cx).value().start(), 0.42);
    });

    let mut stale = snapshot_for(BluetoothAccess::NotRequested, pending());
    stale.volume = ControlState::Ready(VolumeStatus {
        level: 0.3,
        muted: false,
    });
    cx.update(|cx| cx.set_global(stale));
    cx.read(|cx| {
        assert_eq!(
            volume.read(cx).value().start(),
            0.42,
            "the committed a11y value must not be reverted within the window"
        )
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
