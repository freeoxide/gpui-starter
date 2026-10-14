use gpui_kit::{
    AnyView, App, AppContext as _, Context, IntoElement, Render, TestAppContext, Window, div, px,
    size,
};

use super::*;
use crate::ui::floating::{
    FloatSurface, FloatingHandle, FloatingWindowId, floating_snapshot, init,
};

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
async fn spawns_immediately_when_displays_are_enumerable(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init);
    assert!(!cx.read(|cx| cx.displays().is_empty()));
    cx.update(|cx| {
        spawn_floating_after_displays(
            FloatingSpec::new(FloatingWindowId("deferred".into()), size(px(120.), px(90.))),
            blank_view,
            cx,
        )
    });
    let snapshot = cx.read(|cx| floating_snapshot(cx));
    assert_eq!(snapshot.len(), 1);
    assert_eq!(snapshot[0].id.0, "deferred");
    assert!(matches!(snapshot[0].handle, FloatingHandle::Native(_)));
    assert_eq!(snapshot[0].surface, FloatSurface::Window);
}
