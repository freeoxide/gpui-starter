use gpui_kit::prelude::*;
use gpui_kit::{
    Bounds, Entity, Modifiers, MouseButton, Subscription, TestAppContext, Window, black, div,
    point, px, red, size,
};

use crate::ui::canvas::{
    BrushConfig, Canvas, CanvasPointerEvent, CanvasShape, CanvasState, ShapeKind, ToolContext,
};

fn pointer(screen: (f32, f32), button: MouseButton) -> CanvasPointerEvent {
    let position = point(px(screen.0), px(screen.1));
    CanvasPointerEvent {
        screen_position: position,
        doc_position: position,
        button,
        click_count: 1,
        modifiers: Modifiers::none(),
    }
}

fn tool_ctx(canvas: &CanvasState) -> ToolContext {
    ToolContext {
        viewport: Bounds {
            origin: point(px(0.), px(0.)),
            size: size(px(480.), px(360.)),
        },
        transform: canvas.transform(),
        hit: None,
    }
}

struct ObsHost {
    canvas: Entity<CanvasState>,
    observed: usize,
    _subscriptions: Vec<Subscription>,
}

impl ObsHost {
    fn new(canvas: Entity<CanvasState>, cx: &mut Context<Self>) -> Self {
        let subscription = cx.observe(&canvas, |this, _, cx| {
            this.observed += 1;
            cx.notify();
        });
        Self {
            canvas,
            observed: 0,
            _subscriptions: vec![subscription],
        }
    }
}

impl Render for ObsHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .child(Canvas::new("obs-host-canvas", self.canvas.clone()))
    }
}

#[gpui_kit::test]
fn new_paint_canvas_mounts_a_brush_on_one_layer(cx: &mut TestAppContext) {
    let canvas = cx.update(super::new_paint_canvas);
    cx.update(|cx| {
        assert_eq!(canvas.read(cx).active_tool_id(), "brush");
        assert_eq!(canvas.read(cx).document().layers().len(), 1);
    });
}

#[gpui_kit::test]
fn brush_stroke_commits_exactly_one_path_shape(cx: &mut TestAppContext) {
    let canvas = cx.update(super::new_paint_canvas);
    cx.update(|cx| {
        let ctx = tool_ctx(canvas.read(cx));
        canvas.update(cx, |state, cx| {
            state.dispatch_tool_event(cx, |tool, state, cx| {
                tool.pointer_down(&pointer((10., 10.), MouseButton::Left), &ctx, state, cx)
            });
            state.dispatch_tool_event(cx, |tool, state, cx| {
                tool.pointer_move(&pointer((30., 12.), MouseButton::Left), &ctx, state, cx)
            });
            state.dispatch_tool_event(cx, |tool, state, cx| {
                tool.pointer_move(&pointer((50., 20.), MouseButton::Left), &ctx, state, cx)
            });
            state.dispatch_tool_event(cx, |tool, state, cx| {
                tool.pointer_up(&pointer((50., 20.), MouseButton::Left), &ctx, state, cx)
            });
        });
        let doc = canvas.read(cx).document();
        assert_eq!(doc.shapes().count(), 1);
        assert!(matches!(
            doc.shapes().next().unwrap().2,
            CanvasShape::Path { .. }
        ));
    });
}

#[gpui_kit::test]
async fn one_entity_update_notifies_hosts_in_both_windows(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let canvas = cx.update(super::new_paint_canvas);
    let host_one = cx.open_window(size(px(300.), px(200.)), {
        let canvas = canvas.clone();
        |_, cx| ObsHost::new(canvas, cx)
    });
    let host_two = cx.open_window(size(px(300.), px(200.)), {
        let canvas = canvas.clone();
        |_, cx| ObsHost::new(canvas, cx)
    });
    cx.update(|cx| canvas.update(cx, |state, cx| state.pan_by(point(px(5.), px(0.)), cx)));
    let observed_one = cx.update(|cx| host_one.read(cx).unwrap().observed);
    let observed_two = cx.update(|cx| host_two.read(cx).unwrap().observed);
    assert_eq!(observed_one, 1);
    assert_eq!(observed_two, 1);
    assert_eq!(cx.read(|cx| canvas.read(cx).pan()), point(px(5.), px(0.)));
}

fn drag_shape(canvas: &Entity<CanvasState>, cx: &mut TestAppContext) {
    cx.update(|cx| {
        let ctx = tool_ctx(canvas.read(cx));
        canvas.update(cx, |state, cx| {
            state.dispatch_tool_event(cx, |tool, state, cx| {
                tool.pointer_down(&pointer((20., 20.), MouseButton::Left), &ctx, state, cx)
            });
            state.dispatch_tool_event(cx, |tool, state, cx| {
                tool.pointer_move(&pointer((90., 60.), MouseButton::Left), &ctx, state, cx)
            });
            state.dispatch_tool_event(cx, |tool, state, cx| {
                tool.pointer_up(&pointer((90., 60.), MouseButton::Left), &ctx, state, cx)
            });
        });
    });
}

#[gpui_kit::test]
fn shape_tool_commits_the_configured_style(cx: &mut TestAppContext) {
    let canvas = cx.update(super::new_paint_canvas);
    let panel = cx.update(|cx| cx.new(|cx| super::MiniPaintPanel::new(canvas.clone(), cx)));

    // The width toggles ride set_brush even while a shape tool is active, so
    // the committed stroke must carry the chosen width, not the default.
    cx.update(|cx| {
        panel.update(cx, |panel, cx| {
            panel.tool = super::PaintTool::Shape(ShapeKind::Rect);
            panel.set_brush(
                BrushConfig {
                    color: black(),
                    width: px(10.),
                },
                cx,
            );
        })
    });
    cx.update(|cx| assert_eq!(canvas.read(cx).active_tool_id(), "shape"));
    drag_shape(&canvas, cx);

    cx.update(|cx| {
        let shapes: Vec<_> = canvas.read(cx).document().shapes().collect();
        assert_eq!(shapes.len(), 1);
        match shapes[0].2 {
            CanvasShape::Rect { bounds, style } => {
                assert_eq!(bounds.origin, point(px(20.), px(20.)));
                assert_eq!(bounds.size, size(px(70.), px(40.)));
                assert_eq!(style.fill(), Some(black().opacity(super::FILL_OPACITY)));
                let stroke = style.stroke().expect("filled shapes still stroke");
                assert_eq!(stroke.width, px(10.));
            }
            other => panic!("expected a rect, got {other:?}"),
        }
    });

    // Unfilled triangles commit stroke-only, still at the chosen width.
    cx.update(|cx| {
        panel.update(cx, |panel, cx| {
            panel.filled = false;
            panel.tool = super::PaintTool::Shape(ShapeKind::Triangle);
            panel.install_tool(cx);
        })
    });
    drag_shape(&canvas, cx);

    cx.update(|cx| {
        let shapes: Vec<_> = canvas.read(cx).document().shapes().collect();
        assert_eq!(shapes.len(), 2);
        match shapes[1].2 {
            CanvasShape::Triangle { bounds, style } => {
                assert_eq!(bounds.size, size(px(70.), px(40.)));
                assert_eq!(style.fill(), None);
                assert_eq!(style.stroke().unwrap().width, px(10.));
            }
            other => panic!("expected a triangle, got {other:?}"),
        }
    });
}

#[gpui_kit::test]
fn set_brush_reinstalls_the_brush_with_the_new_config(cx: &mut TestAppContext) {
    let canvas = cx.update(super::new_paint_canvas);
    let panel = cx.update(|cx| cx.new(|cx| super::MiniPaintPanel::new(canvas.clone(), cx)));

    cx.update(|cx| {
        panel.update(cx, |panel, cx| {
            panel.set_brush(
                BrushConfig {
                    color: red(),
                    width: px(10.),
                },
                cx,
            );
        })
    });
    cx.update(|cx| assert_eq!(canvas.read(cx).active_tool_id(), "brush"));
    drag_shape(&canvas, cx);

    cx.update(|cx| {
        let shapes: Vec<_> = canvas.read(cx).document().shapes().collect();
        assert_eq!(shapes.len(), 1);
        match shapes[0].2 {
            CanvasShape::Path { style, .. } => {
                assert_eq!(style.width, px(10.));
                assert_eq!(style.color, red());
                assert_eq!(style.dash, None);
            }
            other => panic!("expected a path, got {other:?}"),
        }
    });
}
