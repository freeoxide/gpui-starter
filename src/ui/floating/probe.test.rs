use gpui_kit::TestAppContext;

use super::*;
use crate::ui::floating::init;

#[gpui_kit::test]
async fn probe_facts_agree_with_the_capability_rows(cx: &TestAppContext) {
    cx.update(init);
    let (info, snapshot) = cx.read(|cx| {
        let snapshot = crate::capabilities::snapshot(cx);
        (probe(cx), snapshot)
    });
    assert_eq!(
        info.native_windows,
        snapshot[&Capability::FLOATING_WINDOWS].supported,
        "probe and the capability row must not drift apart"
    );
    assert_eq!(
        info.always_on_top,
        snapshot[&Capability::FLOATING_ALWAYS_ON_TOP].supported,
        "probe and the capability row must not drift apart"
    );
    if info.notch.is_some() {
        assert!(
            snapshot[&Capability::NOTCH_GEOMETRY].supported,
            "live notch data implies the capability row is supported"
        );
    }
}

#[gpui_kit::test]
async fn probe_fills_the_cache_once_init_ran(cx: &TestAppContext) {
    cx.update(init);
    let first = cx.read(probe);
    let filled = cx.read(|cx| {
        cx.try_global::<PlatformInfoCache>()
            .is_some_and(|cache| cache.0.borrow().is_some())
    });
    assert!(filled, "first probe() after init must populate the cache");
    assert_eq!(cx.read(probe), first);
}

#[gpui_kit::test]
async fn init_registers_the_four_capability_rows(cx: &TestAppContext) {
    cx.update(init);
    let snapshot = cx.read(crate::capabilities::snapshot);
    for capability in [
        Capability::FLOATING_WINDOWS,
        Capability::FLOATING_ALWAYS_ON_TOP,
        Capability::FLOATING_CLICK_THROUGH,
        Capability::NOTCH_GEOMETRY,
    ] {
        assert!(
            snapshot.contains_key(&capability),
            "missing capability {capability}"
        );
    }
    assert!(snapshot[&Capability::FLOATING_WINDOWS].supported);
    assert!(snapshot[&Capability::FLOATING_ALWAYS_ON_TOP].supported);
    assert!(snapshot[&Capability::FLOATING_CLICK_THROUGH].supported);
}
