use gpui_kit::component::{
    ActiveTheme as _, Sizable as _,
    button::{Button, ButtonVariants as _, Toggle, ToggleGroup},
    h_flex, v_flex,
};
use gpui_kit::{prelude::*, *};

use crate::ui::canvas::{BrushConfig, BuiltinTool, Canvas, CanvasState};

pub(crate) const MINI_PAINT_ID: &str = "canvas-mini-paint";

const BRUSH_WIDTHS: [Pixels; 3] = [px(2.), px(4.), px(10.)];

fn default_brush() -> BrushConfig {
    BrushConfig {
        color: black(),
        width: BRUSH_WIDTHS[1],
    }
}

// Palette values are document data (the paint is the content), not chrome;
// same data exemption as the fps scene palette (charts/fps.rs).
fn palette(cx: &App) -> [Hsla; 4] {
    [
        black(),
        cx.theme().primary,
        cx.theme().danger,
        cx.theme().warning,
    ]
}

pub(crate) fn new_paint_canvas(cx: &mut App) -> Entity<CanvasState> {
    cx.new(|cx| {
        let mut state = CanvasState::new();
        state.add_layer(cx);
        state.set_builtin_tool(BuiltinTool::Brush(default_brush()), cx);
        state
    })
}

pub(crate) struct MiniPaintPanel {
    canvas: Entity<CanvasState>,
    brush: BrushConfig,
    _subscriptions: Vec<Subscription>,
}

impl MiniPaintPanel {
    pub(crate) fn new_view(
        canvas: Entity<CanvasState>,
        _window: &mut Window,
        cx: &mut App,
    ) -> AnyView {
        cx.new(|cx| Self::new(canvas, cx)).into()
    }

    fn new(canvas: Entity<CanvasState>, cx: &mut Context<Self>) -> Self {
        let subscription = cx.observe(&canvas, |_, _, cx| cx.notify());
        Self {
            canvas,
            brush: default_brush(),
            _subscriptions: vec![subscription],
        }
    }

    pub(crate) fn set_brush(&mut self, config: BrushConfig, cx: &mut Context<Self>) {
        self.brush = config;
        self.canvas.update(cx, |state, cx| {
            state.set_builtin_tool(BuiltinTool::Brush(config), cx);
        });
        cx.notify();
    }

    fn toolbar(&self, cx: &mut Context<Self>) -> Div {
        let colors = palette(cx);
        h_flex()
            .gap_2()
            .p_2()
            .flex_wrap()
            .border_b_1()
            .border_color(cx.theme().border)
            .child(
                ToggleGroup::new("paint-color")
                    .small()
                    .segmented()
                    .child(
                        Toggle::new("paint-color-ink")
                            .label("Ink")
                            .checked(self.brush.color == colors[0]),
                    )
                    .child(
                        Toggle::new("paint-color-accent")
                            .label("Accent")
                            .checked(self.brush.color == colors[1]),
                    )
                    .child(
                        Toggle::new("paint-color-danger")
                            .label("Danger")
                            .checked(self.brush.color == colors[2]),
                    )
                    .child(
                        Toggle::new("paint-color-warn")
                            .label("Warn")
                            .checked(self.brush.color == colors[3]),
                    )
                    .on_click(cx.listener(|this, values: &Vec<bool>, _, cx| {
                        if let Some(ix) = values.iter().position(|b| *b)
                            && let Some(color) = palette(cx).get(ix)
                        {
                            this.set_brush(
                                BrushConfig {
                                    color: *color,
                                    width: this.brush.width,
                                },
                                cx,
                            );
                        }
                    })),
            )
            .child(
                ToggleGroup::new("paint-width")
                    .small()
                    .segmented()
                    .child(
                        Toggle::new("paint-width-fine")
                            .label("Fine")
                            .checked(self.brush.width == BRUSH_WIDTHS[0]),
                    )
                    .child(
                        Toggle::new("paint-width-medium")
                            .label("Medium")
                            .checked(self.brush.width == BRUSH_WIDTHS[1]),
                    )
                    .child(
                        Toggle::new("paint-width-broad")
                            .label("Broad")
                            .checked(self.brush.width == BRUSH_WIDTHS[2]),
                    )
                    .on_click(cx.listener(|this, values: &Vec<bool>, _, cx| {
                        if let Some(ix) = values.iter().position(|b| *b)
                            && let Some(width) = BRUSH_WIDTHS.get(ix)
                        {
                            this.set_brush(
                                BrushConfig {
                                    color: this.brush.color,
                                    width: *width,
                                },
                                cx,
                            );
                        }
                    })),
            )
            .child(
                Button::new("paint-clear")
                    .ghost()
                    .small()
                    .label("Clear")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.canvas.update(cx, |state, cx| state.clear(cx));
                    })),
            )
    }
}

impl Render for MiniPaintPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .min_h_0()
            .child(self.toolbar(cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .child(Canvas::new("mini-paint-float", self.canvas.clone())),
            )
    }
}

#[cfg(test)]
#[path = "paint.test.rs"]
mod paint_test;
