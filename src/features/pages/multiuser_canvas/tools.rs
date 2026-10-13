use gpui_kit::*;

use super::session::{CollabSession, canvas_shape_to_wire, wire_color_to_hsla};
use crate::ui::canvas::{
    CanvasPointerEvent, CanvasShape, CanvasState, CanvasTool, ShapeId, StrokeStyle, ToolContext,
};

pub(crate) struct PenTool {
    session: Entity<CollabSession>,
    color: Hsla,
    width: Pixels,
    points: Vec<Point<Pixels>>,
    stroking: bool,
}

impl PenTool {
    pub(crate) fn new(session: Entity<CollabSession>, cx: &App) -> Self {
        let color = wire_color_to_hsla(session.read(cx).identity().color);
        Self {
            session,
            color,
            width: px(3.),
            points: Vec::new(),
            stroking: false,
        }
    }

    fn stroke_style(&self) -> StrokeStyle {
        StrokeStyle {
            width: self.width,
            color: self.color,
            dash: None,
        }
    }

    fn stroke_shape(&self) -> CanvasShape {
        CanvasShape::path(self.points.clone(), self.stroke_style())
    }

    fn report_cursor(&self, event: &CanvasPointerEvent, cx: &mut Context<CanvasState>) {
        let doc = event.doc_position;
        self.session
            .update(cx, |session, cx| session.set_own_cursor(doc, cx));
    }
}

impl CanvasTool for PenTool {
    fn id(&self) -> &'static str {
        "pen"
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
        self.report_cursor(event, cx);
        if event.button != MouseButton::Left {
            return;
        }
        self.points = vec![event.doc_position];
        self.stroking = true;
        state.set_tool_overlay(Some(self.stroke_shape()), cx);
    }

    fn pointer_move(
        &mut self,
        event: &CanvasPointerEvent,
        tool_ctx: &ToolContext,
        state: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) {
        self.report_cursor(event, cx);
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
        state.set_tool_overlay(Some(self.stroke_shape()), cx);
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
        let shape = self.stroke_shape();
        let layer = state
            .document()
            .top_layer()
            .unwrap_or_else(|| state.add_layer(cx));
        let local_id = state.add_shape(layer, shape.clone(), cx);
        state.set_tool_overlay(None, cx);
        self.stroking = false;
        self.points.clear();
        if let Some(local_id) = local_id {
            self.session
                .update(cx, |session, cx| session.publish_add(&shape, local_id, cx));
        }
    }
}

pub(crate) struct SelectTool {
    session: Entity<CollabSession>,
    move_origin: Option<Point<Pixels>>,
    snapshot: Vec<(ShapeId, CanvasShape)>,
}

impl SelectTool {
    pub(crate) fn new(session: Entity<CollabSession>) -> Self {
        Self {
            session,
            move_origin: None,
            snapshot: Vec::new(),
        }
    }

    fn report_cursor(&self, event: &CanvasPointerEvent, cx: &mut Context<CanvasState>) {
        let doc = event.doc_position;
        self.session
            .update(cx, |session, cx| session.set_own_cursor(doc, cx));
    }
}

impl CanvasTool for SelectTool {
    fn id(&self) -> &'static str {
        "select"
    }

    fn cursor(&self) -> CursorStyle {
        CursorStyle::Arrow
    }

    fn pointer_down(
        &mut self,
        event: &CanvasPointerEvent,
        tool_ctx: &ToolContext,
        state: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) {
        self.report_cursor(event, cx);
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
        self.report_cursor(event, cx);
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
        event: &CanvasPointerEvent,
        _: &ToolContext,
        state: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) {
        self.report_cursor(event, cx);
        let snapshot = std::mem::take(&mut self.snapshot);
        self.move_origin = None;
        for (id, snapshot_shape) in &snapshot {
            let Some(current) = state.document().shape(*id) else {
                continue;
            };
            if canvas_shape_to_wire(current) == canvas_shape_to_wire(snapshot_shape) {
                continue;
            }
            let current = current.clone();
            self.session
                .update(cx, |session, cx| session.publish_update(*id, &current, cx));
        }
    }
}

#[cfg(test)]
#[path = "tools.test.rs"]
mod tools_test;
