use gpui_kit::component::{
    ActiveTheme as _, IconName, Sizable as _,
    button::{Button, ButtonVariants as _},
};
use gpui_kit::{prelude::*, *};

use crate::accessibility::A11yExt as _;
use crate::ui::canvas::{Canvas, *};

use super::geometry::{rect_from_corners, sample_cubic_bezier};

const NODE_W: Pixels = px(120.);
const NODE_H: Pixels = px(56.);

pub(super) struct NodeGraphDemo {
    state: Entity<CanvasState>,
    _subscription: Subscription,
}

impl NodeGraphDemo {
    pub(super) fn new(cx: &mut Context<Self>) -> Self {
        let state = cx.new(|_| CanvasState::new());
        state.update(cx, |s, cx| {
            let tool = NodeGraphTool::seed(s, cx);
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
            let tool = NodeGraphTool::seed(s, cx);
            s.set_tool(Box::new(tool), cx);
        });
    }
}

impl Render for NodeGraphDemo {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let selected = self.state.read(cx).selected_shape_ids().len();
        let count = selected.to_string();
        let readout = crate::i18n::localize_with_args(
            "showcase_selected_count",
            &[("count", count.as_str())],
        );
        super::render_demo_shell(
            "nodes",
            crate::i18n::localize(super::CanvasDemo::Nodes.title_key()),
            crate::i18n::localize(super::CanvasDemo::Nodes.desc_key()),
            vec![
                Button::new("nodes-reset")
                    .ghost()
                    .small()
                    .icon(IconName::RotateCw)
                    .label(crate::i18n::localize("showcase_reset"))
                    .on_click(cx.listener(|this, _, _, cx| this.reset(cx)))
                    .into_any_element(),
                div()
                    .id("nodes-selected-count")
                    .a11y(Role::Status, readout.clone())
                    .a11y_live(accesskit::Live::Polite)
                    .text_color(cx.theme().muted_foreground)
                    .child(readout)
                    .into_any_element(),
            ],
            Canvas::new("nodes-canvas", self.state.clone()),
            cx,
        )
    }
}

struct NodeCell {
    bounds: Bounds<Pixels>,
    rect_id: ShapeId,
    label_id: ShapeId,
}

struct EdgeCell {
    from: usize,
    to: usize,
    path_id: ShapeId,
}

struct NodeDrag {
    node_ix: usize,
    origin: Point<Pixels>,
}

struct Band {
    start: Point<Pixels>,
    current: Point<Pixels>,
}

pub(super) struct NodeGraphTool {
    nodes: Vec<NodeCell>,
    edges: Vec<EdgeCell>,
    node_drag: Option<NodeDrag>,
    band: Option<Band>,
}

impl NodeGraphTool {
    pub(super) fn seed(state: &mut CanvasState, cx: &mut Context<CanvasState>) -> Self {
        state.edit_document(cx, |doc| doc.clear());
        let edges_layer = state.add_layer(cx);
        let nodes_layer = state.add_layer(cx);
        state.set_grid(
            CanvasGrid {
                visible: false,
                spacing: px(16.),
            },
            cx,
        );
        let (group_box, border, foreground, muted_foreground) = {
            let theme = cx.theme();
            (
                theme.group_box,
                theme.border,
                theme.foreground,
                theme.muted_foreground,
            )
        };
        let seeds: [(&'static str, (f32, f32)); 4] = [
            ("Input", (100., 120.)),
            ("Filter", (320., 100.)),
            ("Transform", (330., 260.)),
            ("Output", (560., 170.)),
        ];
        let mut nodes = Vec::new();
        let edge_seeds: [(usize, usize); 4] = [(0, 1), (0, 2), (1, 3), (2, 3)];
        let mut edges = Vec::new();
        state.edit_document(cx, |doc| {
            for (title, (x, y)) in seeds {
                let bounds = Bounds {
                    origin: point(px(x), px(y)),
                    size: size(NODE_W, NODE_H),
                };
                let rect_id = doc
                    .add_shape(
                        nodes_layer,
                        CanvasShape::rect(
                            bounds,
                            ShapeStyle::both(
                                group_box,
                                StrokeStyle {
                                    width: px(1.),
                                    color: border,
                                    dash: None,
                                },
                            ),
                        ),
                    )
                    .unwrap_or_default();
                let label_id = doc
                    .add_shape(
                        nodes_layer,
                        CanvasShape::text(
                            bounds.origin + point(px(8.), px(8.)),
                            title,
                            px(14.),
                            foreground,
                        ),
                    )
                    .unwrap_or_default();
                nodes.push(NodeCell {
                    bounds,
                    rect_id,
                    label_id,
                });
            }
            for (from, to) in edge_seeds {
                let path_id = doc
                    .add_shape(
                        edges_layer,
                        CanvasShape::path(
                            edge_points(&nodes[from], &nodes[to]),
                            StrokeStyle {
                                width: px(1.5),
                                color: muted_foreground,
                                dash: None,
                            },
                        ),
                    )
                    .unwrap_or_default();
                edges.push(EdgeCell { from, to, path_id });
            }
        });
        Self {
            nodes,
            edges,
            node_drag: None,
            band: None,
        }
    }

    fn node_at_hit(&self, hit: ShapeId) -> Option<usize> {
        self.nodes
            .iter()
            .position(|node| node.rect_id == hit || node.label_id == hit)
    }

    fn band_overlay_shape(start: Point<Pixels>, current: Point<Pixels>, fill: Hsla) -> CanvasShape {
        CanvasShape::rect(rect_from_corners(start, current), ShapeStyle::filled(fill))
    }
}

fn edge_points(from: &NodeCell, to: &NodeCell) -> Vec<Point<Pixels>> {
    let s = point(
        px(from.bounds.origin.x.as_f32() + NODE_W.as_f32()),
        px(from.bounds.origin.y.as_f32() + NODE_H.as_f32() / 2.),
    );
    let t = point(
        px(to.bounds.origin.x.as_f32()),
        px(to.bounds.origin.y.as_f32() + NODE_H.as_f32() / 2.),
    );
    let off = ((t.x - s.x).as_f32() / 2.).clamp(40., 120.);
    sample_cubic_bezier(
        s,
        point(px(s.x.as_f32() + off), s.y),
        point(px(t.x.as_f32() - off), t.y),
        t,
        20,
    )
}

impl CanvasTool for NodeGraphTool {
    fn id(&self) -> &'static str {
        "node-graph"
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
            if let Some(node_ix) = self.node_at_hit(hit) {
                let rect_id = self.nodes[node_ix].rect_id;
                if event.modifiers.shift {
                    state.select_shape(rect_id, true, cx);
                } else if !state.selected_shape_ids().contains(&rect_id) {
                    state.select_shape(rect_id, false, cx);
                }
                self.node_drag = Some(NodeDrag {
                    node_ix,
                    origin: event.doc_position,
                });
                return;
            }
        }
        let start = event.doc_position;
        self.band = Some(Band {
            start,
            current: start,
        });
        let fill = cx.theme().primary.opacity(0.08);
        state.set_tool_overlay(Some(Self::band_overlay_shape(start, start, fill)), cx);
        if !event.modifiers.shift {
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
        if let Some(drag) = self.node_drag.as_mut() {
            let delta = event.doc_position - drag.origin;
            drag.origin = event.doc_position;
            let node = &mut self.nodes[drag.node_ix];
            node.bounds.origin += delta;
            let node_ix = drag.node_ix;
            let resampled: Vec<(ShapeId, Vec<Point<Pixels>>)> = self
                .edges
                .iter()
                .filter(|edge| edge.from == node_ix || edge.to == node_ix)
                .map(|edge| {
                    (
                        edge.path_id,
                        edge_points(&self.nodes[edge.from], &self.nodes[edge.to]),
                    )
                })
                .collect();
            let nodes = &self.nodes;
            state.edit_document(cx, |doc| {
                let node = &nodes[node_ix];
                doc.update_shape(node.rect_id, |shape| {
                    if let CanvasShape::Rect { bounds, .. } = shape {
                        *bounds = node.bounds;
                    }
                });
                doc.update_shape(node.label_id, |shape| {
                    if let CanvasShape::Text { position, .. } = shape {
                        *position = node.bounds.origin + point(px(8.), px(8.));
                    }
                });
                for (path_id, points) in &resampled {
                    doc.update_shape(*path_id, |shape| {
                        if let CanvasShape::Path { points: p, .. } = shape {
                            *p = points.clone();
                        }
                    });
                }
            });
        } else if let Some(band) = self.band.as_mut() {
            band.current = event.doc_position;
            let fill = cx.theme().primary.opacity(0.08);
            state.set_tool_overlay(
                Some(Self::band_overlay_shape(band.start, band.current, fill)),
                cx,
            );
        }
    }

    fn pointer_up(
        &mut self,
        event: &CanvasPointerEvent,
        _: &ToolContext,
        state: &mut CanvasState,
        cx: &mut Context<CanvasState>,
    ) {
        if let Some(band) = self.band.take() {
            let band_rect = rect_from_corners(band.start, band.current);
            if !event.modifiers.shift {
                state.clear_selection(cx);
            }
            for node in &self.nodes {
                if node.bounds.intersects(&band_rect) {
                    state.select_shape(node.rect_id, true, cx);
                }
            }
            state.set_tool_overlay(None, cx);
        }
        self.node_drag = None;
    }
}

#[cfg(test)]
#[path = "node_graph.test.rs"]
mod node_graph_test;
