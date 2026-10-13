use gpui_kit::{Context, CursorStyle, Pixels, Point, ScrollDelta, point, px};

use crate::ui::canvas::{
    CanvasPointerEvent, CanvasScrollEvent, CanvasState, CanvasTool, ToolContext,
};

pub(crate) const PAGE_MIN_ZOOM: f32 = 0.1;
pub(crate) const PAGE_MAX_ZOOM: f32 = 8.0;
pub(crate) const LABEL_LOD_ZOOM: f32 = 0.5;
pub(crate) const LINE_PX: f32 = 40.;

pub(crate) fn clamp_zoom(z: f32) -> f32 {
    z.clamp(PAGE_MIN_ZOOM, PAGE_MAX_ZOOM)
}

pub(crate) fn page_factor(zoom: f32, factor: f32) -> f32 {
    let desired = clamp_zoom(zoom * factor);
    if (desired - zoom).abs() <= f32::EPSILON {
        1.0
    } else {
        desired / zoom
    }
}

pub(crate) fn lines(delta: &ScrollDelta) -> Point<f32> {
    match delta {
        ScrollDelta::Lines(p) => *p,
        ScrollDelta::Pixels(p) => point(p.x.as_f32() / LINE_PX, p.y.as_f32() / LINE_PX),
    }
}

pub(crate) struct ExploreTool {
    dragging: bool,
    last_screen: Option<Point<Pixels>>,
}

impl ExploreTool {
    pub(crate) fn new() -> Self {
        Self {
            dragging: false,
            last_screen: None,
        }
    }
}

impl Default for ExploreTool {
    fn default() -> Self {
        Self::new()
    }
}

impl CanvasTool for ExploreTool {
    fn id(&self) -> &'static str {
        "explore"
    }

    fn cursor(&self) -> CursorStyle {
        CursorStyle::OpenHand
    }

    fn pointer_down(
        &mut self,
        event: &CanvasPointerEvent,
        _: &ToolContext,
        _: &mut CanvasState,
        _: &mut Context<CanvasState>,
    ) {
        if event.button == gpui_kit::MouseButton::Left {
            self.dragging = true;
            self.last_screen = Some(event.screen_position);
        }
    }

    fn pointer_move(
        &mut self,
        event: &CanvasPointerEvent,
        _: &ToolContext,
        state: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) {
        if !self.dragging {
            return;
        }
        if let Some(last) = self.last_screen {
            state.pan_by(event.screen_position - last, cx);
        }
        self.last_screen = Some(event.screen_position);
    }

    fn pointer_up(
        &mut self,
        _: &CanvasPointerEvent,
        _: &ToolContext,
        _: &mut CanvasState,
        _: &mut Context<CanvasState>,
    ) {
        self.dragging = false;
        self.last_screen = None;
    }

    fn scroll(
        &mut self,
        event: &CanvasScrollEvent,
        _: &ToolContext,
        state: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) -> bool {
        if event.modifiers.secondary() {
            let factor = page_factor(state.zoom(), (-lines(&event.delta).y * 0.1).exp());
            if factor != 1.0 {
                state.zoom_at(event.screen_position, factor, cx);
            }
        } else {
            let d = lines(&event.delta);
            state.pan_by(point(px(-d.x * LINE_PX), px(-d.y * LINE_PX)), cx);
        }
        true
    }
}
