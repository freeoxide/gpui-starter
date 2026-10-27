use gpui_kit::prelude::*;
use gpui_kit::{
    Bounds, Entity, Modifiers, MouseButton, TestAppContext, VisualTestContext, Window, div, point,
    px, size,
};

use crate::ui::canvas::{CanvasPointerEvent, CanvasState, ToolContext};
use crate::ui::floating::{close_floating, floating_snapshot, init};

use super::hud::HUD_ID_PREFIX;
use super::paint::MINI_PAINT_ID;
use super::shader::SHADER_ID;

struct StubView;

impl Render for StubView {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

fn draw_stroke(canvas: &Entity<CanvasState>, cx: &mut VisualTestContext) {
    cx.update(|_, cx| {
        let ctx = ToolContext {
            viewport: Bounds {
                origin: point(px(0.), px(0.)),
                size: size(px(480.), px(360.)),
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
                tool.pointer_down(&pointer((10., 10.), MouseButton::Left), &ctx, state, cx)
            });
            state.dispatch_tool_event(cx, |tool, state, cx| {
                tool.pointer_move(&pointer((40., 14.), MouseButton::Left), &ctx, state, cx)
            });
            state.dispatch_tool_event(cx, |tool, state, cx| {
                tool.pointer_up(&pointer((40., 14.), MouseButton::Left), &ctx, state, cx)
            });
        });
    });
}

#[gpui_kit::test]
async fn register_pushes_the_floating_canvas_section(cx: &mut TestAppContext) {
    let handle = cx.open_window(size(px(400.), px(300.)), |_, _| StubView);
    let mut cx = VisualTestContext::from_window(handle.into(), cx);
    cx.update(|window, cx| {
        let mut sections = vec![];
        super::register(&mut sections, window, cx);
        assert_eq!(sections.len(), 1);
        assert_eq!(sections[0].id(), "floating-canvas");
    });
}

#[gpui_kit::test]
fn detach_respawn_close_lifecycle_keeps_the_document(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init);
    let window = cx.open_window(size(px(800.), px(600.)), |_, _| StubView);
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    let section = cx.update(super::FloatingCanvasSection::view);

    cx.update(|_, cx| section.update(cx, |this, cx| this.detach_paint(cx)));
    let mini_paint = cx.update(|_, cx| {
        floating_snapshot(cx)
            .into_iter()
            .filter(|entry| entry.id.0 == MINI_PAINT_ID)
            .collect::<Vec<_>>()
    });
    assert_eq!(mini_paint.len(), 1);

    cx.update(|_, cx| section.update(cx, |this, cx| this.detach_paint(cx)));
    let respawned = cx.update(|_, cx| {
        floating_snapshot(cx)
            .into_iter()
            .filter(|entry| entry.id.0 == MINI_PAINT_ID)
            .count()
    });
    assert_eq!(respawned, 1);

    let canvas = cx.update(|_, cx| section.read(cx).paint_canvas.clone());
    draw_stroke(&canvas, &mut cx);
    let float_handle = cx.update(|_, cx| {
        floating_snapshot(cx)
            .into_iter()
            .find(|entry| entry.id.0 == MINI_PAINT_ID)
            .map(|entry| entry.handle)
            .unwrap()
    });
    let first = cx.update(|_, cx| close_floating(&float_handle, cx));
    assert!(first);
    let second = cx.update(|_, cx| close_floating(&float_handle, cx));
    assert!(!second);
    cx.update(|_, cx| {
        assert_eq!(canvas.read(cx).document().shapes().count(), 1);
    });
}

#[gpui_kit::test]
fn spawn_hud_assigns_cascade_ids_and_advances_the_shared_model(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init);
    let window = cx.open_window(size(px(800.), px(600.)), |_, _| StubView);
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    let section = cx.update(super::FloatingCanvasSection::view);

    cx.update(|_, cx| section.update(cx, |this, cx| this.spawn_hud(cx)));
    cx.update(|_, cx| section.update(cx, |this, cx| this.spawn_hud(cx)));
    let ids = cx.update(|_, cx| {
        floating_snapshot(cx)
            .into_iter()
            .filter(|entry| entry.id.0.starts_with(HUD_ID_PREFIX))
            .map(|entry| entry.id.0.to_string())
            .collect::<Vec<_>>()
    });
    assert_eq!(ids, vec!["canvas-hud-0", "canvas-hud-1"]);

    cx.executor()
        .advance_clock(core::time::Duration::from_millis(250 * 3));
    cx.run_until_parked();
    let samples = cx.update(|_, cx| section.read(cx).metrics.read(cx).samples().len());
    assert_eq!(samples, 3);

    let hud_handle = cx.update(|_, cx| {
        floating_snapshot(cx)
            .into_iter()
            .find(|entry| entry.id.0 == "canvas-hud-0")
            .map(|entry| entry.handle)
    });
    let closed = cx.update(|_, cx| close_floating(&hud_handle.unwrap(), cx));
    assert!(closed);
    let remaining = cx.update(|_, cx| {
        floating_snapshot(cx)
            .into_iter()
            .filter(|entry| entry.id.0.starts_with(HUD_ID_PREFIX))
            .count()
    });
    assert_eq!(remaining, 1);
}

#[gpui_kit::test]
fn open_shader_is_idempotent_per_id(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init);
    let window = cx.open_window(size(px(800.), px(600.)), |_, _| StubView);
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    let section = cx.update(super::FloatingCanvasSection::view);

    cx.update(|_, cx| section.update(cx, |this, cx| this.open_shader(cx)));
    let shader_count = cx.update(|_, cx| {
        floating_snapshot(cx)
            .into_iter()
            .filter(|entry| entry.id.0 == SHADER_ID)
            .count()
    });
    assert_eq!(shader_count, 1);

    cx.update(|_, cx| section.update(cx, |this, cx| this.open_shader(cx)));
    let respawned = cx.update(|_, cx| {
        floating_snapshot(cx)
            .into_iter()
            .filter(|entry| entry.id.0 == SHADER_ID)
            .count()
    });
    assert_eq!(respawned, 1);
}

#[gpui_kit::test]
fn close_ours_closes_only_this_sections_floats(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init);
    let window = cx.open_window(size(px(800.), px(600.)), |_, _| StubView);
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    let section = cx.update(super::FloatingCanvasSection::view);

    cx.update(|_, cx| {
        section.update(cx, |this, cx| {
            this.detach_paint(cx);
            this.spawn_hud(cx);
            this.open_shader(cx);
        })
    });
    let closed = cx.update(|_, cx| section.update(cx, |this, cx| this.close_ours(cx)));
    assert_eq!(closed, 3);
    let ours = cx.update(|_, cx| {
        floating_snapshot(cx)
            .into_iter()
            .filter(|entry| entry.id.0.starts_with("canvas-"))
            .count()
    });
    assert_eq!(ours, 0);
    let idle = cx.update(|_, cx| section.update(cx, |this, cx| this.close_ours(cx)));
    assert_eq!(idle, 0);
}
