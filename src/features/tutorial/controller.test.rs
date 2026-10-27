// No glob `use gpui_kit::*` here: it would import gpui's `test` proc-macro and
// shadow this file's `#[gpui_kit::test]` attribute.
use gpui_kit::{Keystroke, TestAppContext};

#[test]
fn tour_keystrokes_parse() {
    for key in ["right", "left", "enter", "escape"] {
        assert!(
            Keystroke::parse(key).is_ok(),
            "binding constant {key} must parse"
        );
    }
}

#[gpui_kit::test]
fn start_without_handle_is_a_noop(cx: &mut TestAppContext) {
    cx.update(crate::features::tutorial::start);
    assert!(
        cx.read(|cx| {
            cx.try_global::<crate::features::tutorial::target::TargetRegistry>()
                .is_none()
        }),
        "a rejected start must not touch the registry global"
    );
}
