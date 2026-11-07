use gpui_kit::*;

use super::geometry::CanvasTransform;
use super::scene::{CanvasShape, ShapeId, ShapeStyle, StrokeStyle};
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

/// Per-event tool snapshot. `viewport` is element-local at origin (0,0); `hit` is
/// set on pointer_down only, sparing moves and scrolls the document hit-test.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub struct ToolContext {
    pub viewport: Bounds<Pixels>,
    pub transform: CanvasTransform,
    pub hit: Option<ShapeId>,
}

/// Pluggable tool, called inside the state's update: mutate `state` directly
/// (`state.pan_by(d, cx)`); re-entering via `cx.entity().update(cx, ..)` panics.
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
        event: &CanvasPointerEvent,
        _: &ToolContext,
        _: &mut CanvasState,
        _: &mut Context<CanvasState>,
    ) {
        if event.button != MouseButton::Left {
            return;
        }
        self.dragging = false;
        self.last_screen = None;
    }
}

pub struct SelectTool {
    last_doc_position: Option<Point<Pixels>>,
}

impl SelectTool {
    pub fn new() -> Self {
        Self {
            last_doc_position: None,
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
            self.last_doc_position = Some(event.doc_position);
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
        let Some(last) = self.last_doc_position else {
            return;
        };
        let delta = event.doc_position - last;
        self.last_doc_position = Some(event.doc_position);
        let selected = state.selected_shape_ids().to_vec();
        state.edit_document(cx, |doc| {
            for id in selected {
                doc.update_shape(id, |shape| shape.translate(delta));
            }
        });
    }

    fn pointer_up(
        &mut self,
        event: &CanvasPointerEvent,
        _: &ToolContext,
        _: &mut CanvasState,
        _: &mut Context<CanvasState>,
    ) {
        if event.button != MouseButton::Left {
            return;
        }
        self.last_doc_position = None;
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
        self.points.clear();
        self.points.push(event.doc_position);
        self.stroking = true;
        let overlay = CanvasShape::path(vec![event.doc_position], self.stroke_style());
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
        event: &CanvasPointerEvent,
        _: &ToolContext,
        state: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) {
        if !self.stroking || event.button != MouseButton::Left {
            return;
        }
        let layer = state
            .document()
            .top_layer()
            .unwrap_or_else(|| state.add_layer(cx));
        state.add_shape(
            layer,
            CanvasShape::path(std::mem::take(&mut self.points), self.stroke_style()),
            cx,
        );
        state.set_tool_overlay(None, cx);
        self.stroking = false;
    }
}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum BuiltinTool {
    Pan,
    Select,
    Brush(BrushConfig),
    Shape(ShapeConfig),
}

impl BuiltinTool {
    pub(crate) fn into_boxed_tool(self) -> Box<dyn CanvasTool> {
        match self {
            Self::Pan => Box::new(PanTool::new()),
            Self::Select => Box::new(SelectTool::new()),
            Self::Brush(config) => Box::new(BrushTool::new(config)),
            Self::Shape(config) => Box::new(ShapeTool::new(config)),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum ShapeKind {
    Rect,
    Ellipse,
    Triangle,
}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct ShapeConfig {
    pub kind: ShapeKind,
    pub style: ShapeStyle,
}

/// Drag-to-draw tool: anchors on pointer-down, previews through the tool
/// overlay, and commits the shape on pointer-up when the drag has extent.
pub struct ShapeTool {
    config: ShapeConfig,
    anchor: Option<Point<Pixels>>,
    dragging: bool,
}

impl ShapeTool {
    pub fn new(config: ShapeConfig) -> Self {
        Self {
            config,
            anchor: None,
            dragging: false,
        }
    }

    fn shape(&self, from: Point<Pixels>, to: Point<Pixels>) -> CanvasShape {
        let bounds = Bounds::from_corners(
            point(from.x.min(to.x), from.y.min(to.y)),
            point(from.x.max(to.x), from.y.max(to.y)),
        );
        match self.config.kind {
            ShapeKind::Rect => CanvasShape::rect(bounds, self.config.style.clone()),
            ShapeKind::Ellipse => CanvasShape::ellipse(bounds, self.config.style.clone()),
            ShapeKind::Triangle => CanvasShape::triangle(bounds, self.config.style.clone()),
        }
    }
}

impl CanvasTool for ShapeTool {
    fn id(&self) -> &'static str {
        "shape"
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
        self.anchor = Some(event.doc_position);
        self.dragging = true;
        state.set_tool_overlay(Some(self.shape(event.doc_position, event.doc_position)), cx);
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
        if let Some(anchor) = self.anchor {
            let overlay = self.shape(anchor, event.doc_position);
            state.set_tool_overlay(Some(overlay), cx);
        }
    }

    fn pointer_up(
        &mut self,
        event: &CanvasPointerEvent,
        _: &ToolContext,
        state: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) {
        if !self.dragging || event.button != MouseButton::Left {
            return;
        }
        self.dragging = false;
        if let Some(anchor) = self.anchor.take() {
            let shape = self.shape(anchor, event.doc_position);
            let bounds = shape.doc_bounds();
            if bounds.size.width > px(0.) && bounds.size.height > px(0.) {
                let layer = state
                    .document()
                    .top_layer()
                    .unwrap_or_else(|| state.add_layer(cx));
                state.add_shape(layer, shape, cx);
            }
        }
        state.set_tool_overlay(None, cx);
    }
}

#[cfg(test)]
#[path = "tool.test.rs"]
mod tool_test;
