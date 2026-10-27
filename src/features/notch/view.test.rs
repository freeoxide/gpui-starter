use gpui_kit::{AppContext as _, Bounds, Context, TestAppContext, WindowOptions, point, px, size};

use super::NotchFloaterView;
use crate::features::notch::geometry::notch_layout;

fn demo_view(cx: &mut Context<NotchFloaterView>) -> NotchFloaterView {
    NotchFloaterView::new(
        notch_layout(
            Bounds::new(point(px(0.), px(0.)), size(px(360.), px(140.))),
            None,
            None,
        ),
        false,
        cx,
    )
}

#[gpui_kit::test]
async fn hover_and_pin_drive_expanded_state(cx: &TestAppContext) {
    cx.update(gpui_kit::init);
    let (window, view) = cx
        .update(|cx| gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| cx.new(demo_view)))
        .unwrap();
    cx.update(|cx| {
        window
            .update(cx, |_, window, cx| {
                view.update(cx, |this, cx| this.toggle_pinned(window, cx));
                assert!(view.read(cx).is_expanded());
            })
            .unwrap();
    });
    cx.update(|cx| {
        window
            .update(cx, |_, window, cx| {
                view.update(cx, |this, cx| this.set_hovered(true, window, cx));
                view.update(cx, |this, cx| this.set_hovered(false, window, cx));
                assert!(
                    view.read(cx).is_expanded(),
                    "a held pin keeps the floater expanded when hover ends"
                );
            })
            .unwrap();
    });
    cx.update(|cx| {
        window
            .update(cx, |_, window, cx| {
                view.update(cx, |this, cx| this.toggle_pinned(window, cx));
                assert!(!view.read(cx).is_expanded());
            })
            .unwrap();
    });
}

#[gpui_kit::test]
async fn render_smoke_does_not_panic(cx: &TestAppContext) {
    cx.update(gpui_kit::init);
    let (_, view) = cx
        .update(|cx| gpui_kit::open_window(WindowOptions::default(), cx, |_, cx| cx.new(demo_view)))
        .unwrap();
    let minute = cx.read(|cx| view.read(cx).minute.clone());
    assert_eq!(minute.len(), 5, "the clock seeds an HH:MM string");
}
