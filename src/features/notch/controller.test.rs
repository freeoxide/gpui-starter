use gpui_kit::TestAppContext;

use super::{is_open, toggle};
use crate::features::notch::CAPABILITY_NOTCH_FLOATER;

fn boot(cx: &mut gpui_kit::App) {
    gpui_kit::init(cx);
    crate::ui::floating::init(cx);
    crate::features::notch::init(cx);
}

#[gpui_kit::test]
async fn init_registers_capability(cx: &TestAppContext) {
    cx.update(boot);
    let status = cx
        .read(|cx| crate::capabilities::snapshot(cx))
        .get(CAPABILITY_NOTCH_FLOATER)
        .cloned();
    assert!(
        status.is_some_and(|status| status.supported),
        "notch_floater must be registered as supported on every host"
    );
}

#[gpui_kit::test]
async fn toggle_opens_and_closes_floater(cx: &TestAppContext) {
    cx.update(boot);
    cx.update(|cx| assert!(!is_open(cx)));
    cx.update(|cx| assert!(toggle(cx)));
    cx.update(|cx| assert!(is_open(cx)));
    #[cfg(not(target_os = "macos"))]
    cx.update(|cx| {
        assert_eq!(
            crate::ui::floating::floating_snapshot(cx).len(),
            1,
            "open registers the demo float"
        );
    });
    cx.update(|cx| assert!(!toggle(cx)));
    cx.update(|cx| assert!(!is_open(cx)));
    #[cfg(not(target_os = "macos"))]
    cx.update(|cx| {
        assert_eq!(
            crate::ui::floating::floating_snapshot(cx).len(),
            0,
            "close removes the demo float from the registry"
        );
    });
}

#[gpui_kit::test]
async fn toggle_is_idempotent_across_cycles(cx: &TestAppContext) {
    cx.update(boot);
    cx.update(|cx| assert!(toggle(cx)));
    cx.update(|cx| assert!(!toggle(cx)));
    cx.update(|cx| assert!(toggle(cx)));
    cx.update(|cx| assert!(is_open(cx)));
    #[cfg(not(target_os = "macos"))]
    cx.update(|cx| assert_eq!(crate::ui::floating::floating_snapshot(cx).len(), 1));
}

#[gpui_kit::test]
#[cfg(not(target_os = "macos"))]
async fn external_float_close_clears_state(cx: &TestAppContext) {
    cx.update(boot);
    cx.update(|cx| assert!(toggle(cx)));
    cx.update(|cx| crate::ui::floating::close_all_floating(cx));
    cx.update(|cx| {
        assert!(
            !is_open(cx),
            "the registry watcher must clear notch state on an external close"
        );
    });
}
