use gpui_kit::*;

use super::document::{ImageEditorDocument, capture_annotations};
use super::history::{AnnotationSnapshot, EditLabel};
use crate::ui::canvas::{
    CanvasPointerEvent, CanvasShape, CanvasState, CanvasTool, LayerId, ShapeStyle, StrokeStyle,
    ToolContext,
};

pub(crate) fn bounds_from_corners(a: Point<Pixels>, b: Point<Pixels>) -> Bounds<Pixels> {
    Bounds {
        origin: point(a.x.min(b.x), a.y.min(b.y)),
        size: size((a.x - b.x).abs(), (a.y - b.y).abs()),
    }
}

pub(crate) fn arrow_segments(
    from: Point<Pixels>,
    to: Point<Pixels>,
) -> [(Point<Pixels>, Point<Pixels>); 3] {
    let (fx, fy) = (from.x.as_f32(), from.y.as_f32());
    let (tx, ty) = (to.x.as_f32(), to.y.as_f32());
    let (dx, dy) = (tx - fx, ty - fy);
    let len = (dx * dx + dy * dy).sqrt();
    let (ux, uy) = if len > 0. {
        (dx / len, dy / len)
    } else {
        (1., 0.)
    };
    let head_len = (len * 0.2).clamp(8., 32.);
    let (vx, vy) = (-ux * head_len, -uy * head_len);
    let angle = 25_f32.to_radians();
    let head_tip = |sign: f32| {
        let (c, s) = (angle.cos(), angle.sin());
        point(
            px(tx + vx * c - vy * s * sign),
            px(ty + vx * s * sign + vy * c),
        )
    };
    [(from, to), (to, head_tip(1.)), (to, head_tip(-1.))]
}

fn is_left(event: &CanvasPointerEvent) -> bool {
    event.button == MouseButton::Left
}

fn commit_annotation(
    doc: &Entity<ImageEditorDocument>,
    label: EditLabel,
    before: AnnotationSnapshot,
    state: &mut CanvasState,
    layer: LayerId,
    cx: &mut Context<CanvasState>,
) {
    let after = capture_annotations(state, layer);
    doc.update(cx, |doc, cx| doc.record_annotate(label, before, after, cx));
}

pub(crate) struct BrushTool {
    doc: Entity<ImageEditorDocument>,
    layer: LayerId,
    stroke: StrokeStyle,
    points: Vec<Point<Pixels>>,
    stroking: bool,
    before: Option<AnnotationSnapshot>,
}

impl BrushTool {
    pub(crate) fn new(
        doc: Entity<ImageEditorDocument>,
        layer: LayerId,
        stroke: StrokeStyle,
    ) -> Self {
        Self {
            doc,
            layer,
            stroke,
            points: Vec::new(),
            stroking: false,
            before: None,
        }
    }
}

impl CanvasTool for BrushTool {
    fn id(&self) -> &'static str {
        "image-brush"
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
        if !is_left(event) {
            return;
        }
        self.points = vec![event.doc_position];
        self.stroking = true;
        self.before = Some(capture_annotations(state, self.layer));
        let overlay = CanvasShape::path(self.points.clone(), self.stroke.clone());
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
        let overlay = CanvasShape::path(self.points.clone(), self.stroke.clone());
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
        let before = self.before.take().unwrap_or_default();
        let points = std::mem::take(&mut self.points);
        state.add_shape(
            self.layer,
            CanvasShape::path(points, self.stroke.clone()),
            cx,
        );
        state.set_tool_overlay(None, cx);
        self.stroking = false;
        commit_annotation(&self.doc, EditLabel::Stroke, before, state, self.layer, cx);
    }
}

pub(crate) struct RectTool {
    doc: Entity<ImageEditorDocument>,
    layer: LayerId,
    stroke: StrokeStyle,
    anchor: Option<Point<Pixels>>,
    current: Option<Point<Pixels>>,
    before: Option<AnnotationSnapshot>,
}

impl RectTool {
    pub(crate) fn new(
        doc: Entity<ImageEditorDocument>,
        layer: LayerId,
        stroke: StrokeStyle,
    ) -> Self {
        Self {
            doc,
            layer,
            stroke,
            anchor: None,
            current: None,
            before: None,
        }
    }
}

impl CanvasTool for RectTool {
    fn id(&self) -> &'static str {
        "image-rect"
    }

    fn cursor(&self) -> CursorStyle {
        CursorStyle::Crosshair
    }

    fn pointer_down(
        &mut self,
        event: &CanvasPointerEvent,
        _: &ToolContext,
        state: &mut CanvasState,
        _: &mut Context<CanvasState>,
    ) {
        if !is_left(event) {
            return;
        }
        self.anchor = Some(event.doc_position);
        self.current = None;
        self.before = Some(capture_annotations(state, self.layer));
    }

    fn pointer_move(
        &mut self,
        event: &CanvasPointerEvent,
        _: &ToolContext,
        state: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) {
        let Some(anchor) = self.anchor else {
            return;
        };
        self.current = Some(event.doc_position);
        let overlay = CanvasShape::rect(
            bounds_from_corners(anchor, event.doc_position),
            ShapeStyle::stroked(self.stroke.clone()),
        );
        state.set_tool_overlay(Some(overlay), cx);
    }

    fn pointer_up(
        &mut self,
        _: &CanvasPointerEvent,
        _: &ToolContext,
        state: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) {
        let Some(anchor) = self.anchor.take() else {
            return;
        };
        let before = self.before.take().unwrap_or_default();
        let current = self.current.take().unwrap_or(anchor);
        state.set_tool_overlay(None, cx);
        let bounds = bounds_from_corners(anchor, current);
        if bounds.size.width > px(0.) && bounds.size.height > px(0.) {
            state.add_shape(
                self.layer,
                CanvasShape::rect(bounds, ShapeStyle::stroked(self.stroke.clone())),
                cx,
            );
            commit_annotation(&self.doc, EditLabel::Stroke, before, state, self.layer, cx);
        }
    }
}

pub(crate) struct EllipseTool {
    doc: Entity<ImageEditorDocument>,
    layer: LayerId,
    stroke: StrokeStyle,
    anchor: Option<Point<Pixels>>,
    current: Option<Point<Pixels>>,
    before: Option<AnnotationSnapshot>,
}

impl EllipseTool {
    pub(crate) fn new(
        doc: Entity<ImageEditorDocument>,
        layer: LayerId,
        stroke: StrokeStyle,
    ) -> Self {
        Self {
            doc,
            layer,
            stroke,
            anchor: None,
            current: None,
            before: None,
        }
    }
}

impl CanvasTool for EllipseTool {
    fn id(&self) -> &'static str {
        "image-ellipse"
    }

    fn cursor(&self) -> CursorStyle {
        CursorStyle::Crosshair
    }

    fn pointer_down(
        &mut self,
        event: &CanvasPointerEvent,
        _: &ToolContext,
        state: &mut CanvasState,
        _: &mut Context<CanvasState>,
    ) {
        if !is_left(event) {
            return;
        }
        self.anchor = Some(event.doc_position);
        self.current = None;
        self.before = Some(capture_annotations(state, self.layer));
    }

    fn pointer_move(
        &mut self,
        event: &CanvasPointerEvent,
        _: &ToolContext,
        state: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) {
        let Some(anchor) = self.anchor else {
            return;
        };
        self.current = Some(event.doc_position);
        let overlay = CanvasShape::ellipse(
            bounds_from_corners(anchor, event.doc_position),
            ShapeStyle::stroked(self.stroke.clone()),
        );
        state.set_tool_overlay(Some(overlay), cx);
    }

    fn pointer_up(
        &mut self,
        _: &CanvasPointerEvent,
        _: &ToolContext,
        state: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) {
        let Some(anchor) = self.anchor.take() else {
            return;
        };
        let before = self.before.take().unwrap_or_default();
        let current = self.current.take().unwrap_or(anchor);
        state.set_tool_overlay(None, cx);
        let bounds = bounds_from_corners(anchor, current);
        if bounds.size.width > px(0.) && bounds.size.height > px(0.) {
            state.add_shape(
                self.layer,
                CanvasShape::ellipse(bounds, ShapeStyle::stroked(self.stroke.clone())),
                cx,
            );
            commit_annotation(&self.doc, EditLabel::Stroke, before, state, self.layer, cx);
        }
    }
}

pub(crate) struct ArrowTool {
    doc: Entity<ImageEditorDocument>,
    layer: LayerId,
    stroke: StrokeStyle,
    anchor: Option<Point<Pixels>>,
    current: Option<Point<Pixels>>,
    before: Option<AnnotationSnapshot>,
}

impl ArrowTool {
    pub(crate) fn new(
        doc: Entity<ImageEditorDocument>,
        layer: LayerId,
        stroke: StrokeStyle,
    ) -> Self {
        Self {
            doc,
            layer,
            stroke,
            anchor: None,
            current: None,
            before: None,
        }
    }
}

impl CanvasTool for ArrowTool {
    fn id(&self) -> &'static str {
        "image-arrow"
    }

    fn cursor(&self) -> CursorStyle {
        CursorStyle::Crosshair
    }

    fn pointer_down(
        &mut self,
        event: &CanvasPointerEvent,
        _: &ToolContext,
        state: &mut CanvasState,
        _: &mut Context<CanvasState>,
    ) {
        if !is_left(event) {
            return;
        }
        self.anchor = Some(event.doc_position);
        self.current = None;
        self.before = Some(capture_annotations(state, self.layer));
    }

    fn pointer_move(
        &mut self,
        event: &CanvasPointerEvent,
        _: &ToolContext,
        state: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) {
        let Some(anchor) = self.anchor else {
            return;
        };
        self.current = Some(event.doc_position);
        let overlay = CanvasShape::path(vec![anchor, event.doc_position], self.stroke.clone());
        state.set_tool_overlay(Some(overlay), cx);
    }

    fn pointer_up(
        &mut self,
        _: &CanvasPointerEvent,
        _: &ToolContext,
        state: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) {
        let Some(anchor) = self.anchor.take() else {
            return;
        };
        let before = self.before.take().unwrap_or_default();
        let current = self.current.take().unwrap_or(anchor);
        state.set_tool_overlay(None, cx);
        if anchor != current {
            for (a, b) in arrow_segments(anchor, current) {
                state.add_shape(
                    self.layer,
                    CanvasShape::path(vec![a, b], self.stroke.clone()),
                    cx,
                );
            }
            commit_annotation(&self.doc, EditLabel::Stroke, before, state, self.layer, cx);
        }
    }
}

pub(crate) struct EraserTool {
    doc: Entity<ImageEditorDocument>,
    layer: LayerId,
    radius: f32,
    before: Option<AnnotationSnapshot>,
    removed: bool,
}

impl EraserTool {
    pub(crate) fn new(doc: Entity<ImageEditorDocument>, layer: LayerId, radius: f32) -> Self {
        Self {
            doc,
            layer,
            radius,
            before: None,
            removed: false,
        }
    }

    fn erase(
        &mut self,
        doc_position: Point<Pixels>,
        state: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) {
        let tolerance = px(self.radius);
        let hits: Vec<crate::ui::canvas::ShapeId> = state
            .document()
            .layer(self.layer)
            .map(|layer| {
                layer
                    .shapes()
                    .iter()
                    .filter(|(_, shape)| shape.hit_test(doc_position, tolerance))
                    .map(|(id, _)| *id)
                    .collect()
            })
            .unwrap_or_default();
        for id in hits {
            if state.remove_shape(id, cx) {
                self.removed = true;
            }
        }
    }
}

impl CanvasTool for EraserTool {
    fn id(&self) -> &'static str {
        "image-eraser"
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
        if !is_left(event) {
            return;
        }
        self.before = Some(capture_annotations(state, self.layer));
        self.removed = false;
        self.erase(event.doc_position, state, cx);
    }

    fn pointer_move(
        &mut self,
        event: &CanvasPointerEvent,
        _: &ToolContext,
        state: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) {
        if self.before.is_none() {
            return;
        }
        self.erase(event.doc_position, state, cx);
    }

    fn pointer_up(
        &mut self,
        _: &CanvasPointerEvent,
        _: &ToolContext,
        state: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) {
        let Some(before) = self.before.take() else {
            return;
        };
        let removed = self.removed;
        self.removed = false;
        if removed {
            commit_annotation(&self.doc, EditLabel::Erase, before, state, self.layer, cx);
        }
    }
}

pub(crate) struct CropTool {
    doc: Entity<ImageEditorDocument>,
    anchor: Option<Point<Pixels>>,
    current: Option<Point<Pixels>>,
    color: Hsla,
}

impl CropTool {
    pub(crate) fn new(doc: Entity<ImageEditorDocument>, color: Hsla) -> Self {
        Self {
            doc,
            anchor: None,
            current: None,
            color,
        }
    }

    fn crop_style(&self) -> StrokeStyle {
        StrokeStyle {
            width: px(1.),
            color: self.color,
            dash: Some(vec![px(4.), px(4.)]),
        }
    }
}

impl CanvasTool for CropTool {
    fn id(&self) -> &'static str {
        "image-crop"
    }

    fn cursor(&self) -> CursorStyle {
        CursorStyle::Crosshair
    }

    fn pointer_down(
        &mut self,
        event: &CanvasPointerEvent,
        _: &ToolContext,
        _: &mut CanvasState,
        _: &mut Context<CanvasState>,
    ) {
        if !is_left(event) {
            return;
        }
        self.anchor = Some(event.doc_position);
        self.current = None;
    }

    fn pointer_move(
        &mut self,
        event: &CanvasPointerEvent,
        _: &ToolContext,
        state: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) {
        let Some(anchor) = self.anchor else {
            return;
        };
        self.current = Some(event.doc_position);
        let overlay = CanvasShape::rect(
            bounds_from_corners(anchor, event.doc_position),
            ShapeStyle::stroked(self.crop_style()),
        );
        state.set_tool_overlay(Some(overlay), cx);
    }

    fn pointer_up(
        &mut self,
        _: &CanvasPointerEvent,
        _: &ToolContext,
        state: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) {
        let Some(anchor) = self.anchor.take() else {
            return;
        };
        let rect = bounds_from_corners(anchor, self.current.take().unwrap_or(anchor));
        state.set_tool_overlay(None, cx);
        self.doc
            .update(cx, |doc, cx| doc.set_crop_rect(Some(rect), cx));
    }
}

#[cfg(test)]
#[path = "tools.test.rs"]
mod tools_test;
