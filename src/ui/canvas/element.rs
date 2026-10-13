use gpui_kit::*;

use super::cache::CanvasCaches;
use super::paint::{paint_grid, paint_selection, paint_shape};
use super::state::{CanvasState, HIT_TOLERANCE};
use super::tool::{CanvasPointerEvent, CanvasScrollEvent, ToolContext};

/// Mounts the canvas. `id` keys the element state (path caches) and must be
/// unique among siblings. Fills its parent. The mounting view must
/// `cx.observe(&state, |_, _, cx| cx.notify())` so mutations repaint.
pub struct Canvas {
    id: ElementId,
    state: Entity<CanvasState>,
}

impl Canvas {
    pub fn new(id: impl Into<ElementId>, state: Entity<CanvasState>) -> Self {
        Self {
            id: id.into(),
            state,
        }
    }
}

impl IntoElement for Canvas {
    type Element = CanvasElement;

    fn into_element(self) -> Self::Element {
        CanvasElement {
            id: self.id,
            state: self.state,
        }
    }
}

pub struct CanvasElement {
    id: ElementId,
    state: Entity<CanvasState>,
}

impl IntoElement for CanvasElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

fn pointer_event(
    position: Point<Pixels>,
    button: MouseButton,
    click_count: usize,
    modifiers: Modifiers,
    doc_position: Point<Pixels>,
) -> CanvasPointerEvent {
    CanvasPointerEvent {
        screen_position: position,
        doc_position,
        button,
        click_count,
        modifiers,
    }
}

fn tool_ctx(bounds: Bounds<Pixels>, s: &CanvasState, doc: Point<Pixels>) -> ToolContext {
    ToolContext {
        viewport: Bounds {
            origin: Point::default(),
            size: bounds.size,
        },
        transform: s.transform(),
        hit: s.hit_test(doc, px(HIT_TOLERANCE.as_f32() / s.zoom())),
    }
}

impl Element for CanvasElement {
    type RequestLayoutState = ();
    type PrepaintState = Hitbox;

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let style = Style {
            size: Size::full(),
            ..Default::default()
        };
        (window.request_layout(style, None, cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        _: &mut App,
    ) -> Self::PrepaintState {
        window.insert_hitbox(bounds, HitboxBehavior::BlockMouse)
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let state = self.state.clone();
        let hitbox = prepaint.clone();
        let id = self.id.clone();

        state.update(cx, |s, cx| {
            window.with_content_mask(Some(ContentMask { bounds }), |window| {
                paint_grid(bounds, s.grid(), s.transform(), window, cx);
                let caches = CanvasCaches::for_paint((id.clone(), "caches"), window, cx);
                caches.update(cx, |caches, cx| {
                    let mut shape_ix = 0usize;
                    for layer in s.document().layers() {
                        if !layer.is_visible() {
                            continue;
                        }
                        for (_, shape) in layer.shapes() {
                            paint_shape(
                                shape,
                                s.transform(),
                                bounds,
                                caches,
                                2 * shape_ix,
                                2 * shape_ix + 1,
                                window,
                                cx,
                            );
                            shape_ix += 1;
                        }
                    }
                    if let Some(overlay) = s.tool_overlay() {
                        paint_shape(
                            overlay,
                            s.transform(),
                            bounds,
                            caches,
                            2 * shape_ix,
                            2 * shape_ix + 1,
                            window,
                            cx,
                        );
                    }
                    paint_selection(
                        s.document(),
                        s.selected_shape_ids(),
                        s.transform(),
                        bounds,
                        window,
                        cx,
                    );
                });
            });
        });

        window.set_cursor_style(state.read(cx).active_tool_cursor(), &hitbox);

        window.on_mouse_event({
            let hitbox = hitbox.clone();
            let state = state.clone();
            move |e: &MouseDownEvent, phase, window, cx| {
                if !phase.bubble() || !hitbox.is_hovered(window) {
                    return;
                }
                let position = e.position - bounds.origin;
                cx.stop_propagation();
                if e.button == MouseButton::Middle {
                    state.update(cx, |s, _| s.middle_pan = Some(position));
                } else {
                    state.update(cx, |s, cx| {
                        let doc = s.transform().to_doc(position);
                        let event =
                            pointer_event(position, e.button, e.click_count, e.modifiers, doc);
                        let ctx = tool_ctx(bounds, s, doc);
                        s.pointer_active = true;
                        s.dispatch_tool_event(cx, |tool, s, cx| {
                            tool.pointer_down(&event, &ctx, s, cx)
                        });
                    });
                }
            }
        });

        window.on_mouse_event({
            let hitbox = hitbox.clone();
            let state = state.clone();
            move |e: &MouseMoveEvent, phase, window, cx| {
                if !phase.bubble() {
                    return;
                }
                let position = e.position - bounds.origin;
                if state.read(cx).middle_pan.is_some() {
                    state.update(cx, |s, cx| {
                        if let Some(last) = s.middle_pan {
                            s.pan_by(position - last, cx);
                        }
                        s.middle_pan = Some(position);
                    });
                    cx.stop_propagation();
                    return;
                }
                if hitbox.is_hovered(window) || state.read(cx).is_pointer_active() {
                    state.update(cx, |s, cx| {
                        let doc = s.transform().to_doc(position);
                        let event = pointer_event(
                            position,
                            e.pressed_button.unwrap_or(MouseButton::Left),
                            0,
                            e.modifiers,
                            doc,
                        );
                        let ctx = tool_ctx(bounds, s, doc);
                        s.dispatch_tool_event(cx, |tool, s, cx| {
                            tool.pointer_move(&event, &ctx, s, cx)
                        });
                    });
                    cx.stop_propagation();
                }
            }
        });

        window.on_mouse_event({
            let state = state.clone();
            move |e: &MouseUpEvent, phase, _, cx| {
                if !phase.bubble() {
                    return;
                }
                let position = e.position - bounds.origin;
                if e.button == MouseButton::Middle {
                    let was_panning = state.read(cx).middle_pan.is_some();
                    state.update(cx, |s, _| s.middle_pan = None);
                    if was_panning {
                        cx.stop_propagation();
                    }
                    return;
                }
                if state.read(cx).is_pointer_active() {
                    state.update(cx, |s, cx| {
                        let doc = s.transform().to_doc(position);
                        let event =
                            pointer_event(position, e.button, e.click_count, e.modifiers, doc);
                        let ctx = tool_ctx(bounds, s, doc);
                        s.dispatch_tool_event(cx, |tool, s, cx| {
                            tool.pointer_up(&event, &ctx, s, cx)
                        });
                        s.pointer_active = false;
                    });
                    cx.stop_propagation();
                }
            }
        });

        window.on_mouse_event({
            let hitbox = hitbox.clone();
            let state = state.clone();
            move |e: &ScrollWheelEvent, phase, window, cx| {
                if !phase.bubble() || !hitbox.should_handle_scroll(window) {
                    return;
                }
                let position = e.position - bounds.origin;
                cx.stop_propagation();
                state.update(cx, |s, cx| {
                    let doc = s.transform().to_doc(position);
                    let event = CanvasScrollEvent {
                        screen_position: position,
                        doc_position: doc,
                        delta: e.delta,
                        modifiers: e.modifiers,
                    };
                    let ctx = tool_ctx(bounds, s, doc);
                    let consumed =
                        s.dispatch_tool_event(cx, |tool, s, cx| tool.scroll(&event, &ctx, s, cx));
                    if consumed {
                        return;
                    }
                    if e.modifiers.control || e.modifiers.platform {
                        let delta_lines = match e.delta {
                            ScrollDelta::Lines(delta) => delta.y,
                            ScrollDelta::Pixels(delta) => delta.y.as_f32() / 40.,
                        };
                        let factor = (-delta_lines * 0.1).exp();
                        s.zoom_at(position, factor, cx);
                    } else {
                        let delta = match e.delta {
                            ScrollDelta::Lines(delta) => {
                                point(px(-delta.x * 40.), px(-delta.y * 40.))
                            }
                            ScrollDelta::Pixels(delta) => point(-delta.x, -delta.y),
                        };
                        s.pan_by(delta, cx);
                    }
                });
            }
        });
    }
}

#[cfg(test)]
#[path = "element.test.rs"]
mod element_test;
