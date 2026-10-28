use gpui_kit::prelude::*;
use gpui_kit::{App, TestAppContext, Window, div, px, size};

use crate::ui::floating::{
    FloatingSpec, FloatingWindowId, close_floating, floating_snapshot, init,
};

use super::{
    close_all_examples, dictation::DICTATION_ID, entry_handle, menu_card::MENU_ID,
    sketch::SKETCH_ID, spawn_dictation, spawn_menu_card, spawn_sketch,
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
