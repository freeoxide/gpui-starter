use gpui_kit::{App, TestAppContext};

use crate::shortcuts::{self, RegistryError};
use crate::ui::floating::{floating_snapshot, init};

use super::super::dictation::DICTATION_ID;
use super::{install_quick_float, toggle_quick_float};

fn dictation_open(cx: &App) -> bool {
    floating_snapshot(cx)
        .iter()
        .any(|entry| entry.id.0 == DICTATION_ID)
}

#[gpui_kit::test]
fn toggle_quick_float_spawns_then_closes_with_no_main_window(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init);
    cx.update(|cx| toggle_quick_float(cx));
    assert!(
        cx.update(|cx| dictation_open(cx)),
        "the first trigger opens the screen float"
    );
    cx.update(|cx| toggle_quick_float(cx));
    assert!(
        !cx.update(|cx| dictation_open(cx)),
        "the second trigger closes it"
    );
}

#[gpui_kit::test]
fn install_routes_the_accelerator_to_the_toggle(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init);
    cx.update(|cx| install_quick_float(cx)).expect("install");

    let fired = cx.update(|cx| shortcuts::trigger(shortcuts::QUICK_FLOAT, cx));
    assert!(fired, "the handler must be wired");
    assert!(cx.update(|cx| dictation_open(cx)));

    let fired = cx.update(|cx| shortcuts::trigger(shortcuts::QUICK_FLOAT, cx));
    assert!(fired);
    assert!(!cx.update(|cx| dictation_open(cx)));

    assert!(cx.update(|cx| shortcuts::remove_handler(shortcuts::QUICK_FLOAT, cx)));
    let fired = cx.update(|cx| shortcuts::trigger(shortcuts::QUICK_FLOAT, cx));
    assert!(!fired, "an unregistered accelerator must not fire");
    assert!(!cx.update(|cx| dictation_open(cx)));
}

#[gpui_kit::test]
fn a_second_installation_is_rejected(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(|cx| install_quick_float(cx))
        .expect("first install");
    let second = cx.update(|cx| install_quick_float(cx));
    assert_eq!(second.err(), Some(RegistryError::DuplicateId));
    cx.update(|cx| shortcuts::remove_handler(shortcuts::QUICK_FLOAT, cx));
}
