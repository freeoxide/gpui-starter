use gpui_kit::component::{
    ActiveTheme as _, IconName, Sizable as _,
    button::{Button, ButtonVariants as _},
};
use gpui_kit::{prelude::*, *};

use crate::ui::canvas::{Canvas, *};

use super::geometry::{self, sample_cubic_bezier};

const CURVE_SEGMENTS: usize = 24;
const ANCHOR_SIDE: Pixels = px(12.);
const CONTROL_SIDE: Pixels = px(10.);

pub(super) struct BezierDemo {
    state: Entity<CanvasState>,
    _subscription: Subscription,
}

impl BezierDemo {
    pub(super) fn new(cx: &mut Context<Self>) -> Self {
        let state = cx.new(|_| CanvasState::new());
        state.update(cx, |s, cx| {
            let tool = BezierEditTool::seed(s, cx);
            s.set_tool(Box::new(tool), cx);
        });
        let _subscription = cx.observe(&state, |_, _, cx| cx.notify());
        Self {
            state,
            _subscription,
        }
    }

    fn reset(&mut self, cx: &mut Context<Self>) {
        self.state.update(cx, |s, cx| {
            let tool = BezierEditTool::seed(s, cx);
            s.set_tool(Box::new(tool), cx);
        });
    }
}

impl Render for BezierDemo {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        super::render_demo_shell(
            "bezier",
            crate::i18n::localize(super::CanvasDemo::Bezier.title_key()),
            crate::i18n::localize(super::CanvasDemo::Bezier.desc_key()),
            vec![
                Button::new("bezier-reset")
                    .ghost()
                    .small()
                    .icon(IconName::RotateCw)
                    .label(crate::i18n::localize("showcase_reset"))
                    .on_click(cx.listener(|this, _, _, cx| this.reset(cx)))
                    .into_any_element(),
            ],
            Canvas::new("bezier-canvas", self.state.clone()),
            cx,
        )
    }
}

pub(super) struct BezierEditTool {
    points: [Point<Pixels>; 4],
    handle_ids: [ShapeId; 4],
    curve_id: ShapeId,
    guide_ids: [ShapeId; 2],
    dragging: Option<usize>,
}

impl BezierEditTool {
    pub(super) fn seed(state: &mut CanvasState, cx: &mut Context<CanvasState>) -> Self {
        state.edit_document(cx, |doc| doc.clear());
        let guides_layer = state.add_layer(cx);
        let curve_layer = state.add_layer(cx);
        let handles_layer = state.add_layer(cx);
        state.set_grid(
            CanvasGrid {
                visible: false,
                spacing: px(16.),
            },
            cx,
        );
        let [p0, c1, c2, p1] = [
            point(px(120.), px(260.)),
            point(px(200.), px(80.)),
            point(px(360.), px(440.)),
            point(px(460.), px(260.)),
        ];
        let (foreground, muted_foreground, primary, background) = {
            let theme = cx.theme();
            (
                theme.foreground,
                theme.muted_foreground,
                theme.primary,
                theme.background,
            )
        };
        let (curve_id, guide_ids, handle_ids) = state.edit_document(cx, |doc| {
            let curve_id = doc
                .add_shape(
                    curve_layer,
                    CanvasShape::path(
                        sample_cubic_bezier(p0, c1, c2, p1, CURVE_SEGMENTS),
                        StrokeStyle {
                            width: px(2.),
                            color: foreground,
                            dash: None,
                        },
                    ),
                )
                .unwrap_or_default();
            let guide_ids = [
                doc.add_shape(
                    guides_layer,
                    CanvasShape::path(
                        vec![p0, c1],
                        StrokeStyle {
                            width: px(1.),
                            color: muted_foreground.opacity(0.6),
                            dash: None,
                        },
                    ),
                )
                .unwrap_or_default(),
                doc.add_shape(
                    guides_layer,
                    CanvasShape::path(
                        vec![c2, p1],
                        StrokeStyle {
                            width: px(1.),
                            color: muted_foreground.opacity(0.6),
                            dash: None,
                        },
                    ),
                )
                .unwrap_or_default(),
            ];
            let handle_ids = [
                doc.add_shape(
                    handles_layer,
                    CanvasShape::ellipse(
                        geometry::square_at(p0, ANCHOR_SIDE),
                        ShapeStyle::filled(primary),
                    ),
                )
                .unwrap_or_default(),
                doc.add_shape(
                    handles_layer,
                    CanvasShape::ellipse(
                        geometry::square_at(c1, CONTROL_SIDE),
                        ShapeStyle::both(
                            background,
                            StrokeStyle {
                                width: px(1.),
                                color: muted_foreground,
                                dash: None,
                            },
                        ),
                    ),
                )
                .unwrap_or_default(),
                doc.add_shape(
                    handles_layer,
                    CanvasShape::ellipse(
                        geometry::square_at(c2, CONTROL_SIDE),
                        ShapeStyle::both(
                            background,
                            StrokeStyle {
                                width: px(1.),
                                color: muted_foreground,
                                dash: None,
                            },
                        ),
                    ),
                )
                .unwrap_or_default(),
                doc.add_shape(
                    handles_layer,
                    CanvasShape::ellipse(
                        geometry::square_at(p1, ANCHOR_SIDE),
                        ShapeStyle::filled(primary),
                    ),
                )
                .unwrap_or_default(),
            ];
            (curve_id, guide_ids, handle_ids)
        });
        Self {
            points: [p0, c1, c2, p1],
            handle_ids,
            curve_id,
            guide_ids,
            dragging: None,
        }
    }
}

impl CanvasTool for BezierEditTool {
    fn id(&self) -> &'static str {
        "bezier-edit"
    }

    fn cursor(&self) -> CursorStyle {
        CursorStyle::Crosshair
    }

    fn pointer_down(
        &mut self,
        event: &CanvasPointerEvent,
        tool_ctx: &ToolContext,
        _state: &mut CanvasState,
        _cx: &mut Context<CanvasState>,
    ) {
        if event.button != MouseButton::Left {
            return;
        }
        if let Some(hit) = tool_ctx.hit {
            if let Some(ix) = self.handle_ids.iter().position(|id| *id == hit) {
                self.dragging = Some(ix);
            }
        }
    }

    fn pointer_move(
        &mut self,
        event: &CanvasPointerEvent,
        _: &ToolContext,
        state: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) {
        let Some(ix) = self.dragging else {
            return;
        };
        self.points[ix] = event.doc_position;
        let position = event.doc_position;
        let [p0, c1, c2, p1] = self.points;
        let handle_side = if matches!(ix, 0 | 3) {
            ANCHOR_SIDE
        } else {
            CONTROL_SIDE
        };
        let handle_ids = self.handle_ids;
        let guide_ids = self.guide_ids;
        let guide_ix = if ix < 2 { 0 } else { 1 };
        let curve_id = self.curve_id;
        state.edit_document(cx, |doc| {
            doc.update_shape(handle_ids[ix], |shape| {
                if let CanvasShape::Ellipse { bounds, .. } = shape {
                    *bounds = geometry::square_at(position, handle_side);
                }
            });
            doc.update_shape(guide_ids[guide_ix], |shape| {
                if let CanvasShape::Path { points, .. } = shape {
                    *points = if guide_ix == 0 {
                        vec![p0, c1]
                    } else {
                        vec![c2, p1]
                    };
                }
            });
            doc.update_shape(curve_id, |shape| {
                if let CanvasShape::Path { points, .. } = shape {
                    *points = sample_cubic_bezier(p0, c1, c2, p1, CURVE_SEGMENTS);
                }
            });
        });
    }

    fn pointer_up(
        &mut self,
        _: &CanvasPointerEvent,
        _: &ToolContext,
        _: &mut CanvasState,
        _: &mut Context<CanvasState>,
    ) {
        self.dragging = None;
    }
}

#[cfg(test)]
#[path = "bezier.test.rs"]
mod bezier_test;
