use std::rc::Rc;

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
    // A half-pixel move stays under the dedup threshold and must not add a
    // point, so the stroke still carries only the mouse-down position.
    cx.simulate_mouse_move(
        start + point(px(0.5), px(0.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    cx.run_until_parked();
    cx.update(|_, cx| {
        section.update(cx, |state, _| {
            assert_eq!(
                state.current_stroke.as_ref().unwrap().points.len(),
                1,
                "sub-pixel moves are deduped"
            );
        });
    });
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
            // Points are canvas-local: the drag visited (20,20), (50,30)
            // and (80,45) relative to the canvas origin.
            assert_eq!(state.strokes[0].points.len(), 3);
            assert_eq!(state.strokes[0].points[0], point(px(20.), px(20.)));
            assert_eq!(state.strokes[0].points[1], point(px(50.), px(30.)));
            assert_eq!(state.strokes[0].points[2], point(px(80.), px(45.)));
            assert!(!state.is_drawing, "drawing state cleared on mouse up");
        });
    });
}

#[gpui_kit::test]
fn releasing_outside_the_canvas_ends_the_stroke(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let (section, cx) = cx.add_window_view(|window, cx| BrushSection::new(window, cx));
    cx.run_until_parked();

    let bounds = cx
        .update(|_, cx| section.update(cx, |state, _| state.canvas_bounds))
        .expect("canvas bounds recorded after the first draw");
    let start = bounds.origin + point(px(20.), px(20.));

    cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(
        start + point(px(30.), px(10.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    // The button was released off-canvas, so no mouse-up arrives: only a
    // button-less move when the cursor re-enters.
    cx.simulate_mouse_move(start + point(px(40.), px(15.)), None, Modifiers::none());
    cx.run_until_parked();

    cx.update(|_, cx| {
        section.update(cx, |state, _| {
            assert_eq!(
                state.strokes.len(),
                1,
                "stroke committed on release-outside"
            );
            assert!(!state.is_drawing, "drawing disarmed without a mouse-up");
            assert!(state.current_stroke.is_none());
        });
    });

    cx.simulate_mouse_move(start + point(px(60.), px(25.)), None, Modifiers::none());
    cx.run_until_parked();

    cx.update(|_, cx| {
        section.update(cx, |state, _| {
            assert_eq!(
                state.strokes[0].points.len(),
                2,
                "no points appended while no button is held"
            );
        });
    });
}

#[gpui_kit::test]
fn mouse_up_outside_the_canvas_commits_the_stroke(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let (section, cx) = cx.add_window_view(|window, cx| BrushSection::new(window, cx));
    cx.run_until_parked();

    let bounds = cx
        .update(|_, cx| section.update(cx, |state, _| state.canvas_bounds))
        .expect("canvas bounds recorded after the first draw");
    let start = bounds.origin + point(px(20.), px(20.));

    cx.simulate_mouse_down(start, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(
        start + point(px(30.), px(10.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    cx.simulate_mouse_up(
        bounds.origin + point(px(10.), bounds.size.height + px(20.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    cx.run_until_parked();

    cx.update(|_, cx| {
        section.update(cx, |state, _| {
            assert_eq!(
                state.strokes.len(),
                1,
                "off-canvas release commits the stroke"
            );
            assert!(!state.is_drawing, "release outside disarms the canvas");
            assert!(state.current_stroke.is_none());
        });
    });
}

#[gpui_kit::test]
fn clicking_without_moving_leaves_a_dot(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let (section, cx) = cx.add_window_view(|window, cx| BrushSection::new(window, cx));
    cx.run_until_parked();

    let bounds = cx
        .update(|_, cx| section.update(cx, |state, _| state.canvas_bounds))
        .expect("canvas bounds recorded after the first draw");
    let at = bounds.origin + point(px(40.), px(30.));

    cx.simulate_mouse_down(at, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(at, MouseButton::Left, Modifiers::none());
    cx.run_until_parked();

    let cached = cx.update(|_, cx| section.update(cx, |state, _| state.stroke_paths.clone()));
    cx.run_until_parked();

    cx.update(|_, cx| {
        section.update(cx, |state, _| {
            assert_eq!(state.strokes.len(), 1, "a click commits a one-point stroke");
            assert_eq!(state.strokes[0].points.len(), 1);
            assert_eq!(
                state.stroke_paths.len(),
                state.strokes.len(),
                "committed strokes are tessellated"
            );
            assert!(
                Rc::ptr_eq(&cached, &state.stroke_paths),
                "a repaint without new strokes reuses the tessellated paths"
            );
        });
    });
}
