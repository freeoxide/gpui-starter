use std::time::Duration;

use gpui_kit::TestAppContext;
use gpui_kit::component::slider::{SliderEvent, SliderValue};

use crate::platform::clock::Instant;
use crate::system_controls::{
    BluetoothAccess, BrightnessStatus, ControlState, SystemControlsSnapshot, VolumeStatus,
};

use super::{SliderTarget, SystemSlidersWidget, level_of, percent_of, should_sync};

#[test]
fn percent_of_clamps_both_ends_and_rounds() {
    assert_eq!(percent_of(0.), 0.);
    assert_eq!(percent_of(1.), 100.);
    assert_eq!(percent_of(-0.5), 0.);
    assert_eq!(percent_of(1.5), 100.);
    assert_eq!(percent_of(0.125), 13.);
}

#[test]
fn level_of_clamps_into_the_unit_range() {
    assert_eq!(level_of(0.), 0.);
    assert_eq!(level_of(100.), 1.);
    assert_eq!(level_of(-10.), 0.);
    assert_eq!(level_of(200.), 1.);
    assert_eq!(level_of(50.), 0.5);
}

#[test]
fn should_sync_allows_all_when_no_deadline_is_set() {
    let now = Instant::now();
    assert!(should_sync(now, None));
}

#[test]
fn should_sync_blocks_before_the_deadline_and_passes_at_it() {
    let now = Instant::now();
    let deadline = now + Duration::from_secs(2);
    assert!(!should_sync(now, Some(deadline)));
    assert!(!should_sync(
        deadline - Duration::from_millis(1),
        Some(deadline)
    ));
    assert!(should_sync(deadline, Some(deadline)));
    assert!(should_sync(
        deadline + Duration::from_secs(1),
        Some(deadline)
    ));
}

fn pending<T>() -> ControlState<T> {
    ControlState::Unavailable {
        error: "pending first read".into(),
    }
}

fn ready_snapshot(volume: f32, brightness: f32) -> SystemControlsSnapshot {
    SystemControlsSnapshot {
        battery: pending(),
        bluetooth: pending(),
        volume: ControlState::Ready(VolumeStatus {
            level: volume,
            muted: false,
        }),
        brightness: ControlState::Ready(BrightnessStatus { level: brightness }),
        bluetooth_access: BluetoothAccess::NotRequested,
    }
}

#[gpui_kit::test]
async fn an_external_snapshot_cannot_stomp_a_drag_in_progress(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    cx.update(|cx| cx.set_global(ready_snapshot(0.5, 0.4)));
    let handle = cx.add_window(SystemSlidersWidget::new);
    let widget = handle.root(cx).expect("widget entity");
    cx.read(|cx| {
        let widget = widget.read(cx);
        assert_eq!(widget.volume.read(cx).value().start(), 50.);
        assert_eq!(widget.brightness.read(cx).value().start(), 40.);
    });

    // WindowHandle::update already leases the widget; use its root directly.
    handle
        .update(cx, |widget, window, cx| {
            widget
                .volume
                .update(cx, |state, cx| state.set_value(30., window, cx));
            widget.on_event(
                SliderTarget::Volume,
                &SliderEvent::Change(SliderValue::Single(30.)),
                cx,
            );
        })
        .unwrap();

    cx.update(|cx| cx.set_global(ready_snapshot(0.9, 0.7)));
    cx.read(|cx| {
        let widget = widget.read(cx);
        assert_eq!(
            widget.volume.read(cx).value().start(),
            30.,
            "a suppressed sync must not stomp the dragged slider"
        );
        assert_eq!(widget.brightness.read(cx).value().start(), 40.);
    });

    // The 2s suppression window has elapsed; the next external change syncs.
    cx.update(|cx| {
        widget.update(cx, |widget, _| widget.suppress_until = None);
    });
    cx.update(|cx| cx.set_global(ready_snapshot(0.85, 0.6)));
    cx.read(|cx| {
        let widget = widget.read(cx);
        assert_eq!(widget.volume.read(cx).value().start(), 85.);
        assert_eq!(widget.brightness.read(cx).value().start(), 60.);
    });
}
