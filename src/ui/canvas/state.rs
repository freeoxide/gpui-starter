use gpui_kit::*;

use super::geometry::CanvasTransform;
use super::scene::{CanvasDocument, CanvasShape, LayerId, ShapeId};
use super::tool::{BuiltinTool, CanvasTool, NoopTool, PanTool};

pub const MIN_ZOOM: f32 = 0.05;
pub const MAX_ZOOM: f32 = 32.0;
/// Minimum hit-test tolerance in screen pixels; doc-space tolerance = this / zoom.
pub const HIT_TOLERANCE: Pixels = px(4.);

#[derive(Clone, Copy, Debug, PartialEq)]
#[non_exhaustive]
pub struct CanvasGrid {
    pub visible: bool,
    pub spacing: Pixels,
}

impl Default for CanvasGrid {
    fn default() -> Self {
        Self {
            visible: true,
            spacing: px(16.),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum CanvasEvent {
    DocumentChanged,
    SelectionChanged,
    ViewTransformed,
    ToolChanged,
}

/// Retained canvas state. Mount with `cx.new(|_| CanvasState::new())` and
/// render with `Canvas::new(id, entity)`; mutations repaint only when the
/// mounting view observes (`Canvas::new` documents the exact wiring).
pub struct CanvasState {
    document: CanvasDocument,
    transform: CanvasTransform,
    grid: CanvasGrid,
    selection: Vec<ShapeId>,
    doc_revision: u64,
    tool: Box<dyn CanvasTool>,
    overlay: Option<CanvasShape>,
    pub(crate) middle_pan: Option<Point<Pixels>>,
    pub(crate) pointer_active: bool,
}

impl EventEmitter<CanvasEvent> for CanvasState {}

impl Default for CanvasState {
    fn default() -> Self {
        Self::new()
    }
}

impl CanvasState {
    pub fn new() -> Self {
        Self {
            document: CanvasDocument::new(),
            transform: CanvasTransform::IDENTITY,
            grid: CanvasGrid::default(),
            selection: Vec::new(),
            doc_revision: 0,
            tool: Box::new(PanTool::new()),
            overlay: None,
            middle_pan: None,
            pointer_active: false,
        }
    }

    pub(crate) fn dispatch_tool_event<R>(
        &mut self,
        cx: &mut Context<Self>,
        f: impl FnOnce(&mut dyn CanvasTool, &mut CanvasState, &mut Context<CanvasState>) -> R,
    ) -> R {
        let mut tool = std::mem::replace(&mut self.tool, Box::new(NoopTool));
        let result = f(tool.as_mut(), self, cx);
        if self.tool.id() == "inactive" {
            self.tool = tool;
        }
        result
    }

    pub fn pan(&self) -> Point<Pixels> {
        self.transform.pan
    }

    pub fn zoom(&self) -> f32 {
        self.transform.zoom
    }

    pub fn transform(&self) -> CanvasTransform {
        self.transform
    }

    pub fn grid(&self) -> CanvasGrid {
        self.grid
    }

    pub fn set_grid(&mut self, grid: CanvasGrid, cx: &mut Context<Self>) {
        self.grid = grid;
        cx.notify();
    }

    pub fn document(&self) -> &CanvasDocument {
        &self.document
    }

    pub fn document_revision(&self) -> u64 {
        self.doc_revision
    }

    pub fn edit_document<R>(
        &mut self,
        cx: &mut Context<Self>,
        edit: impl FnOnce(&mut CanvasDocument) -> R,
    ) -> R {
        let result = edit(&mut self.document);
        self.doc_revision += 1;
        cx.emit(CanvasEvent::DocumentChanged);
        cx.notify();
        result
    }

    pub fn add_layer(&mut self, cx: &mut Context<Self>) -> LayerId {
        self.edit_document(cx, |doc| doc.add_layer())
    }

    pub fn add_shape(
        &mut self,
        layer: LayerId,
        shape: CanvasShape,
        cx: &mut Context<Self>,
    ) -> Option<ShapeId> {
        self.edit_document(cx, |doc| doc.add_shape(layer, shape))
    }

    pub fn remove_shape(&mut self, id: ShapeId, cx: &mut Context<Self>) -> bool {
        self.edit_document(cx, |doc| doc.remove_shape(id)).is_some()
    }

    pub fn set_layer_visible(&mut self, layer: LayerId, visible: bool, cx: &mut Context<Self>) {
        self.edit_document(cx, |doc| doc.set_layer_visible(layer, visible));
    }

    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.edit_document(cx, |doc| doc.clear());
    }

    pub fn pan_by(&mut self, delta: Point<Pixels>, cx: &mut Context<Self>) {
        self.transform.pan += delta;
        cx.emit(CanvasEvent::ViewTransformed);
        cx.notify();
    }

    pub fn zoom_at(&mut self, anchor_screen: Point<Pixels>, factor: f32, cx: &mut Context<Self>) {
        let new_zoom = (self.transform.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
        let ratio = new_zoom / self.transform.zoom;
        self.transform.pan = anchor_screen - (anchor_screen - self.transform.pan) * ratio;
        self.transform.zoom = new_zoom;
        cx.emit(CanvasEvent::ViewTransformed);
        cx.notify();
    }

    pub fn set_zoom(&mut self, zoom: f32, cx: &mut Context<Self>) {
        self.transform.zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
        cx.emit(CanvasEvent::ViewTransformed);
        cx.notify();
    }

    /// Fits `doc_bounds` into `viewport` with a 0.9 margin; empty or
    /// zero-size bounds reset the view.
    pub fn zoom_to_fit(
        &mut self,
        doc_bounds: Bounds<Pixels>,
        viewport: Size<Pixels>,
        cx: &mut Context<Self>,
    ) {
        let (doc_w, doc_h) = (
            doc_bounds.size.width.as_f32(),
            doc_bounds.size.height.as_f32(),
        );
        let (view_w, view_h) = (viewport.width.as_f32(), viewport.height.as_f32());
        if doc_w <= 0. || doc_h <= 0. || view_w <= 0. || view_h <= 0. {
            self.reset_view(cx);
            return;
        }
        let zoom = (0.9 * (view_w / doc_w).min(view_h / doc_h)).clamp(MIN_ZOOM, MAX_ZOOM);
        let viewport_center = point(px(view_w / 2.), px(view_h / 2.));
        self.transform.pan = viewport_center - doc_bounds.center() * zoom;
        self.transform.zoom = zoom;
        cx.emit(CanvasEvent::ViewTransformed);
        cx.notify();
    }

    pub fn reset_view(&mut self, cx: &mut Context<Self>) {
        self.transform = CanvasTransform::IDENTITY;
        cx.emit(CanvasEvent::ViewTransformed);
        cx.notify();
    }

    pub fn selected_shape_ids(&self) -> &[ShapeId] {
        &self.selection
    }

    pub fn hit_test(&self, doc_point: Point<Pixels>, tolerance: Pixels) -> Option<ShapeId> {
        self.document
            .layers()
            .iter()
            .rev()
            .filter(|layer| layer.is_visible())
            .find_map(|layer| {
                layer
                    .shapes()
                    .iter()
                    .rev()
                    .find(|(_, shape)| shape.hit_test(doc_point, tolerance))
                    .map(|(id, _)| *id)
            })
    }

    pub fn select_shape(&mut self, id: ShapeId, additive: bool, cx: &mut Context<Self>) {
        let next = if additive {
            let mut next = self.selection.clone();
            if !next.contains(&id) {
                next.push(id);
            }
            next
        } else {
            vec![id]
        };
        if next == self.selection {
            return;
        }
        self.selection = next;
        cx.emit(CanvasEvent::SelectionChanged);
        cx.notify();
    }

    pub fn clear_selection(&mut self, cx: &mut Context<Self>) {
        if self.selection.is_empty() {
            return;
        }
        self.selection.clear();
        cx.emit(CanvasEvent::SelectionChanged);
        cx.notify();
    }

    /// Transient shape painted above the document, below the selection
    /// outline. Not hit-testable and not part of the document; `set_tool`
    /// clears it.
    pub fn set_tool_overlay(&mut self, overlay: Option<CanvasShape>, cx: &mut Context<Self>) {
        self.overlay = overlay;
        cx.notify();
    }

    pub fn tool_overlay(&self) -> Option<&CanvasShape> {
        self.overlay.as_ref()
    }

    pub fn is_pointer_active(&self) -> bool {
        self.pointer_active
    }

    pub fn set_tool(&mut self, tool: Box<dyn CanvasTool>, cx: &mut Context<Self>) {
        self.tool = tool;
        self.overlay = None;
        cx.emit(CanvasEvent::ToolChanged);
        cx.notify();
    }

    pub fn set_builtin_tool(&mut self, tool: BuiltinTool, cx: &mut Context<Self>) {
        self.set_tool(tool.into_boxed_tool(), cx);
    }

    pub fn active_tool_id(&self) -> &str {
        self.tool.id()
    }

    pub fn active_tool_cursor(&self) -> CursorStyle {
        self.tool.cursor()
    }
}

#[cfg(test)]
#[path = "state.test.rs"]
mod state_test;
