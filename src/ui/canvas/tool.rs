use gpui_kit::*;

use super::geometry::CanvasTransform;
use super::scene::{CanvasShape, ShapeId, StrokeStyle};
use super::state::CanvasState;

#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CanvasPointerEvent {
    pub screen_position: Point<Pixels>,
    pub doc_position: Point<Pixels>,
    pub button: MouseButton,
    pub click_count: usize,
    pub modifiers: Modifiers,
}

#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct CanvasScrollEvent {
    pub screen_position: Point<Pixels>,
    pub doc_position: Point<Pixels>,
    pub delta: ScrollDelta,
    pub modifiers: Modifiers,
}

/// Router-computed snapshot handed to tools each event. `viewport` is viewport
/// space: origin (0,0), element size.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct ToolContext {
    pub viewport: Bounds<Pixels>,
    pub transform: CanvasTransform,
    pub hit: Option<ShapeId>,
}

/// Pluggable tool. `state` is the live canvas state: call the ordinary
/// mutators on it (`state.pan_by(d, cx)`, ...); re-entering the entity via
/// `cx.entity().update(cx, ..)` panics (`GPUI/app/entity_map.rs:160-165`).
pub trait CanvasTool: 'static {
    /// Stable identifier ("pan", "select", "brush"); shown by diagnostics
    /// only, never UI copy.
    fn id(&self) -> &'static str;
    fn cursor(&self) -> CursorStyle {
        CursorStyle::Arrow
    }
    fn pointer_down(
        &mut self,
        _: &CanvasPointerEvent,
        _: &ToolContext,
        _: &mut CanvasState,
        _: &mut Context<CanvasState>,
    ) {
    }
    fn pointer_move(
        &mut self,
        _: &CanvasPointerEvent,
        _: &ToolContext,
        _: &mut CanvasState,
        _: &mut Context<CanvasState>,
    ) {
    }
    fn pointer_up(
        &mut self,
        _: &CanvasPointerEvent,
        _: &ToolContext,
        _: &mut CanvasState,
        _: &mut Context<CanvasState>,
    ) {
    }
    /// Return true when consumed; false falls through to the engine default
    /// (wheel pans, ctrl/cmd+wheel zooms at the cursor).
    fn scroll(
        &mut self,
        _: &CanvasScrollEvent,
        _: &ToolContext,
        _: &mut CanvasState,
        _: &mut Context<CanvasState>,
    ) -> bool {
        false
    }
}

pub(crate) struct NoopTool;

impl CanvasTool for NoopTool {
    fn id(&self) -> &'static str {
        "inactive"
    }
}

pub struct PanTool {
    dragging: bool,
    last_screen: Option<Point<Pixels>>,
}

impl PanTool {
    pub fn new() -> Self {
        Self {
            dragging: false,
            last_screen: None,
        }
    }
}

impl Default for PanTool {
    fn default() -> Self {
        Self::new()
    }
}

impl CanvasTool for PanTool {
    fn id(&self) -> &'static str {
        "pan"
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
        if event.button == MouseButton::Left {
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
        if event.modifiers.control || event.modifiers.platform {
            return false;
        }
        let delta = match event.delta {
            ScrollDelta::Lines(delta) => point(px(-delta.x * 40.), px(-delta.y * 40.)),
            ScrollDelta::Pixels(delta) => point(-delta.x, -delta.y),
        };
        state.pan_by(delta, cx);
        true
    }
}

pub struct SelectTool {
    move_origin: Option<Point<Pixels>>,
    snapshot: Vec<(ShapeId, CanvasShape)>,
}

impl SelectTool {
    pub fn new() -> Self {
        Self {
            move_origin: None,
            snapshot: Vec::new(),
        }
    }
}

impl Default for SelectTool {
    fn default() -> Self {
        Self::new()
    }
}

impl CanvasTool for SelectTool {
    fn id(&self) -> &'static str {
        "select"
    }

    fn pointer_down(
        &mut self,
        event: &CanvasPointerEvent,
        tool_ctx: &ToolContext,
        state: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) {
        if event.button != MouseButton::Left {
            return;
        }
        if let Some(hit) = tool_ctx.hit {
            if !state.selected_shape_ids().contains(&hit) {
                state.select_shape(hit, event.modifiers.shift, cx);
            }
            self.snapshot = state
                .selected_shape_ids()
                .iter()
                .filter_map(|id| {
                    state
                        .document()
                        .shape(*id)
                        .map(|shape| (*id, shape.clone()))
                })
                .collect();
            self.move_origin = Some(event.doc_position);
        } else if !event.modifiers.shift {
            state.clear_selection(cx);
        }
    }

    fn pointer_move(
        &mut self,
        event: &CanvasPointerEvent,
        _: &ToolContext,
        state: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) {
        let Some(origin) = self.move_origin else {
            return;
        };
        let delta = event.doc_position - origin;
        let snapshot = &self.snapshot;
        state.edit_document(cx, |doc| {
            for (id, snapshot_shape) in snapshot {
                doc.update_shape(*id, |shape| {
                    *shape = snapshot_shape.clone();
                    shape.translate(delta);
                });
            }
        });
    }

    fn pointer_up(
        &mut self,
        _: &CanvasPointerEvent,
        _: &ToolContext,
        _: &mut CanvasState,
        _: &mut Context<CanvasState>,
    ) {
        self.move_origin = None;
        self.snapshot.clear();
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct BrushConfig {
    pub color: Hsla,
    pub width: Pixels,
}

impl Default for BrushConfig {
    fn default() -> Self {
        Self {
            color: black(),
            width: px(3.),
        }
    }
}

pub struct BrushTool {
    config: BrushConfig,
    points: Vec<Point<Pixels>>,
    stroking: bool,
}

impl BrushTool {
    pub fn new(config: BrushConfig) -> Self {
        Self {
            config,
            points: Vec::new(),
            stroking: false,
        }
    }

    fn stroke_style(&self) -> StrokeStyle {
        StrokeStyle {
            width: self.config.width,
            color: self.config.color,
            dash: None,
        }
    }
}

impl CanvasTool for BrushTool {
    fn id(&self) -> &'static str {
        "brush"
    }

    fn cursor(&self) -> CursorStyle {
        CursorStyle::Crosshair
    }

    fn pointer_down(
        &mut self,
        event: &CanvasPointerEvent,
        _: &ToolContext,
        state: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) {
        if event.button != MouseButton::Left {
            return;
        }
        self.points = vec![event.doc_position];
        self.stroking = true;
        let overlay = CanvasShape::path(self.points.clone(), self.stroke_style());
        state.set_tool_overlay(Some(overlay), cx);
    }

    fn pointer_move(
        &mut self,
        event: &CanvasPointerEvent,
        tool_ctx: &ToolContext,
        state: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) {
        if !self.stroking {
            return;
        }
        let Some(last) = self.points.last() else {
            return;
        };
        let last_screen = tool_ctx.transform.to_screen(*last);
        let next_screen = tool_ctx.transform.to_screen(event.doc_position);
        let distance = last_screen - next_screen;
        let distance_sq = distance.x.as_f32().powi(2) + distance.y.as_f32().powi(2);
        if distance_sq <= 1. {
            return;
        }
        self.points.push(event.doc_position);
        let overlay = CanvasShape::path(self.points.clone(), self.stroke_style());
        state.set_tool_overlay(Some(overlay), cx);
    }

    fn pointer_up(
        &mut self,
        _: &CanvasPointerEvent,
        _: &ToolContext,
        state: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) {
        if !self.stroking {
            return;
        }
        let layer = state
            .document()
            .top_layer()
            .unwrap_or_else(|| state.add_layer(cx));
        state.add_shape(
            layer,
            CanvasShape::path(self.points.clone(), self.stroke_style()),
            cx,
        );
        state.set_tool_overlay(None, cx);
        self.stroking = false;
        self.points.clear();
    }
}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum BuiltinTool {
    Pan,
    Select,
    Brush(BrushConfig),
}

impl BuiltinTool {
    pub(crate) fn into_boxed_tool(self) -> Box<dyn CanvasTool> {
        match self {
            Self::Pan => Box::new(PanTool::new()),
            Self::Select => Box::new(SelectTool::new()),
            Self::Brush(config) => Box::new(BrushTool::new(config)),
        }
    }
}

#[cfg(test)]
#[path = "tool.test.rs"]
mod tool_test;
