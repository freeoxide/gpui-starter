use std::time::Duration;

use gpui_kit::component::{ActiveTheme as _, h_flex, v_flex};
use gpui_kit::{prelude::*, *};

use crate::ui::canvas::{
    BuiltinTool, Canvas, CanvasGrid, CanvasShape, CanvasState, ShapeId, StrokeStyle,
};
use crate::ui::floating::{FloatAnchor, FloatLevel, FloatingSpec, FloatingWindowId};

pub(crate) const HUD_TICK: Duration = Duration::from_millis(250);
pub(crate) const HUD_SAMPLE_CAP: usize = 120;
pub(crate) const HUD_ID_PREFIX: &str = "canvas-hud-";

const HUD_DOC_BOUNDS: Bounds<Pixels> = Bounds {
    origin: point(px(0.), px(0.)),
    size: size(px(256.), px(96.)),
};

pub(crate) fn sample_value(tick: u64) -> f32 {
    let t = tick as f32 / 4.;
    (0.55 + 0.28 * (t * 0.9).sin() + 0.14 * (t * 2.3 + 1.7).sin()).clamp(0.08, 0.92)
}

pub(crate) struct HudMetrics {
    samples: Vec<f32>,
    tick: u64,
}

impl HudMetrics {
    pub(crate) fn new(cx: &mut Context<Self>) -> Self {
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(HUD_TICK).await;
                if this.update(cx, |metrics, cx| metrics.tick(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        Self {
            samples: Vec::new(),
            tick: 0,
        }
    }

    pub(crate) fn tick(&mut self, cx: &mut Context<Self>) {
        self.tick += 1;
        self.samples.push(sample_value(self.tick));
        if self.samples.len() > HUD_SAMPLE_CAP {
            self.samples.drain(..1);
        }
        cx.notify();
    }

    pub(crate) fn samples(&self) -> &[f32] {
        &self.samples
    }

    pub(crate) fn current(&self) -> f32 {
        self.samples.last().copied().unwrap_or(0.)
    }

    pub(crate) fn tick_count(&self) -> u64 {
        self.tick
    }
}

pub(crate) fn sparkline_points(samples: &[f32], bounds: Bounds<Pixels>) -> Vec<Point<Pixels>> {
    if samples.is_empty() {
        return Vec::new();
    }
    let width = bounds.size.width.as_f32();
    let height = bounds.size.height.as_f32();
    let denom = (samples.len() - 1).max(1) as f32;
    samples
        .iter()
        .enumerate()
        .map(|(index, value)| {
            point(
                bounds.origin.x + px(index as f32 / denom * width),
                bounds.origin.y + px((1. - value) * height),
            )
        })
        .collect()
}

pub(crate) struct HudPanel {
    metrics: Entity<HudMetrics>,
    canvas: Entity<CanvasState>,
    path_id: Option<ShapeId>,
    _subscriptions: Vec<Subscription>,
}

impl HudPanel {
    pub(crate) fn new_view(
        metrics: Entity<HudMetrics>,
        _window: &mut Window,
        cx: &mut App,
    ) -> AnyView {
        cx.new(|cx| Self::new(metrics, cx)).into()
    }

    fn new(metrics: Entity<HudMetrics>, cx: &mut Context<Self>) -> Self {
        let canvas = cx.new(|cx| {
            let mut state = CanvasState::new();
            state.set_grid(
                CanvasGrid {
                    visible: false,
                    spacing: px(16.),
                },
                cx,
            );
            state.set_builtin_tool(BuiltinTool::Pan, cx);
            state.add_layer(cx);
            state
        });
        let subscription = cx.observe(&metrics, |this, _, cx| this.refresh(cx));
        Self {
            metrics,
            canvas,
            path_id: None,
            _subscriptions: vec![subscription],
        }
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        let samples = self.metrics.read(cx).samples().to_vec();
        let stroke = StrokeStyle {
            width: px(2.),
            color: cx.theme().primary,
            dash: None,
        };
        let old = self.path_id;
        self.canvas.update(cx, |state, cx| {
            if let Some(old) = old {
                state.remove_shape(old, cx);
            }
            let layer = state
                .document()
                .top_layer()
                .unwrap_or_else(|| state.add_layer(cx));
            self.path_id = state.add_shape(
                layer,
                CanvasShape::path(sparkline_points(&samples, HUD_DOC_BOUNDS), stroke),
                cx,
            );
        });
        cx.notify();
    }
}

impl Render for HudPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .child(
                h_flex()
                    .justify_between()
                    .px_2()
                    .py_1()
                    .border_b_1()
                    .border_color(cx.theme().border)
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child("Load"),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .child(format!("{:.2}", self.metrics.read(cx).current())),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .child(Canvas::new("hud-spark", self.canvas.clone())),
            )
    }
}

pub(crate) fn hud_spec(index: u32, anchor: FloatAnchor) -> FloatingSpec {
    FloatingSpec::new(
        FloatingWindowId(format!("{HUD_ID_PREFIX}{index}").into()),
        size(px(260.), px(150.)),
    )
    .title("HUD")
    .level(FloatLevel::AlwaysOnTop)
    .anchor(anchor)
    .focus(false)
    .resizable(false)
    .movable(true)
    .min_size(size(px(200.), px(110.)))
}

#[cfg(test)]
#[path = "hud.test.rs"]
mod hud_test;
