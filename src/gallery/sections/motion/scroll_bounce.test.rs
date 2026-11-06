use std::time::Duration;

use gpui_kit::{Modifiers, MouseButton, TestAppContext, point, px};

use super::ScrollBounceSection;

#[gpui_kit::test]
fn mouse_drag_scrolls_and_stretches_the_example_list(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let (section, cx) = cx.add_window_view(|window, cx| ScrollBounceSection::view(window, cx));
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
    // ScrollBounce runs on the real monotonic clock, not the test executor's
    // virtual timer. A delayed frame may settle fully; either is valid.
    std::thread::sleep(Duration::from_millis(30));
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let returning = handle.viewport_bounds().origin.y;
    assert!(returning >= bounds.origin.y && returning < stretched);
}
