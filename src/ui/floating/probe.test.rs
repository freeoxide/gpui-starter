use gpui_kit::TestAppContext;

use super::*;
use crate::ui::floating::{
    CAPABILITY_ALWAYS_ON_TOP, CAPABILITY_FLOATING_WINDOWS, CAPABILITY_NOTCH_GEOMETRY, init,
};

#[gpui_kit::test]
async fn probe_reports_native_windows_only_off_wasm(cx: &TestAppContext) {
    cx.update(init);
    let info = cx.read(|cx| probe(cx));
    if cfg!(target_family = "wasm") {
        assert!(!info.native_windows);
        assert!(!info.always_on_top);
        assert!(!info.non_focus_stealing);
        assert_eq!(info.menu_bar, None);
    } else {
        assert!(info.native_windows);
        assert!(info.always_on_top);
        assert!(info.menu_bar.is_some());
    }
}

#[gpui_kit::test]
async fn probe_never_panics_and_reports_no_notch_off_macos(cx: &TestAppContext) {
    let info = cx.read(|cx| probe(cx));
    #[cfg(not(target_os = "macos"))]
    assert!(info.notch.is_none());
    #[cfg(target_os = "macos")]
    let _ = info.notch;
}

#[gpui_kit::test]
async fn init_registers_the_three_capability_rows(cx: &TestAppContext) {
    cx.update(init);
    let snapshot = cx.read(|cx| crate::capabilities::snapshot(cx));
    for key in [
        CAPABILITY_FLOATING_WINDOWS,
        CAPABILITY_ALWAYS_ON_TOP,
        CAPABILITY_NOTCH_GEOMETRY,
    ] {
        assert!(snapshot.contains_key(key), "missing capability {key}");
    }
    if cfg!(target_family = "wasm") {
        let floating = &snapshot[CAPABILITY_FLOATING_WINDOWS];
        assert!(!floating.supported);
        assert!(floating.enabled);
        assert!(floating.degraded);
        assert!(floating.reason.is_some());
    } else {
        assert!(snapshot[CAPABILITY_FLOATING_WINDOWS].supported);
        assert!(snapshot[CAPABILITY_ALWAYS_ON_TOP].supported);
    }
}
