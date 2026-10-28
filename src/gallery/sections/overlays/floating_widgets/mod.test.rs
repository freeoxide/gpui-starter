use gpui_kit::prelude::*;
use gpui_kit::{App, TestAppContext, VisualTestContext, Window, div, point, px, size};

use crate::ui::floating::{
    FloatAnchor, FloatLevel, FloatingHandle, FloatingSpec, FloatingWindowId, close_floating,
    floating_snapshot, init,
};

use super::{
    INDEPENDENT_ID, OFF_WINDOW_ID, close_all_examples, dictation::DICTATION_ID, entry_handle,
    independent_spec, menu_card::MENU_ID, off_window_spec, sketch::SKETCH_ID, spawn_dictation,
    spawn_independent, spawn_menu_card, spawn_off_window, spawn_sketch,
};

struct Blank;

impl Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

#[gpui_kit::test]
fn entry_handle_finds_only_open_ids(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init);
    cx.update(|cx| {
        assert!(entry_handle(DICTATION_ID, cx).is_none());
        spawn_dictation(cx).expect("dictation opens");
        assert!(entry_handle(DICTATION_ID, cx).is_some());
        assert!(entry_handle(SKETCH_ID, cx).is_none());
    });
}

#[gpui_kit::test]
fn close_all_examples_closes_only_this_sections_floats(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init);
    cx.update(|cx| {
        spawn_dictation(cx).expect("dictation opens");
        spawn_sketch(cx).expect("sketch opens");
        spawn_menu_card(cx).expect("menu opens");
        let spec = FloatingSpec::new(
            FloatingWindowId("unrelated-float".into()),
            size(px(60.), px(40.)),
        );
        crate::ui::floating::spawn_floating(spec, |_, cx: &mut App| cx.new(|_| Blank).into(), cx)
            .expect("an unrelated float opens");
    });

    let closed = cx.update(close_all_examples);
    assert_eq!(closed, 3);
    cx.update(|cx| {
        let remaining: Vec<String> = floating_snapshot(cx)
            .into_iter()
            .map(|entry| entry.id.0.to_string())
            .collect();
        assert_eq!(remaining, vec!["unrelated-float".to_string()]);
    });

    let handle = cx.update(|cx| entry_handle("unrelated-float", cx)).unwrap();
    cx.update(|cx| assert!(close_floating(&handle, cx)));
    let idle = cx.update(close_all_examples);
    assert_eq!(idle, 0);
}

#[gpui_kit::test]
fn variant_specs_are_chromeless_screen_floats() {
    for spec in [independent_spec(), off_window_spec()] {
        assert!(spec.is_chromeless(), "variant demos drop the header");
        assert!(
            spec.is_focus(),
            "informational HUDs take focus so Esc dismisses"
        );
        assert!(matches!(spec.float_anchor(), FloatAnchor::Screen { .. }));
        assert_eq!(spec.float_level(), FloatLevel::AlwaysOnTop);
    }
}

#[gpui_kit::test]
fn spawn_independent_opens_an_idempotent_float_with_no_main_window(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init);
    let spawned = cx.update(|cx| spawn_independent(cx));
    assert!(matches!(spawned, Ok(FloatingHandle::Native(_))));
    cx.update(|cx| {
        let entry = floating_snapshot(cx)
            .into_iter()
            .find(|entry| entry.id.0 == INDEPENDENT_ID)
            .expect("the independent float registers");
        assert_eq!(
            entry.bounds.origin,
            point(px(1584.), px(16.)),
            "top-right screen corner, inset 16 px"
        );
        assert_eq!(entry.bounds.size, size(px(320.), px(140.)));
    });

    let _again = cx.update(|cx| spawn_independent(cx));
    let count = cx.update(|cx| {
        floating_snapshot(cx)
            .iter()
            .filter(|entry| entry.id.0 == INDEPENDENT_ID)
            .count()
    });
    assert_eq!(count, 1, "a second spawn must reuse the open float");
}

#[gpui_kit::test]
async fn the_off_window_float_lands_outside_the_main_window(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init);
    cx.open_window(size(px(800.), px(600.)), |_, _| Blank);
    let spawned = cx.update(|cx| spawn_off_window(cx));
    assert!(matches!(spawned, Ok(FloatingHandle::Native(_))));
    cx.update(|cx| {
        let entry = floating_snapshot(cx)
            .into_iter()
            .find(|entry| entry.id.0 == OFF_WINDOW_ID)
            .expect("the off-window float registers");
        assert_eq!(
            entry.bounds.origin,
            point(px(16.), px(924.)),
            "bottom-left of the 1920x1080 display, inset 16 px"
        );
        assert!(
            entry.bounds.origin.y > px(600.),
            "outside the 800x600 main window at (0,0); a window-scoped clamp would move it"
        );
    });
}

#[gpui_kit::test]
async fn register_pushes_the_floating_widgets_section(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init);
    let handle = cx.open_window(size(px(400.), px(300.)), |_, _| Blank);
    let mut cx = VisualTestContext::from_window(handle.into(), cx);
    cx.update(|window, cx| {
        let mut sections = vec![];
        super::register(&mut sections, window, cx);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].id(), "floating-widgets");
    });
}
