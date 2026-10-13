use gpui_kit::{
    App, Bounds, Entity, IntoElement, KeyDownEvent, KeyUpEvent, MouseDownEvent, PinchEvent, Pixels,
    Point, Styled, Window, canvas,
};

use crate::ui::canvas::CanvasState;

use super::InfiniteCanvasPage;
use super::explore_tool::page_factor;

pub(crate) fn local_viewport(viewport: Bounds<Pixels>) -> Bounds<Pixels> {
    Bounds {
        origin: Point::default(),
        size: viewport.size,
    }
}

pub(crate) fn interaction_probe(
    page: Entity<InfiniteCanvasPage>,
    state: Entity<CanvasState>,
) -> impl IntoElement {
    canvas(
        move |_, _, _| (),
        move |_, (), window: &mut Window, _cx: &mut App| {
            {
                let page = page.clone();
                window.on_key_event::<KeyDownEvent>(move |event, phase, window, cx| {
                    if !phase.bubble() {
                        return;
                    }
                    let in_scope = page.read(cx).focus_handle.contains_focused(window, cx)
                        || window.focused(cx).is_none();
                    if in_scope && event.keystroke.key == "space" && !event.is_held {
                        page.update(cx, |page, cx| page.set_space_pan(true, cx));
                    }
                });
            }
            {
                let page = page.clone();
                window.on_key_event::<KeyUpEvent>(move |event, _, _, cx| {
                    if event.keystroke.key == "space" {
                        page.update(cx, |page, cx| page.set_space_pan(false, cx));
                    }
                });
            }
            {
                let page = page.clone();
                let state = state.clone();
                window.on_mouse_event::<PinchEvent>(move |event, _, _, cx| {
                    let viewport = page.read(cx).viewport;
                    if !viewport.contains(&event.position) {
                        return;
                    }
                    let anchor = event.position - viewport.origin;
                    let factor = page_factor(state.read(cx).zoom(), 1.0 + event.delta);
                    if factor != 1.0 {
                        state.update(cx, |state, cx| state.zoom_at(anchor, factor, cx));
                    }
                });
            }
            {
                let page = page.clone();
                window.on_mouse_event::<MouseDownEvent>(move |event, _, window, cx| {
                    let (viewport, minimap_bounds) = {
                        let page = page.read(cx);
                        (page.viewport, page.minimap_bounds)
                    };
                    if !viewport.contains(&event.position)
                        || minimap_bounds.contains(&event.position)
                    {
                        return;
                    }
                    let handle = page.read(cx).focus_handle.clone();
                    handle.focus(window, cx);
                });
            }
        },
    )
    .absolute()
    .top_0()
    .left_0()
    .size_full()
}
