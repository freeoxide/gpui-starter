use gpui_kit::{
    Bounds, Entity, Modifiers, MouseButton, TestAppContext, VisualTestContext, point, px, size,
};

use crate::ui::canvas::{CanvasPointerEvent, CanvasState, ToolContext};
use crate::ui::floating::{FloatAnchor, floating_snapshot, init};

use super::{SKETCH_ID, SketchPad, new_sketch_canvas, sketch_spec, spawn_sketch};

fn draw_stroke(canvas: &Entity<CanvasState>, cx: &mut VisualTestContext) {
    cx.update(|_, cx| {
        let ctx = ToolContext {
            viewport: Bounds {
                origin: point(px(0.), px(0.)),
                size: size(px(360.), px(260.)),
            },
            transform: canvas.read(cx).transform(),
            hit: None,
        };
        let pointer = |screen: (f32, f32), button: MouseButton| {
            let position = point(px(screen.0), px(screen.1));
            CanvasPointerEvent {
                screen_position: position,
                doc_position: position,
                button,
                click_count: 1,
                modifiers: Modifiers::none(),
            }
        };
        canvas.update(cx, |state, cx| {
            state.dispatch_tool_event(cx, |tool, state, cx| {
                tool.pointer_down(&pointer((12., 12.), MouseButton::Left), &ctx, state, cx)
            });
            state.dispatch_tool_event(cx, |tool, state, cx| {
                tool.pointer_move(&pointer((60., 30.), MouseButton::Left), &ctx, state, cx)
            });
            state.dispatch_tool_event(cx, |tool, state, cx| {
                tool.pointer_up(&pointer((60., 30.), MouseButton::Left), &ctx, state, cx)
            });
        });
    });
}

#[gpui_kit::test]
fn new_sketch_canvas_mounts_a_brush_on_one_hidden_grid_layer(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let canvas = cx.update(|cx| new_sketch_canvas(cx));
    cx.update(|cx| {
        assert_eq!(canvas.read(cx).active_tool_id(), "brush");
        assert_eq!(canvas.read(cx).document().layers().len(), 1);
        assert!(!canvas.read(cx).grid().visible);
    });
}

#[gpui_kit::test]
fn sketch_spec_is_chromeless_screen_anchored_and_resizable() {
    let spec = sketch_spec();
    assert!(spec.is_chromeless());
    assert!(matches!(spec.float_anchor(), FloatAnchor::Screen { .. }));
    assert!(spec.is_resizable());
    assert_eq!(spec.min_window_size(), Some(size(px(280.), px(200.))));
}

#[gpui_kit::test]
fn spawn_sketch_opens_an_idempotent_screen_float_without_a_main_window(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init);
    let _ = cx.update(|cx| spawn_sketch(cx));
    let _ = cx.update(|cx| spawn_sketch(cx));
    let count = cx.update(|cx| {
        floating_snapshot(cx)
            .iter()
            .filter(|entry| entry.id.0 == SKETCH_ID)
            .count()
    });
    assert_eq!(count, 1, "a second spawn must reuse the open sketch float");
}

#[gpui_kit::test]
async fn the_clear_button_empties_drawn_strokes(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let window = cx.open_window(size(px(360.), px(260.)), |_, cx| {
        let canvas = new_sketch_canvas(cx);
        SketchPad::new(canvas, cx)
    });
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let canvas = cx.read(|cx| window.read(cx).unwrap().canvas.clone());
    draw_stroke(&canvas, &mut cx);
    cx.update(|_, cx| {
        assert_eq!(canvas.read(cx).document().shapes().count(), 1);
    });

    let mut cleared = false;
    for y in (6..30usize).step_by(4) {
        for x in (300..350usize).step_by(8) {
            cx.simulate_click(point(px(x as f32), px(y as f32)), Modifiers::none());
            if cx.update(|_, cx| canvas.read(cx).document().shapes().count() == 0) {
                cleared = true;
                break;
            }
        }
        if cleared {
            break;
        }
    }
    assert!(cleared, "the Clear button must empty the document");
}
