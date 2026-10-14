use gpui_kit::{
    AnyView, App, AppContext as _, Context, IntoElement, Render, TestAppContext, Window, div, px,
    size,
};

use super::*;
use crate::ui::floating::{FloatingSpec, FloatingWindowId, init, spawn_floating};

struct Blank;

impl Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

fn blank_view(_: &mut Window, cx: &mut App) -> AnyView {
    cx.new(|_| Blank).into()
}

#[gpui_kit::test]
async fn returns_false_off_wayland_without_panicking(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init);
    let handle = cx
        .update(|cx| {
            spawn_floating(
                FloatingSpec::new(
                    FloatingWindowId("passthrough".into()),
                    size(px(120.), px(90.)),
                ),
                blank_view,
                cx,
            )
        })
        .expect("float spawns on the test platform");
    assert!(!cx.update(|cx| set_input_passthrough(&handle, true, cx)));
    assert!(!cx.update(|cx| set_input_passthrough(&handle, false, cx)));
    let overlay = FloatingHandle::Overlay(FloatingWindowId("overlay".into()));
    assert!(!cx.update(|cx| set_input_passthrough(&overlay, true, cx)));
    assert!(!cx.update(|cx| set_input_passthrough(&overlay, false, cx)));
}
