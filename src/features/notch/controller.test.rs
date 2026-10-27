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

fn demo_window(cx: &TestAppContext) -> gpui_kit::AnyWindowHandle {
    let handle = cx
        .read(|cx| crate::ui::floating::floating_snapshot(cx))
        .first()
        .expect("the demo float must be registered")
        .handle
        .clone();
    let crate::ui::floating::FloatingHandle::Native(window) = handle else {
        panic!("the demo float must spawn as a native window");
    };
    window
}

#[gpui_kit::test]
#[cfg(not(target_os = "macos"))]
async fn hover_expands_the_demo_and_its_close_button_dismisses_it(cx: &TestAppContext) {
    use gpui_kit::{Bounds, Modifiers, VisualTestContext, point, px, size};

    cx.update(boot);
    cx.update(|cx| assert!(toggle(cx)));
    let bounds = cx.read(|cx| crate::ui::floating::floating_snapshot(cx))[0].bounds;
    assert_eq!(
        bounds,
        Bounds::new(point(px(780.), px(8.)), size(px(360.), px(232.))),
        "the demo must spawn top-centered, inset 8px, sized for the expanded stack"
    );
    let mut vcx = VisualTestContext::from_window(demo_window(cx), cx);
    // Hover the panel body (window-local y > the h_8 float header).
    vcx.simulate_mouse_move(point(px(180.), px(120.)), None, Modifiers::none());
    vcx.update(|window, cx| window.draw(cx).clear(cx));
    // The expanded action row sits under the widget stack (h_8 shell header
    // plus the stack), right-aligned. Nothing else in that strip closes the
    // window, so scanning it proves the close button is on-screen and works.
    for x in [330., 316., 300.] {
        vcx.simulate_click(point(px(x), px(162.)), Modifiers::none());
        if cx
            .read(|cx| crate::ui::floating::floating_snapshot(cx))
            .is_empty()
        {
            break;
        }
    }
    assert!(
        cx.read(|cx| crate::ui::floating::floating_snapshot(cx))
            .is_empty(),
        "the close button must be on-screen once expanded and dismiss the float"
    );
    assert!(!cx.read(is_open));
}

#[gpui_kit::test]
#[cfg(not(target_os = "macos"))]
async fn escape_dismisses_the_demo_float_and_clears_notch_state(cx: &TestAppContext) {
    use gpui_kit::{Modifiers, VisualTestContext, point, px};

    cx.update(boot);
    cx.update(|cx| assert!(toggle(cx)));
    let mut vcx = VisualTestContext::from_window(demo_window(cx), cx);
    // Click below the h_8 header: the header drag calls start_window_move,
    // which is unimplemented on the test platform.
    vcx.simulate_click(point(px(180.), px(120.)), Modifiers::none());
    vcx.simulate_keystrokes("escape");
    assert!(
        cx.read(|cx| crate::ui::floating::floating_snapshot(cx))
            .is_empty(),
        "Esc must dismiss the chromeless demo float"
    );
    assert!(!cx.read(is_open));
}
