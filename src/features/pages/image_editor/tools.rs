use gpui_kit::*;

use super::document::{ImageEditorDocument, MIN_CROP_PX, capture_annotations};
use super::history::{AnnotationSnapshot, EditLabel};
use crate::ui::canvas::{
    CanvasPointerEvent, CanvasShape, CanvasState, CanvasTool, LayerId, ShapeId, ShapeStyle,
    StrokeStyle, ToolContext,
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

/// Screen-space distance gate: sub-pixel moves never change what a drag
/// renders, so skip them before allocating the next shape.
fn moved_beyond(tool_ctx: &ToolContext, from: Point<Pixels>, to: Point<Pixels>) -> bool {
    let distance = tool_ctx.transform.to_screen(from) - tool_ctx.transform.to_screen(to);
    let distance_sq = distance.x.as_f32().powi(2) + distance.y.as_f32().powi(2);
    distance_sq > 1.
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
        if !moved_beyond(tool_ctx, *last, event.doc_position) {
            return;
        }
        self.points.push(event.doc_position);
        let overlay = CanvasShape::path(self.points.clone(), self.stroke.clone());
        state.set_tool_overlay(Some(overlay), cx);
    }

    fn pointer_up(
        &mut self,
        event: &CanvasPointerEvent,
        _: &ToolContext,
        state: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) {
        if !is_left(event) || !self.stroking {
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

/// What one anchor-to-current drag commits: a rect/ellipse/arrow annotation
/// with history, or a crop rect that records nothing.
enum DragKind {
    Rect,
    Ellipse,
    Arrow,
    Crop,
}

pub(crate) struct DragTool {
    kind: DragKind,
    doc: Entity<ImageEditorDocument>,
    layer: LayerId,
    stroke: StrokeStyle,
    anchor: Option<Point<Pixels>>,
    current: Option<Point<Pixels>>,
    before: Option<AnnotationSnapshot>,
}

impl DragTool {
    pub(crate) fn rect(
        doc: Entity<ImageEditorDocument>,
        layer: LayerId,
        stroke: StrokeStyle,
    ) -> Self {
        Self::new(DragKind::Rect, doc, layer, stroke)
    }

    pub(crate) fn ellipse(
        doc: Entity<ImageEditorDocument>,
        layer: LayerId,
        stroke: StrokeStyle,
    ) -> Self {
        Self::new(DragKind::Ellipse, doc, layer, stroke)
    }

    pub(crate) fn arrow(
        doc: Entity<ImageEditorDocument>,
        layer: LayerId,
        stroke: StrokeStyle,
    ) -> Self {
        Self::new(DragKind::Arrow, doc, layer, stroke)
    }

    /// Crop drags store the rect only; they add no shapes and no history.
    pub(crate) fn crop(doc: Entity<ImageEditorDocument>, layer: LayerId, color: Hsla) -> Self {
        Self::new(
            DragKind::Crop,
            doc,
            layer,
            StrokeStyle {
                width: px(1.),
                color,
                dash: Some(vec![px(4.), px(4.)]),
            },
        )
    }

    fn new(
        kind: DragKind,
        doc: Entity<ImageEditorDocument>,
        layer: LayerId,
        stroke: StrokeStyle,
    ) -> Self {
        Self {
            kind,
            doc,
            layer,
            stroke,
            anchor: None,
            current: None,
            before: None,
        }
    }

    fn overlay(&self, anchor: Point<Pixels>, current: Point<Pixels>) -> CanvasShape {
        match self.kind {
            DragKind::Rect | DragKind::Crop => CanvasShape::rect(
                bounds_from_corners(anchor, current),
                ShapeStyle::stroked(self.stroke.clone()),
            ),
            DragKind::Ellipse => CanvasShape::ellipse(
                bounds_from_corners(anchor, current),
                ShapeStyle::stroked(self.stroke.clone()),
            ),
            DragKind::Arrow => CanvasShape::path(vec![anchor, current], self.stroke.clone()),
        }
    }

    /// Shapes a finished annotation drag commits; empty when it degenerated
    /// to a click.
    fn commit_shapes(&self, anchor: Point<Pixels>, current: Point<Pixels>) -> Vec<CanvasShape> {
        match self.kind {
            DragKind::Rect | DragKind::Ellipse => {
                let bounds = bounds_from_corners(anchor, current);
                if bounds.size.width > px(0.) && bounds.size.height > px(0.) {
                    vec![self.overlay(anchor, current)]
                } else {
                    Vec::new()
                }
            }
            DragKind::Arrow => {
                if anchor == current {
                    Vec::new()
                } else {
                    arrow_segments(anchor, current)
                        .map(|(a, b)| CanvasShape::path(vec![a, b], self.stroke.clone()))
                        .to_vec()
                }
            }
            DragKind::Crop => Vec::new(),
        }
    }
}

impl CanvasTool for DragTool {
    fn id(&self) -> &'static str {
        match self.kind {
            DragKind::Rect => "image-rect",
            DragKind::Ellipse => "image-ellipse",
            DragKind::Arrow => "image-arrow",
            DragKind::Crop => "image-crop",
        }
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
        if !matches!(self.kind, DragKind::Crop) {
            self.before = Some(capture_annotations(state, self.layer));
        }
    }

    fn pointer_move(
        &mut self,
        event: &CanvasPointerEvent,
        tool_ctx: &ToolContext,
        state: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) {
        let Some(anchor) = self.anchor else {
            return;
        };
        let last = self.current.unwrap_or(anchor);
        if !moved_beyond(tool_ctx, last, event.doc_position) {
            return;
        }
        self.current = Some(event.doc_position);
        state.set_tool_overlay(Some(self.overlay(anchor, event.doc_position)), cx);
    }

    fn pointer_up(
        &mut self,
        event: &CanvasPointerEvent,
        _: &ToolContext,
        state: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) {
        if !is_left(event) {
            return;
        }
        let Some(anchor) = self.anchor.take() else {
            return;
        };
        let current = self.current.take().unwrap_or(anchor);
        state.set_tool_overlay(None, cx);
        if matches!(self.kind, DragKind::Crop) {
            // A click or sub-minimum drag leaves any stored rect untouched.
            let rect = bounds_from_corners(anchor, current);
            if rect.size.width >= MIN_CROP_PX && rect.size.height >= MIN_CROP_PX {
                self.doc
                    .update(cx, |doc, cx| doc.set_crop_rect(Some(rect), cx));
            }
            return;
        }
        let before = self.before.take().unwrap_or_default();
        let shapes = self.commit_shapes(anchor, current);
        if shapes.is_empty() {
            return;
        }
        for shape in shapes {
            state.add_shape(self.layer, shape, cx);
        }
        commit_annotation(&self.doc, EditLabel::Stroke, before, state, self.layer, cx);
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
        let hits: Vec<ShapeId> = state
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
        event: &CanvasPointerEvent,
        _: &ToolContext,
        state: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) {
        if !is_left(event) {
            return;
        }
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

#[cfg(test)]
#[path = "tools.test.rs"]
mod tools_test;
