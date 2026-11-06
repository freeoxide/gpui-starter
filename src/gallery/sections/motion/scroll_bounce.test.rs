use std::time::Duration;

use gpui_kit::{
    ListAlignment, ListState, Modifiers, MouseButton, ScrollHandle, TestAppContext, point, px,
};

use super::ScrollBounceSection;

#[gpui_kit::test]
fn mouse_drag_scrolls_and_stretches_the_example_list(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (section, cx) = cx.add_window_view(|_window, _cx| ScrollBounceSection {
        list: ListState::new(super::ROWS, ListAlignment::Top, px(200.)).measure_all(),
        short: ScrollHandle::new(),
        enabled: true,
        wrapper_wheel: true,
        generation: 0,
        drag: None,
    });
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let handle = cx.read(|cx| section.read(cx).list.clone());
    let bounds = handle.viewport_bounds();
    assert!(
        bounds.size.height > px(100.),
        "list viewport must be laid out, got {bounds:?}"
    );
    let anchor = bounds.origin + point(px(50.), px(50.));

    cx.simulate_mouse_down(anchor, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(
        anchor + point(px(0.), px(100.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert_eq!(handle.scroll_px_offset_for_scrollbar().y, px(0.));
    assert!(
        handle.viewport_bounds().origin.y > bounds.origin.y,
        "dragging past the top must stretch the list, not scroll it"
    );

    cx.simulate_mouse_move(
        anchor - point(px(0.), px(40.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert_eq!(handle.scroll_px_offset_for_scrollbar().y, px(-40.));

    cx.simulate_mouse_up(anchor, MouseButton::Left, Modifiers::none());
    assert!(
        cx.read(|cx| section.read(cx).drag.is_none()),
        "release must end the drag"
    );

    // Pull past the top again, then release without reversing. Verify real
    // prepaint displacement decreases as the spring clock advances.
    cx.simulate_mouse_down(anchor, MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(
        anchor + point(px(0.), px(140.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let stretched = handle.viewport_bounds().origin.y;
    assert!(stretched > bounds.origin.y);
    cx.simulate_mouse_up(
        anchor + point(px(0.), px(140.)),
        MouseButton::Left,
        Modifiers::none(),
    );
    // The shared overscroll engine steps on the executor's virtual clock, so
    // the return needs clock advances between frames. A spring that settles
    // between polls also passes.
    let mut returning = stretched;
    for _ in 0..60 {
        cx.executor().advance_clock(Duration::from_millis(16));
        cx.update(|window, cx| window.draw(cx).clear(cx));
        returning = handle.viewport_bounds().origin.y;
        if returning < stretched {
            break;
        }
    }
    assert!(
        returning >= bounds.origin.y && returning < stretched,
        "spring must return from {stretched:?} toward {}, got {returning:?}",
        bounds.origin.y
    );
}
