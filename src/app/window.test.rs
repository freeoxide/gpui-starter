use gpui_kit::{
    AnyWindowHandle, AppContext as _, Context, IntoElement, Render, TestAppContext, Window, div,
    px, size,
};

use super::*;

struct Blank;

impl Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

#[gpui_kit::test]
async fn closing_the_recorded_root_clears_the_global(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(install_close_tracker);
    let window = cx.open_window(size(px(800.), px(600.)), |_, _| Blank);
    let root: AnyWindowHandle = window.into();
    cx.update(|cx| note_root_window(root, cx));
    assert_eq!(cx.read(root_window), Some(root));
    cx.update(|cx| {
        root.update(cx, |_, window, _| window.remove_window())
            .unwrap();
    });
    assert_eq!(
        cx.read(root_window),
        None,
        "a stale root handle must not survive its window"
    );
}

#[gpui_kit::test]
async fn closing_an_unrelated_window_keeps_the_recorded_root(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(install_close_tracker);
    let root_window_handle = cx.open_window(size(px(800.), px(600.)), |_, _| Blank);
    let other = cx.open_window(size(px(400.), px(300.)), |_, _| Blank);
    let root: AnyWindowHandle = root_window_handle.into();
    cx.update(|cx| note_root_window(root, cx));
    cx.update(|cx| {
        other
            .update(cx, |_, window, _| window.remove_window())
            .unwrap();
    });
    assert_eq!(
        cx.read(root_window),
        Some(root),
        "only the recorded window's close may clear the record"
    );
}
