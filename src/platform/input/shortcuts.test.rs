use super::*;
use gpui_kit::{Keystroke, OperatingSystem, TestAppContext};
use std::cell::Cell;

use crate::capabilities::Capability;

/// Dispatch resolves crate hotkey ids through the spec, so two table rows
/// sharing a spec would route both accelerators to the first handler.
#[test]
fn accelerator_ids_and_hotkey_specs_are_unique() {
    for (i, acc) in ACCELERATORS.iter().enumerate() {
        assert_eq!(accelerator(acc.id).unwrap().id, acc.id);
        for other in ACCELERATORS.iter().skip(i + 1) {
            assert_ne!(acc.id, other.id, "accelerator ids must be unique");
            assert_ne!(
                acc.spec, other.spec,
                "two accelerators share one hotkey; dispatch cannot route them"
            );
        }
    }
    assert!(accelerator("missing").is_none());
}

/// The fallback chord is the only trigger on Linux and wasm, so it must
/// parse into the table's spec on every OS flavor gpui resolves keystrokes for.
#[test]
fn quick_float_chord_parses_on_every_os() {
    let acc = accelerator(QUICK_FLOAT).unwrap();
    let key = match acc.spec.key {
        AcceleratorKey::Space => "space",
        AcceleratorKey::D => "d",
    };
    let mut registered = String::new();
    if acc.spec.alt {
        registered.push_str("alt+");
    }
    if acc.spec.shift {
        registered.push_str("shift+");
    }
    registered.push_str(key);
    assert_eq!(
        acc.label.to_lowercase(),
        registered,
        "label and spec describe different chords"
    );

    for os in [
        OperatingSystem::Mac,
        OperatingSystem::Linux,
        OperatingSystem::Windows,
    ] {
        let stroke = Keystroke::parse_with_os(crate::app::keys::TOGGLE_QUICK_FLOAT, os).unwrap();
        assert_eq!(stroke.modifiers.alt, acc.spec.alt, "{os:?}: alt mismatch");
        assert_eq!(
            stroke.modifiers.shift, acc.spec.shift,
            "{os:?}: shift mismatch"
        );
        assert_eq!(stroke.key.as_str(), key, "{os:?}: key mismatch");
    }
}

fn counter() -> (Rc<Cell<u32>>, impl Fn(&mut App) + Clone + 'static) {
    let hits = Rc::new(Cell::new(0u32));
    let closure = {
        let hits = hits.clone();
        move |_: &mut App| hits.set(hits.get() + 1)
    };
    (hits, closure)
}

#[gpui_kit::test]
async fn trigger_fans_out_to_the_registered_handler_per_accelerator(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (launcher_hits, launcher_handler) = counter();
    let (float_hits, float_handler) = counter();
    cx.update(|cx| {
        set_handler(LAUNCHER, launcher_handler, cx).unwrap();
        set_handler(QUICK_FLOAT, float_handler, cx).unwrap();
    });

    cx.update(|cx| assert!(trigger(LAUNCHER, cx)));
    cx.update(|cx| assert!(trigger(QUICK_FLOAT, cx)));

    assert_eq!(launcher_hits.get(), 1);
    assert_eq!(float_hits.get(), 1);
}

#[gpui_kit::test]
async fn register_rejects_a_duplicate_id_and_keeps_the_first_handler(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (first_hits, first_handler) = counter();
    let (second_hits, second_handler) = counter();
    cx.update(|cx| {
        set_handler(LAUNCHER, first_handler, cx).unwrap();
        assert_eq!(
            set_handler(LAUNCHER, second_handler, cx),
            Err(RegistryError::DuplicateId)
        );
    });

    cx.update(|cx| trigger(LAUNCHER, cx));

    assert_eq!(first_hits.get(), 1);
    assert_eq!(second_hits.get(), 0);
}

#[gpui_kit::test]
async fn register_rejects_an_id_outside_the_accelerator_table(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(|cx| {
        assert_eq!(
            set_handler("not-an-accelerator", |_| {}, cx),
            Err(RegistryError::UnknownAccelerator)
        );
    });
}

#[gpui_kit::test]
async fn trigger_reports_unhandled_when_no_handler_is_wired(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(|cx| assert!(!trigger(QUICK_FLOAT, cx)));
}

/// Handlers run outside the dispatch-table lease, so a handler may wire
/// another accelerator mid-dispatch without tripping a nested-global-update.
#[gpui_kit::test]
async fn a_handler_may_register_another_accelerator_during_dispatch(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (hits, handler) = counter();
    cx.update(|cx| {
        let chained = handler.clone();
        set_handler(
            QUICK_FLOAT,
            move |cx| {
                handler(cx);
                set_handler(LAUNCHER, chained.clone(), cx).unwrap();
            },
            cx,
        )
        .unwrap();
    });

    cx.update(|cx| assert!(trigger(QUICK_FLOAT, cx)));
    cx.update(|cx| assert!(trigger(LAUNCHER, cx)));

    assert_eq!(hits.get(), 2);
}

/// The macOS/Windows forwarders route crate hotkey ids to accelerators
/// through the spec table; the uniqueness test only guards that indirectly.
#[cfg(any(target_os = "macos", target_os = "windows"))]
#[gpui_kit::test]
async fn dispatch_hotkey_fires_the_accelerator_owning_the_hotkey_id(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (hits, handler) = counter();
    cx.update(|cx| {
        set_handler(QUICK_FLOAT, handler, cx).unwrap();
        let hotkey_id = crate_hotkey(accelerator(QUICK_FLOAT).unwrap().spec).id();
        dispatch_hotkey(hotkey_id, cx);
    });

    assert_eq!(hits.get(), 1);
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
#[gpui_kit::test]
async fn the_stub_reports_os_global_shortcuts_unsupported(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(initialize);

    let state = cx.update(|cx| snapshot(cx));
    assert!(!state.registered);
    assert!(!state.enabled);
    assert_eq!(state.last_error.as_deref(), Some(UNSUPPORTED_REASON));

    let capabilities = cx.update(|cx| crate::capabilities::snapshot(cx));
    let status = capabilities.get(&Capability::GLOBAL_SHORTCUTS).unwrap();
    assert!(!status.supported);
    assert!(!status.enabled);
    assert_eq!(
        status.reason.as_deref(),
        Some(format!("shortcut unavailable: {UNSUPPORTED_REASON}").as_str())
    );
}
