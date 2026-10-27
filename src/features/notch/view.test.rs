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

// 360x232 mirrors the demo float: big enough that the expanded stack and its
// action buttons layout inside the window.
fn open_demo(
    cx: &TestAppContext,
) -> (
    gpui_kit::AnyWindowHandle,
    gpui_kit::Entity<NotchFloaterView>,
) {
    cx.update(|cx| {
        gpui_kit::open_window(
            WindowOptions {
                window_bounds: Some(gpui_kit::WindowBounds::Windowed(Bounds::new(
                    point(px(0.), px(0.)),
                    size(px(360.), px(232.)),
                ))),
                ..Default::default()
            },
            cx,
            |_, cx| cx.new(demo_view),
        )
    })
    .unwrap()
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

#[gpui_kit::test]
async fn hover_transitions_through_real_events_expand_and_collapse(cx: &TestAppContext) {
    use gpui_kit::{Modifiers, VisualTestContext, point, px};

    cx.update(gpui_kit::init);
    let (window, view) = open_demo(cx);
    let mut vcx = VisualTestContext::from_window(window, cx);
    vcx.simulate_mouse_move(point(px(180.), px(120.)), None, Modifiers::none());
    vcx.update(|window, cx| window.draw(cx).clear(cx));
    assert!(
        cx.read(|cx| view.read(cx).is_expanded()),
        "moving the pointer onto the pill must expand it"
    );
    vcx.simulate_mouse_move(point(px(4000.), px(4000.)), None, Modifiers::none());
    assert!(
        !cx.read(|cx| view.read(cx).is_expanded()),
        "leaving the window must collapse an unpinned floater"
    );
}

#[gpui_kit::test]
async fn pressing_the_action_buttons_does_not_flip_the_pin(cx: &TestAppContext) {
    use gpui_kit::{Modifiers, MouseButton, VisualTestContext, point, px};

    cx.update(gpui_kit::init);
    let (window, view) = open_demo(cx);
    let mut vcx = VisualTestContext::from_window(window, cx);
    vcx.simulate_mouse_move(point(px(180.), px(120.)), None, Modifiers::none());
    vcx.update(|window, cx| window.draw(cx).clear(cx));
    // The expanded action row sits under the widget stack, right-aligned;
    // a press anywhere in it (buttons or their container) must not flip the pin.
    vcx.simulate_mouse_down(
        point(px(330.), px(130.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    assert!(
        !cx.read(|cx| view.read(cx).pinned),
        "a press on the action row must not toggle the pin"
    );
    vcx.simulate_mouse_down(
        point(px(60.), px(80.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    assert!(
        cx.read(|cx| view.read(cx).pinned),
        "a press on the panel body must toggle the pin"
    );
}
