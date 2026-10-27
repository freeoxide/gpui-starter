//! Floating-canvas gallery section: canvas mini-apps in floating windows.
//! Removal = delete this directory and revert its single seam touch: the
//! `mod floating_canvas;` and `floating_canvas::register` lines in
//! src/gallery/sections/overlays/mod.rs.

mod hud;
mod paint;
mod shader;

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Sizable as _,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};
use gpui_kit::{prelude::*, *};

use crate::gallery::registry::GallerySection;
use crate::ui::canvas::{Canvas, CanvasState};
use crate::ui::floating::{
    FloatAnchor, FloatLevel, FloatingHandle, FloatingSpec, FloatingWindowId, Placement9,
    close_floating, floating_snapshot, respawn_floating, spawn_floating,
};

use super::demo::section;
use hud::{HUD_ID_PREFIX, HudMetrics, HudPanel, hud_spec};
use paint::{MINI_PAINT_ID, MiniPaintPanel, new_paint_canvas};
use shader::{SHADER_ID, ShaderPanel};

pub(crate) struct FloatingCanvasSection {
    paint_canvas: Entity<CanvasState>,
    metrics: Entity<HudMetrics>,
    _subscriptions: Vec<Subscription>,
}

impl FloatingCanvasSection {
    pub(crate) fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let paint_canvas = new_paint_canvas(cx);
        let metrics = cx.new(HudMetrics::new);
        let subscriptions = vec![
            cx.observe(&paint_canvas, |_, _, cx| cx.notify()),
            cx.observe(&metrics, |_, _, cx| cx.notify()),
        ];
        Self {
            paint_canvas,
            metrics,
            _subscriptions: subscriptions,
        }
    }

    pub(crate) fn detach_paint(&mut self, cx: &mut Context<Self>) {
        let opened = if is_open(MINI_PAINT_ID, cx) {
            respawn_floating(&FloatingWindowId(MINI_PAINT_ID.into()), cx)
        } else {
            let spec = paint_spec(cx);
            let canvas = self.paint_canvas.clone();
            spawn_floating(
                spec,
                move |window, cx| MiniPaintPanel::new_view(canvas.clone(), window, cx),
                cx,
            )
            .is_ok()
        };
        if !opened {
            tracing::warn!(target: "gpui_starter::gallery", "mini-paint float failed to open");
        }
        cx.notify();
    }

    pub(crate) fn spawn_hud(&mut self, cx: &mut Context<Self>) {
        let index = (0u32..)
            .find(|index| !is_open(&format!("{HUD_ID_PREFIX}{index}"), cx))
            .unwrap_or(0);
        let cascade = px(-24. * index as f32);
        let spec = hud_spec(
            index,
            window_anchor(cx, Placement9::BottomRight, point(cascade, cascade)),
        );
        let metrics = self.metrics.clone();
        let opened = spawn_floating(
            spec,
            move |window, cx| HudPanel::new_view(metrics.clone(), window, cx),
            cx,
        )
        .is_ok();
        if !opened {
            tracing::warn!(target: "gpui_starter::gallery", "hud float failed to open");
        }
        cx.notify();
    }

    pub(crate) fn open_shader(&mut self, cx: &mut Context<Self>) {
        let opened = if is_open(SHADER_ID, cx) {
            respawn_floating(&FloatingWindowId(SHADER_ID.into()), cx)
        } else {
            let spec = shader_spec(cx);
            spawn_floating(spec, ShaderPanel::new_view, cx).is_ok()
        };
        if !opened {
            tracing::warn!(target: "gpui_starter::gallery", "shader float failed to open");
        }
        cx.notify();
    }

    pub(crate) fn close_ours(&mut self, cx: &mut Context<Self>) -> usize {
        let handles: Vec<FloatingHandle> = floating_snapshot(cx)
            .into_iter()
            .filter(|entry| is_ours(&entry.id.0))
            .map(|entry| entry.handle)
            .collect();
        let closed = handles
            .iter()
            .filter(|handle| close_floating(handle, cx))
            .count();
        cx.notify();
        closed
    }
}

fn is_ours(id: &str) -> bool {
    id == MINI_PAINT_ID || id == SHADER_ID || id.starts_with(HUD_ID_PREFIX)
}

fn is_open(id: &str, cx: &App) -> bool {
    floating_snapshot(cx).iter().any(|entry| entry.id.0 == id)
}

fn window_anchor(cx: &App, placement: Placement9, offset: Point<Pixels>) -> FloatAnchor {
    match crate::app::window::root_window(cx) {
        Some(window) => FloatAnchor::Window {
            window,
            placement,
            offset,
        },
        None => FloatAnchor::Screen {
            display: None,
            placement,
            offset,
        },
    }
}

fn paint_spec(cx: &App) -> FloatingSpec {
    FloatingSpec::new(
        FloatingWindowId(MINI_PAINT_ID.into()),
        size(px(480.), px(360.)),
    )
    .title("Mini Paint")
    .level(FloatLevel::Floating)
    .anchor(window_anchor(
        cx,
        Placement9::TopRight,
        point(px(16.), px(16.)),
    ))
    .focus(false)
    .resizable(true)
    .movable(true)
    .min_size(size(px(320.), px(240.)))
}

fn shader_spec(cx: &App) -> FloatingSpec {
    FloatingSpec::new(FloatingWindowId(SHADER_ID.into()), size(px(400.), px(300.)))
        .title("Shader Toy")
        .level(FloatLevel::Floating)
        .anchor(window_anchor(
            cx,
            Placement9::Right,
            point(px(24.), px(24.)),
        ))
        .focus(false)
        .resizable(true)
        .min_size(size(px(280.), px(200.)))
}

impl Render for FloatingCanvasSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let strokes = self.paint_canvas.read(cx).document().shapes().count();
        let snapshot = floating_snapshot(cx);
        let hud_count = snapshot
            .iter()
            .filter(|entry| entry.id.0.starts_with(HUD_ID_PREFIX))
            .count();
        let ours = snapshot.iter().filter(|entry| is_ours(&entry.id.0)).count();
        v_flex()
            .w_full()
            .gap_4()
            .p_4()
            .child(
                section("fc-mini-paint", "Mini paint")
                    .description("One canvas entity renders here and in the detached float.")
                    .w(rems(32.5))
                    .child(
                        div()
                            .relative()
                            .w_full()
                            .h(rems(16.))
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(cx.theme().radius)
                            .overflow_hidden()
                            .child(Canvas::new(
                                "mini-paint-embedded",
                                self.paint_canvas.clone(),
                            )),
                    )
                    .child(
                        Button::new("paint-detach")
                            .label("Detach…")
                            .on_click(cx.listener(|this, _, _, cx| this.detach_paint(cx))),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{strokes} strokes")),
                    ),
            )
            .child(
                section("fc-hud", "Live sparkline HUD")
                    .description("One ticking model entity drives every open HUD window.")
                    .w(rems(32.5))
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!(
                                "load {:.2} · tick {} · {} HUD windows open",
                                self.metrics.read(cx).current(),
                                self.metrics.read(cx).tick_count(),
                                hud_count,
                            )),
                    )
                    .child(
                        Button::new("hud-add")
                            .label("Add HUD…")
                            .on_click(cx.listener(|this, _, _, cx| this.spawn_hud(cx))),
                    ),
            )
            .child(
                section("fc-shader", "Shader toy")
                    .description("A CPU-painted procedural field that pauses on demand.")
                    .w(rems(32.5))
                    .child(
                        Button::new("shader-open")
                            .label("Open Shader Panel…")
                            .on_click(cx.listener(|this, _, _, cx| this.open_shader(cx))),
                    ),
            )
            .child(
                h_flex().w(rems(32.5)).justify_end().child(
                    Button::new("fc-close-all")
                        .ghost()
                        .small()
                        .label("Close Floating Canvases")
                        .disabled(ours == 0)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.close_ours(cx);
                        })),
                ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "floating-canvas",
        "Floating Canvas",
        "Canvas mini-apps in floating utility windows: shared-state mini paint, live sparkline HUDs, a shader toy.",
        FloatingCanvasSection::view(window, cx),
    ));
}

#[cfg(test)]
#[path = "mod.test.rs"]
mod mod_test;
