use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::{App, TestAppContext};

use super::*;

#[gpui_kit::test]
async fn set_and_get_round_trip_a_single_row(cx: &mut TestAppContext) {
    cx.update(initialize);
    assert!(
        cx.read(|cx| get(Capability::AUDIO_PLAYBACK, cx)).is_none(),
        "a row nobody wrote must read as absent"
    );
    cx.update(|cx| {
        set(
            Capability::AUDIO_PLAYBACK,
            CapabilityStatus::degraded("no output device"),
            cx,
        )
    });
    let status = cx
        .read(|cx| get(Capability::AUDIO_PLAYBACK, cx))
        .expect("row present after set");
    assert!(status.supported && status.enabled && status.degraded);
    assert_eq!(status.reason.as_deref(), Some("no output device"));
    let snapshot = cx.read(|cx| snapshot(cx));
    assert_eq!(snapshot.len(), 1);
    assert_eq!(snapshot[&Capability::AUDIO_PLAYBACK], status);
}

#[gpui_kit::test]
async fn identical_rewrites_do_not_notify_observers(cx: &mut TestAppContext) {
    cx.update(initialize);
    let notifies = Rc::new(Cell::new(0u32));
    let counter = notifies.clone();
    cx.update(|cx| {
        cx.observe_global::<CapabilityRegistry>(move |_: &mut App| counter.set(counter.get() + 1))
            .detach();
    });
    cx.update(|cx| {
        set(
            Capability::SYSTEM_TRAY,
            CapabilityStatus::supported_enabled(),
            cx,
        )
    });
    assert_eq!(notifies.get(), 1);
    cx.update(|cx| {
        set(
            Capability::SYSTEM_TRAY,
            CapabilityStatus::supported_enabled(),
            cx,
        )
    });
    assert_eq!(
        notifies.get(),
        1,
        "a rewrite equal to the stored status must not notify"
    );
    cx.update(|cx| {
        set(
            Capability::SYSTEM_TRAY,
            CapabilityStatus::degraded("event drain died"),
            cx,
        )
    });
    assert_eq!(notifies.get(), 2);
}
