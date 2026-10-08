use gpui_kit::{Modifiers, MouseButton, TestAppContext, point, px};

use super::*;

#[gpui_kit::test]
fn dragging_on_the_canvas_records_a_stroke(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let (section, cx) = cx.add_window_view(|window, cx| BrushSection::new(window, cx));
    cx.run_until_parked();

    let bounds = cx
        .update(|_, cx| section.update(cx, |state, _| state.canvas_bounds))
        .expect("canvas bounds recorded after the first draw");
    assert!(
        bounds.size.height > px(0.),
        "canvas must resolve a non-zero height, got {bounds:?}"
    );
    let start = bounds.origin + point(px(20.), px(20.));

    cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(
        start + point(px(30.), px(10.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    cx.simulate_mouse_move(
        start + point(px(60.), px(25.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    cx.simulate_mouse_up(
        start + point(px(60.), px(25.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    cx.run_until_parked();

    cx.update(|_, cx| {
        section.update(cx, |state, _| {
            assert_eq!(
                state.strokes.len(),
                1,
                "one stroke committed after the drag"
            );
            assert!(!state.strokes[0].points.is_empty());
            assert!(!state.is_drawing, "drawing state cleared on mouse up");
        });
    });
}
