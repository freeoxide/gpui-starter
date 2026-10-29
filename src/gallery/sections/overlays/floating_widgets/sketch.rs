use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Sizable as _,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};
use gpui_kit::{prelude::*, *};

use crate::ui::canvas::{BrushConfig, BuiltinTool, Canvas, CanvasGrid, CanvasState};
use crate::ui::floating::{
    FloatAnchor, FloatLevel, FloatingHandle, FloatingSpec, FloatingWindowId, Placement9,
    spawn_screen_floating,
};

pub(crate) const SKETCH_ID: &str = "chromeless-sketch";

pub(crate) fn new_sketch_canvas(cx: &mut App) -> Entity<CanvasState> {
    cx.new(|cx| {
        // Ink is document data (the paint is the content); resolved from the
        // theme once so it stays readable on light and dark surfaces.
        let mut state = CanvasState::new();
        state.set_grid(
            CanvasGrid {
                visible: false,
                spacing: px(16.),
            },
            cx,
        );
        state.add_layer(cx);
        state.set_builtin_tool(
            BuiltinTool::Brush(BrushConfig {
                color: cx.theme().foreground,
                width: px(3.),
            }),
            cx,
        );
        state
    })
}

pub(crate) struct SketchPad {
    canvas: Entity<CanvasState>,
    _subscriptions: Vec<Subscription>,
}

impl SketchPad {
    pub(crate) fn new_view(_: &mut Window, cx: &mut App) -> AnyView {
        let canvas = new_sketch_canvas(cx);
        cx.new(|cx| Self::new(canvas, cx)).into()
    }

    fn new(canvas: Entity<CanvasState>, cx: &mut Context<Self>) -> Self {
        let subscription = cx.observe(&canvas, |_, _, cx| cx.notify());
        Self {
            canvas,
            _subscriptions: vec![subscription],
        }
    }

    pub(crate) fn stroke_count(&self, cx: &App) -> usize {
        self.canvas.read(cx).document().shapes().count()
    }

    pub(crate) fn clear(&mut self, cx: &mut Context<Self>) {
        self.canvas.update(cx, |state, cx| state.clear(cx));
    }
}

impl Render for SketchPad {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let strokes = self.stroke_count(cx);
        v_flex()
            .size_full()
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .px_2()
                    .py_1()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{strokes} strokes")),
                    )
                    .child(
                        Button::new("sketch-clear")
                            .ghost()
                            .small()
                            .label("Clear")
                            .disabled(strokes == 0)
                            .on_click(cx.listener(|this, _, _, cx| this.clear(cx))),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .child(Canvas::new("sketch-canvas", self.canvas.clone())),
            )
    }
}

pub(crate) fn sketch_spec() -> FloatingSpec {
    FloatingSpec::new(FloatingWindowId(SKETCH_ID.into()), size(px(360.), px(260.)))
        .title("Sketch pad")
        .level(FloatLevel::AlwaysOnTop)
        .anchor(FloatAnchor::Screen {
            display: None,
            placement: Placement9::Right,
            offset: point(px(-24.), px(0.)),
        })
        .chromeless(true)
        .focus(false)
        .movable(true)
        .resizable(true)
        .min_size(size(px(280.), px(200.)))
}

/// Open the sketch pad as a screen float; idempotent per id.
pub fn spawn_sketch(cx: &mut App) -> anyhow::Result<FloatingHandle> {
    if let Some(handle) = super::entry_handle(SKETCH_ID, cx) {
        return Ok(handle);
    }
    spawn_screen_floating(sketch_spec(), SketchPad::new_view, cx)
}

#[cfg(test)]
#[path = "sketch.test.rs"]
mod sketch_test;
