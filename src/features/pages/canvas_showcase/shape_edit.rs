use gpui_kit::component::{
    ActiveTheme as _, IconName, Sizable as _,
    button::{Button, ButtonVariants as _},
};
use gpui_kit::{prelude::*, *};

use crate::accessibility::A11yExt as _;
use crate::ui::canvas::{Canvas, *};

use super::geometry::{self, HANDLE_SIDE};

pub(super) struct ShapesDemo {
    state: Entity<CanvasState>,
    _subscription: Subscription,
}

impl ShapesDemo {
    pub(super) fn new(cx: &mut Context<Self>) -> Self {
        let state = cx.new(|_| CanvasState::new());
        state.update(cx, |s, cx| {
            let tool = ShapeEditTool::seed(s, cx);
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
            s.clear_selection(cx);
            let tool = ShapeEditTool::seed(s, cx);
            s.set_tool(Box::new(tool), cx);
        });
    }
}

impl Render for ShapesDemo {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.state.read(cx).selected_shape_ids().len();
        let count = selected.to_string();
        let readout = crate::i18n::localize_with_args(
            "showcase_selected_count",
            &[("count", count.as_str())],
        );
        super::render_demo_shell(
            "shapes",
            crate::i18n::localize(super::CanvasDemo::Shapes.title_key()),
            crate::i18n::localize(super::CanvasDemo::Shapes.desc_key()),
            vec![
                Button::new("shapes-reset")
                    .ghost()
                    .small()
                    .icon(IconName::RotateCw)
                    .label(crate::i18n::localize("showcase_reset"))
                    .on_click(cx.listener(|this, _, _, cx| this.reset(cx)))
                    .into_any_element(),
                div()
                    .id("shapes-selected-count")
                    .a11y(Role::Status, readout.clone())
                    .a11y_live(accesskit::Live::Polite)
                    .text_color(cx.theme().muted_foreground)
                    .child(readout)
                    .into_any_element(),
            ],
            Canvas::new("shapes-canvas", self.state.clone()),
            cx,
        )
    }
}

struct MoveDrag {
    snapshot: Vec<(ShapeId, CanvasShape)>,
    origin: Point<Pixels>,
}

struct ResizeDrag {
    handle_ix: usize,
    start_bounds: Bounds<Pixels>,
    shape_id: ShapeId,
    path_snapshot: Option<Vec<Point<Pixels>>>,
}

pub(super) struct ShapeEditTool {
    handles_layer: LayerId,
    handle_ids: Option<[ShapeId; 8]>,
    move_drag: Option<MoveDrag>,
    resize_drag: Option<ResizeDrag>,
}

impl ShapeEditTool {
    pub(super) fn seed(state: &mut CanvasState, cx: &mut Context<CanvasState>) -> Self {
        state.edit_document(cx, |doc| doc.clear());
        let content_layer = state.add_layer(cx);
        let handles_layer = state.add_layer(cx);
        state.set_grid(
            CanvasGrid {
                visible: false,
                spacing: px(16.),
            },
            cx,
        );
        let (primary, accent, secondary, foreground) = {
            let theme = cx.theme();
            (
                theme.primary,
                theme.accent,
                theme.secondary,
                theme.foreground,
            )
        };
        state.edit_document(cx, |doc| {
            doc.add_shape(
                content_layer,
                CanvasShape::rect(
                    Bounds {
                        origin: point(px(80.), px(80.)),
                        size: size(px(180.), px(100.)),
                    },
                    ShapeStyle::both(
                        primary.opacity(0.30),
                        StrokeStyle {
                            width: px(1.),
                            color: foreground.opacity(0.7),
                            dash: None,
                        },
                    ),
                ),
            );
            doc.add_shape(
                content_layer,
                CanvasShape::ellipse(
                    Bounds {
                        origin: point(px(320.), px(100.)),
                        size: size(px(160.), px(120.)),
                    },
                    ShapeStyle::filled(accent.opacity(0.35)),
                ),
            );
            doc.add_shape(
                content_layer,
                CanvasShape::rect(
                    Bounds {
                        origin: point(px(140.), px(280.)),
                        size: size(px(160.), px(100.)),
                    },
                    ShapeStyle::filled(secondary.opacity(0.35)),
                ),
            );
            doc.add_shape(
                content_layer,
                CanvasShape::path(
                    vec![
                        point(px(100.), px(430.)),
                        point(px(160.), px(390.)),
                        point(px(220.), px(450.)),
                        point(px(280.), px(410.)),
                    ],
                    StrokeStyle {
                        width: px(2.),
                        color: foreground,
                        dash: None,
                    },
                ),
            );
        });
        Self {
            handles_layer,
            handle_ids: None,
            move_drag: None,
            resize_drag: None,
        }
    }

    fn place_handles(&self, doc: &mut CanvasDocument, selected_id: ShapeId) {
        let Some(ids) = self.handle_ids else {
            return;
        };
        let Some(shape) = doc.shape(selected_id) else {
            return;
        };
        let positions = geometry::handle_positions(shape.doc_bounds());
        for (id, position) in ids.iter().zip(positions) {
            doc.update_shape(*id, |handle| {
                if let CanvasShape::Rect { bounds, .. } = handle {
                    *bounds = geometry::square_at(position, HANDLE_SIDE);
                }
            });
        }
    }

    fn sync_handles(&mut self, state: &mut CanvasState, cx: &mut Context<CanvasState>) {
        let selected = state.selected_shape_ids();
        if selected.len() == 1 {
            let selected_id = selected[0];
            match self.handle_ids {
                Some(_) => {
                    state.edit_document(cx, |doc| self.place_handles(doc, selected_id));
                }
                None => {
                    let Some(shape) = state.document().shape(selected_id) else {
                        return;
                    };
                    let positions = geometry::handle_positions(shape.doc_bounds());
                    let fill = ShapeStyle::filled(cx.theme().foreground);
                    let handles_layer = self.handles_layer;
                    let ids: [ShapeId; 8] = state.edit_document(cx, |doc| {
                        let mut ids = [ShapeId::default(); 8];
                        for (ix, id) in ids.iter_mut().enumerate() {
                            *id = doc
                                .add_shape(
                                    handles_layer,
                                    CanvasShape::rect(
                                        geometry::square_at(positions[ix], HANDLE_SIDE),
                                        fill.clone(),
                                    ),
                                )
                                .unwrap_or_default();
                        }
                        ids
                    });
                    self.handle_ids = Some(ids);
                }
            }
        } else if let Some(ids) = self.handle_ids.take() {
            state.edit_document(cx, |doc| {
                for id in ids {
                    doc.remove_shape(id);
                }
            });
        }
    }
}

impl CanvasTool for ShapeEditTool {
    fn id(&self) -> &'static str {
        "shape-edit"
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
            let handle_ix = self
                .handle_ids
                .and_then(|ids| ids.iter().position(|id| *id == hit));
            if let Some(handle_ix) = handle_ix {
                let selected = state.selected_shape_ids();
                if selected.len() == 1 {
                    if let Some(shape) = state.document().shape(selected[0]) {
                        let path_snapshot = match shape {
                            CanvasShape::Path { points, .. } => Some(points.clone()),
                            _ => None,
                        };
                        self.resize_drag = Some(ResizeDrag {
                            handle_ix,
                            start_bounds: shape.doc_bounds(),
                            shape_id: selected[0],
                            path_snapshot,
                        });
                    }
                }
                self.sync_handles(state, cx);
                return;
            }
            if !state.selected_shape_ids().contains(&hit) {
                state.select_shape(hit, event.modifiers.shift, cx);
            }
            let snapshot = state
                .selected_shape_ids()
                .iter()
                .filter_map(|id| {
                    state
                        .document()
                        .shape(*id)
                        .map(|shape| (*id, shape.clone()))
                })
                .collect();
            self.move_drag = Some(MoveDrag {
                snapshot,
                origin: event.doc_position,
            });
        } else if !event.modifiers.shift {
            state.clear_selection(cx);
        }
        self.sync_handles(state, cx);
    }

    fn pointer_move(
        &mut self,
        event: &CanvasPointerEvent,
        _: &ToolContext,
        state: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) {
        if let Some((handle_ix, start_bounds, shape_id)) = self
            .resize_drag
            .as_ref()
            .map(|drag| (drag.handle_ix, drag.start_bounds, drag.shape_id))
        {
            let pointer = event.doc_position;
            let path_snapshot = self
                .resize_drag
                .as_ref()
                .and_then(|drag| drag.path_snapshot.clone());
            state.edit_document(cx, |doc| {
                doc.update_shape(shape_id, |shape| match shape {
                    CanvasShape::Rect { bounds, .. } | CanvasShape::Ellipse { bounds, .. } => {
                        *bounds = geometry::resize_bounds(start_bounds, handle_ix, pointer);
                    }
                    CanvasShape::Path { points, .. } => {
                        if let Some(snapshot) = &path_snapshot {
                            *points = geometry::scale_points(
                                snapshot,
                                start_bounds,
                                geometry::resize_bounds(start_bounds, handle_ix, pointer),
                            );
                        }
                    }
                    _ => {}
                });
                self.place_handles(doc, shape_id);
            });
        } else if let Some(origin) = self.move_drag.as_ref().map(|drag| drag.origin) {
            let delta = event.doc_position - origin;
            state.edit_document(cx, |doc| {
                let Some(drag) = self.move_drag.as_ref() else {
                    return;
                };
                for (id, snapshot_shape) in &drag.snapshot {
                    doc.update_shape(*id, |shape| {
                        *shape = snapshot_shape.clone();
                        shape.translate(delta);
                    });
                }
                if let Some((primary_id, _)) = drag.snapshot.first() {
                    self.place_handles(doc, *primary_id);
                }
            });
        } else {
            let single_selection = state.selected_shape_ids().len() == 1;
            if single_selection != self.handle_ids.is_some() {
                self.sync_handles(state, cx);
            }
        }
    }

    fn pointer_up(
        &mut self,
        _: &CanvasPointerEvent,
        _: &ToolContext,
        _state: &mut CanvasState,
        _cx: &mut Context<CanvasState>,
    ) {
        self.move_drag = None;
        self.resize_drag = None;
    }
}

#[cfg(test)]
#[path = "shape_edit.test.rs"]
mod shape_edit_test;
