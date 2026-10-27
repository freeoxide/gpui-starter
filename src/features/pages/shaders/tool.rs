// Only the native install path constructs the tool; wasm compiles the
// module without it.
#![cfg_attr(target_family = "wasm", allow(dead_code))]

use gpui_kit::{Context, CursorStyle, WeakEntity};

use super::state::ShaderState;
use crate::ui::canvas::{CanvasPointerEvent, CanvasState, CanvasTool, ToolContext};

pub(in crate::features::pages::shaders) struct RippleTool {
    shader: WeakEntity<ShaderState>,
    dims: (f32, f32),
}

impl RippleTool {
    pub(in crate::features::pages::shaders) fn new(
        shader: WeakEntity<ShaderState>,
        width: f32,
        height: f32,
    ) -> Self {
        Self {
            shader,
            dims: (width, height),
        }
    }

    fn forward(&self, event: &CanvasPointerEvent, cx: &mut Context<CanvasState>) {
        let uv = (
            (event.doc_position.x.as_f32() / self.dims.0).clamp(0.0, 1.0),
            (event.doc_position.y.as_f32() / self.dims.1).clamp(0.0, 1.0),
        );
        let _ = self.shader.update(cx, |s, cx| s.set_mouse(uv, cx));
    }
}

impl CanvasTool for RippleTool {
    fn id(&self) -> &'static str {
        "shader-ripple"
    }

    fn cursor(&self) -> CursorStyle {
        CursorStyle::Crosshair
    }

    fn pointer_down(
        &mut self,
        event: &CanvasPointerEvent,
        _: &ToolContext,
        _: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) {
        self.forward(event, cx);
    }

    fn pointer_move(
        &mut self,
        event: &CanvasPointerEvent,
        _: &ToolContext,
        _: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) {
        self.forward(event, cx);
    }
}

#[cfg(test)]
#[path = "tool.test.rs"]
mod tool_test;
