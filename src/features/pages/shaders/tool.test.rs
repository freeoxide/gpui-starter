use gpui_kit::{
    AppContext as _, Bounds, Entity, Modifiers, MouseButton, TestAppContext, point, px, size,
};

use super::RippleTool;
use crate::features::pages::shaders::demos::{DemoId, demo};
use crate::features::pages::shaders::state::ShaderState;
use crate::ui::canvas::{
    CanvasPointerEvent, CanvasState, CanvasTool, CanvasTransform, ToolContext,
};

fn pointer_at(doc: (f32, f32)) -> CanvasPointerEvent {
    let doc_position = point(px(doc.0), px(doc.1));
    CanvasPointerEvent {
        screen_position: doc_position,
        doc_position,
        button: MouseButton::Left,
        click_count: 1,
        modifiers: Modifiers::default(),
    }
}

fn ctx() -> ToolContext {
    ToolContext {
        viewport: Bounds {
            origin: point(px(0.), px(0.)),
            size: size(px(512.), px(512.)),
        },
        transform: CanvasTransform::IDENTITY,
        hit: None,
    }
}

fn installed_pair(cx: &mut TestAppContext) -> (Entity<ShaderState>, Entity<CanvasState>) {
    let shader = cx.new(|cx| ShaderState::new(DemoId::Ripple, cx));
    let canvas = cx.new(|_| CanvasState::new());
    let tool = RippleTool::new(shader.downgrade(), 512.0, 512.0);
    cx.update(|cx| {
        canvas.update(cx, |s, cx| s.set_tool(Box::new(tool), cx));
    });
    (shader, canvas)
}

fn mouse_uv(cx: &mut TestAppContext, shader: &Entity<ShaderState>) -> (f32, f32) {
    cx.update(|cx| shader.read(cx).params_snapshot(demo(DemoId::Ripple)).mouse)
}

#[gpui_kit::test]
async fn pointer_move_maps_doc_position_to_uv(cx: &mut TestAppContext) {
    let (shader, canvas) = installed_pair(cx);
    let event = pointer_at((128., 384.));

    cx.update(|cx| {
        canvas.update(cx, |s, cx| {
            s.dispatch_tool_event(cx, |tool, s, cx| tool.pointer_move(&event, &ctx(), s, cx))
        })
    });

    assert_eq!(mouse_uv(cx, &shader), (0.25, 0.75));
}

#[gpui_kit::test]
async fn out_of_bounds_positions_clamp_to_the_unit_square(cx: &mut TestAppContext) {
    let (shader, canvas) = installed_pair(cx);

    for doc in [(-64., 300.), (700., 512.)] {
        let event = pointer_at(doc);
        cx.update(|cx| {
            canvas.update(cx, |s, cx| {
                s.dispatch_tool_event(cx, |tool, s, cx| tool.pointer_down(&event, &ctx(), s, cx))
            })
        });
    }

    assert_eq!(mouse_uv(cx, &shader), (1.0, 1.0));
}
