use gpui_kit::prelude::*;
use gpui_kit::{
    Bounds, Entity, Modifiers, MouseButton, Subscription, TestAppContext, Window, div, point, px,
    size,
};

use crate::ui::canvas::{Canvas, CanvasPointerEvent, CanvasShape, CanvasState, ToolContext};

fn pointer(screen: (f32, f32), button: MouseButton) -> CanvasPointerEvent {
    let position = point(px(screen.0), px(screen.1));
    CanvasPointerEvent {
        screen_position: position,
        doc_position: position,
        button,
        click_count: 1,
        modifiers: Modifiers::none(),
    }
}

fn tool_ctx(canvas: &CanvasState) -> ToolContext {
    ToolContext {
        viewport: Bounds {
            origin: point(px(0.), px(0.)),
            size: size(px(480.), px(360.)),
        },
        transform: canvas.transform(),
        hit: None,
    }
}

struct ObsHost {
    canvas: Entity<CanvasState>,
    observed: usize,
    _subscriptions: Vec<Subscription>,
}

impl ObsHost {
    fn new(canvas: Entity<CanvasState>, cx: &mut Context<Self>) -> Self {
        let subscription = cx.observe(&canvas, |this, _, cx| {
            this.observed += 1;
            cx.notify();
        });
        Self {
            canvas,
            observed: 0,
            _subscriptions: vec![subscription],
        }
    }
}

impl Render for ObsHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(Canvas::new("obs-host-canvas", self.canvas.clone()))
    }
}

#[gpui_kit::test]
fn new_paint_canvas_mounts_a_brush_on_one_layer(cx: &mut TestAppContext) {
    let canvas = cx.update(super::new_paint_canvas);
    cx.update(|cx| {
        assert_eq!(canvas.read(cx).active_tool_id(), "brush");
        assert_eq!(canvas.read(cx).document().layers().len(), 1);
    });
}

#[gpui_kit::test]
fn brush_stroke_commits_exactly_one_path_shape(cx: &mut TestAppContext) {
    let canvas = cx.update(super::new_paint_canvas);
    cx.update(|cx| {
        let ctx = tool_ctx(canvas.read(cx));
        canvas.update(cx, |state, cx| {
            state.dispatch_tool_event(cx, |tool, state, cx| {
                tool.pointer_down(&pointer((10., 10.), MouseButton::Left), &ctx, state, cx)
            });
            state.dispatch_tool_event(cx, |tool, state, cx| {
                tool.pointer_move(&pointer((30., 12.), MouseButton::Left), &ctx, state, cx)
            });
            state.dispatch_tool_event(cx, |tool, state, cx| {
                tool.pointer_move(&pointer((50., 20.), MouseButton::Left), &ctx, state, cx)
            });
            state.dispatch_tool_event(cx, |tool, state, cx| {
                tool.pointer_up(&pointer((50., 20.), MouseButton::Left), &ctx, state, cx)
            });
        });
        let doc = canvas.read(cx).document();
        assert_eq!(doc.shapes().count(), 1);
        assert!(matches!(
            doc.shapes().next().unwrap().2,
            CanvasShape::Path { .. }
        ));
    });
}

#[gpui_kit::test]
async fn one_entity_update_notifies_hosts_in_both_windows(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let canvas = cx.update(super::new_paint_canvas);
    let host_one = cx.open_window(size(px(300.), px(200.)), {
        let canvas = canvas.clone();
        |_, cx| ObsHost::new(canvas, cx)
    });
    let host_two = cx.open_window(size(px(300.), px(200.)), {
        let canvas = canvas.clone();
        |_, cx| ObsHost::new(canvas, cx)
    });
    cx.update(|cx| canvas.update(cx, |state, cx| state.pan_by(point(px(5.), px(0.)), cx)));
    let observed_one = cx.update(|cx| host_one.read(cx).unwrap().observed);
    let observed_two = cx.update(|cx| host_two.read(cx).unwrap().observed);
    assert_eq!(observed_one, 1);
    assert_eq!(observed_two, 1);
    assert_eq!(cx.read(|cx| canvas.read(cx).pan()), point(px(5.), px(0.)));
}
