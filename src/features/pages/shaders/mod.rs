//! Removable shader-canvas page. `ShaderCanvasPage` and
//! `CAPABILITY_SHADERS` are the only public surface; the WGSL demos, state,
//! runtime, and tool stay private to this module. Removal = delete this
//! directory and revert its ten seam touches: the wgpu/naga lines in
//! Cargo.toml plus the regenerated Cargo.lock (only if plan 11's bridge is
//! not keeping them), the `mod shaders;` line and the
//! `pub use shaders::{ShaderCanvasPage, CAPABILITY_SHADERS};` re-export in
//! features/pages/mod.rs, the Page::ShaderCanvas arms and all() entry in
//! shell/sidebar.rs, the VALID_HOSTS entry in shell/route.rs, the AppRoot
//! wiring in shell/root/app_root/state.rs, the page count in
//! tests/e2e_navigation.rs, the shader_* and nav_shader_canvas keys in both
//! locale files, and the canvas_shaders capability lines in app/init.rs.

mod demos;
#[cfg(not(target_family = "wasm"))]
mod runtime;
mod state;
mod tool;

#[cfg(all(test, not(target_family = "wasm")))]
#[path = "mod.test.rs"]
mod shaders_test;

use demos::DemoId;
use state::ShaderState;

use gpui_kit::component::slider::{SliderEvent, SliderState, SliderValue};
use gpui_kit::component::{ActiveTheme as _, v_flex};
#[cfg(not(target_family = "wasm"))]
use gpui_kit::component::{
    Selectable as _,
    button::{Button, ButtonGroup},
    h_flex,
    slider::Slider,
};
use gpui_kit::{prelude::*, *};

use crate::accessibility::A11yExt as _;
use crate::ui::canvas::CanvasState;
#[cfg(not(target_family = "wasm"))]
use crate::ui::canvas::{BuiltinTool, Canvas, CanvasGrid, CanvasShape, ShapeId};

#[cfg(not(target_family = "wasm"))]
use crate::features::wgpu::WgpuEngine;
#[cfg(not(target_family = "wasm"))]
use crate::platform::clock::{self, Instant};
#[cfg(not(target_family = "wasm"))]
use demos::{DEMOS, DemoSpec};
#[cfg(not(target_family = "wasm"))]
use std::sync::Arc;

/// Capability-registry name registered at startup ("canvas_shaders").
pub const CAPABILITY_SHADERS: &str = "canvas_shaders";

#[cfg(not(target_family = "wasm"))]
fn button_id(id: DemoId) -> &'static str {
    match id {
        DemoId::Gradient => "shader-demo-gradient",
        DemoId::Raymarch => "shader-demo-raymarch",
        DemoId::Warp => "shader-demo-warp",
        DemoId::Ripple => "shader-demo-ripple",
    }
}

#[cfg(not(target_family = "wasm"))]
fn label_key(id: DemoId) -> &'static str {
    match id {
        DemoId::Gradient => "shader_demo_gradient",
        DemoId::Raymarch => "shader_demo_raymarch",
        DemoId::Warp => "shader_demo_warp",
        DemoId::Ripple => "shader_demo_ripple",
    }
}

#[cfg(not(target_family = "wasm"))]
fn param_key(id: DemoId) -> &'static str {
    match id {
        DemoId::Gradient => "shader_param_gradient",
        DemoId::Raymarch => "shader_param_raymarch",
        DemoId::Warp => "shader_param_warp",
        DemoId::Ripple => "shader_param_ripple",
    }
}

#[cfg(not(target_family = "wasm"))]
fn doc_bounds(spec: &DemoSpec) -> Bounds<Pixels> {
    Bounds {
        origin: point(px(0.), px(0.)),
        size: size(px(spec.width as f32), px(spec.height as f32)),
    }
}

#[cfg(not(target_family = "wasm"))]
fn blank_image() -> Arc<RenderImage> {
    crate::features::wgpu::bgra_to_render_image(1, 1, vec![0, 0, 0, 0])
}

pub struct ShaderCanvasPage {
    canvas: Entity<CanvasState>,
    shader: Entity<ShaderState>,
    slider: Entity<SliderState>,
    #[cfg(not(target_family = "wasm"))]
    runtime: Option<Arc<runtime::ShaderRuntime>>,
    #[cfg(not(target_family = "wasm"))]
    image_shape: Option<ShapeId>,
    _subscriptions: Vec<Subscription>,
}

impl ShaderCanvasPage {
    /// Constructs the page, mounts its entities, and on native starts the
    /// wgpu engine init task. Callers never block.
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let canvas = cx.new(|_| CanvasState::new());
        let shader = cx.new(|cx| ShaderState::new(DemoId::Gradient, cx));
        let slider_spec = demos::demo(DemoId::Gradient).slider;
        let slider = cx.new(|_| {
            SliderState::new()
                .min(slider_spec.min)
                .max(slider_spec.max)
                .default_value(slider_spec.default)
        });
        let mut page = Self {
            canvas,
            shader,
            slider,
            #[cfg(not(target_family = "wasm"))]
            runtime: None,
            #[cfg(not(target_family = "wasm"))]
            image_shape: None,
            _subscriptions: Vec::new(),
        };
        page.wire_subscriptions(cx);
        #[cfg(not(target_family = "wasm"))]
        page.start_runtime(cx);
        page
    }

    fn wire_subscriptions(&mut self, cx: &mut Context<Self>) {
        let canvas = self.canvas.clone();
        let shader = self.shader.clone();
        let slider = self.slider.clone();
        let mut subscriptions = vec![
            cx.observe(&canvas, |_, _, cx| cx.notify()),
            cx.observe(&shader, |_, _, cx| cx.notify()),
        ];
        subscriptions.push(cx.subscribe(&slider, |this, _, event, cx| {
            if let SliderEvent::Change(SliderValue::Single(value)) = event {
                this.shader.update(cx, |s, cx| s.set_slider(*value, cx));
            }
        }));
        self._subscriptions = subscriptions;
    }

    #[cfg(not(target_family = "wasm"))]
    fn start_runtime(&mut self, cx: &mut Context<Self>) {
        self.runtime = None;
        self.shader.update(cx, |s, cx| s.clear_error(cx));
        cx.spawn(async move |this, cx| {
            let runtime = cx
                .background_executor()
                .spawn(async move {
                    let engine = Arc::new(WgpuEngine::new().await?);
                    anyhow::Ok(Arc::new(runtime::ShaderRuntime::new(engine)))
                })
                .await;
            let _ = this.update(cx, |page, cx| match runtime {
                Ok(runtime) => {
                    page.runtime = Some(runtime);
                    crate::capabilities::set(
                        CAPABILITY_SHADERS,
                        crate::capabilities::CapabilityStatus::supported_enabled(),
                        cx,
                    );
                    page.install_demo(cx);
                    cx.notify();
                }
                Err(err) => {
                    crate::capabilities::set(
                        CAPABILITY_SHADERS,
                        crate::capabilities::CapabilityStatus::error(err.to_string()),
                        cx,
                    );
                    page.shader
                        .update(cx, |s, cx| s.frame_failed(err.to_string(), cx));
                    cx.notify();
                }
            });
        })
        .detach();
    }

    #[cfg(not(target_family = "wasm"))]
    fn switch_demo(&mut self, ix: usize, cx: &mut Context<Self>) {
        let Some(spec) = DEMOS.get(ix) else {
            return;
        };
        let id = spec.id;
        self.shader.update(cx, |s, cx| s.set_demo(id, cx));
        self.install_demo(cx);
    }

    #[cfg(not(target_family = "wasm"))]
    fn install_demo(&mut self, cx: &mut Context<Self>) {
        let id = self.shader.read(cx).demo();
        let spec = demos::demo(id);
        let shader = self.shader.clone();
        self.image_shape = self.canvas.update(cx, |s, cx| {
            if id == DemoId::Ripple {
                s.set_tool(
                    Box::new(tool::RippleTool::new(
                        shader.downgrade(),
                        spec.width as f32,
                        spec.height as f32,
                    )),
                    cx,
                );
            } else {
                s.set_builtin_tool(BuiltinTool::Pan, cx);
            }
            s.set_grid(
                CanvasGrid {
                    visible: false,
                    spacing: px(16.),
                },
                cx,
            );
            let shape = s.edit_document(cx, |d| {
                d.clear();
                let layer = d.add_layer();
                d.add_shape(layer, CanvasShape::image(doc_bounds(spec), blank_image()))
            });
            s.reset_view(cx);
            shape
        });
        let slider_spec = spec.slider;
        self.slider = cx.new(|_| {
            SliderState::new()
                .min(slider_spec.min)
                .max(slider_spec.max)
                .default_value(slider_spec.default)
        });
        self.wire_subscriptions(cx);
    }

    fn heading() -> Stateful<Div> {
        div()
            .id("shader-title")
            .a11y(Role::Heading, crate::i18n::localize("shader_title"))
            .aria_level(1)
            .text_xl()
            .font_weight(FontWeight::BOLD)
            .child(crate::i18n::localize("shader_title"))
    }

    #[cfg(target_family = "wasm")]
    fn render_wasm(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .min_h_full()
            .p_6()
            .gap_4()
            .child(Self::heading())
            .child(
                div()
                    .border_1()
                    .border_color(cx.theme().border)
                    .rounded_lg()
                    .p_4()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(crate::i18n::localize("shader_unavailable_wasm")),
                    ),
            )
    }

    #[cfg(not(target_family = "wasm"))]
    fn render_native(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let (playing, demo, slider_value, fps, error, frames) = {
            let shader = self.shader.read(cx);
            (
                shader.is_playing(),
                shader.demo(),
                shader.slider(),
                shader.fps(),
                shader.runtime_error().map(str::to_string),
                shader.frames_rendered(),
            )
        };

        let play_label = crate::i18n::localize(if playing {
            "shader_pause"
        } else {
            "shader_play"
        });
        let header = h_flex()
            .w_full()
            .flex_wrap()
            .justify_between()
            .items_center()
            .child(Self::heading())
            .child(
                h_flex().gap_2().flex_shrink_0().child(
                    Button::new("shader-toggle-play")
                        .outline()
                        .label(play_label)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.shader.update(cx, |s, cx| s.toggle_playing(cx));
                        })),
                ),
            );

        let demo_group = ButtonGroup::new("shader-demos")
            .children(DEMOS.iter().map(|spec| {
                Button::new(SharedString::from(button_id(spec.id)))
                    .label(crate::i18n::localize(label_key(spec.id)))
                    .selected(demo == spec.id)
            }))
            .on_click(cx.listener(|this, clicks: &Vec<usize>, _, cx| {
                if let Some(&ix) = clicks.first().filter(|ix| **ix < DEMOS.len()) {
                    this.switch_demo(ix, cx);
                }
            }));

        let param_label = crate::i18n::localize(param_key(demo));
        let slider_row = h_flex()
            .w_full()
            .gap_3()
            .items_center()
            .child(div().text_sm().child(param_label.clone()))
            .child(
                Slider::new(&self.slider)
                    .w(rems(12.5))
                    .accessibility_label(param_label),
            )
            .child(div().text_sm().w_10().child(format!("{slider_value:.2}")));

        let viewport_border = |div: Div| {
            div.relative()
                .flex_1()
                .min_h_0()
                .rounded(cx.theme().radius)
                .overflow_hidden()
                .border_1()
                .border_color(cx.theme().border)
        };

        let overlay_text = if frames == 0 {
            crate::i18n::localize("shader_waiting")
        } else {
            crate::i18n::localize_with_args("shader_fps", &[("fps", &fps.to_string())])
        };
        let fps_overlay = div()
            .absolute()
            .top(rems(0.5))
            .right(rems(0.5))
            .px_2()
            .py_1()
            .rounded_md()
            .bg(cx.theme().popover)
            .border_1()
            .border_color(cx.theme().border)
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(overlay_text);

        let content = match error.as_deref() {
            Some(err) => viewport_border(div())
                .child(
                    v_flex()
                        .size_full()
                        .items_center()
                        .justify_center()
                        .gap_3()
                        .p_4()
                        .child(
                            div()
                                .text_sm()
                                .font_weight(FontWeight::SEMIBOLD)
                                .child(crate::i18n::localize("shader_error_title")),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().danger)
                                .child(err.to_string()),
                        )
                        .child(
                            Button::new("shader-retry")
                                .outline()
                                .label(crate::i18n::localize("shader_retry"))
                                .on_click(cx.listener(|this, _, _, cx| this.start_runtime(cx))),
                        ),
                )
                // The ticker stays mounted on error: it keeps draining retired
                // frames, and its kick gate refuses frames while error is set.
                .child(self.render_ticker(cx)),
            None => viewport_border(div())
                .child(Canvas::new("shader-canvas", self.canvas.clone()))
                .child(fps_overlay)
                .child(self.render_ticker(cx)),
        };

        v_flex()
            .min_h_full()
            .p_6()
            .gap_4()
            .child(header)
            .child(demo_group)
            .child(slider_row)
            .child(content)
    }

    #[cfg(not(target_family = "wasm"))]
    fn render_ticker(&self, cx: &Context<Self>) -> impl IntoElement {
        let shader = self.shader.clone();
        let canvas_entity = self.canvas.clone();
        let page = cx.weak_entity();
        let runtime = self.runtime.clone();
        let spec = demos::demo(shader.read(cx).demo());
        canvas(
            move |_, _, _| (),
            move |bounds, _, window, cx| {
                let (stale, fit) = shader.update(cx, |s, _| (s.take_stale_frames(), s.take_fit()));
                for image in stale {
                    let _ = window.drop_image(image);
                }
                if fit && bounds.size.width > px(0.) && bounds.size.height > px(0.) {
                    let _ = canvas_entity
                        .update(cx, |s, cx| s.zoom_to_fit(doc_bounds(spec), bounds.size, cx));
                }

                if runtime.is_none() {
                    return;
                }
                let eligible = {
                    let snapshot = shader.read(cx);
                    snapshot.is_playing()
                        && !snapshot.is_frame_in_flight()
                        && snapshot.runtime_error().is_none()
                };
                if !eligible {
                    return;
                }

                window.on_next_frame(move |_, cx| {
                    let Some(page) = page.upgrade() else {
                        return;
                    };
                    let page_view = page.read(cx);
                    let (Some(runtime), Some(shape_id)) =
                        (page_view.runtime.clone(), page_view.image_shape)
                    else {
                        return;
                    };
                    let still_eligible = {
                        let snapshot = shader.read(cx);
                        snapshot.is_playing()
                            && !snapshot.is_frame_in_flight()
                            && snapshot.runtime_error().is_none()
                    };
                    if !still_eligible {
                        return;
                    }
                    let now = clock::Instant::now();
                    shader.update(cx, |s, cx| s.mark_frame_started(now, cx));
                    cx.spawn(async move |cx| {
                        frame_task(shader, canvas_entity, runtime, shape_id, spec, cx).await;
                    })
                    .detach();
                });
            },
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full()
    }
}

#[cfg(not(target_family = "wasm"))]
async fn frame_task(
    shader: Entity<ShaderState>,
    canvas: Entity<CanvasState>,
    runtime: Arc<runtime::ShaderRuntime>,
    shape_id: ShapeId,
    spec: &'static DemoSpec,
    cx: &mut AsyncApp,
) {
    let wait = shader.read_with(cx, |s, _| s.pacing_wait(Instant::now()));
    if !wait.is_zero() {
        cx.background_executor().timer(wait).await;
    }
    if !shader.read_with(cx, |s, _| s.is_playing()) {
        return;
    }
    let params = shader.read_with(cx, |s, _| s.params_snapshot(spec));
    let frame = cx
        .background_executor()
        .spawn(async move { runtime.render_frame(spec, params) })
        .await;

    match frame {
        Ok(image) => {
            // A pause after the render drops the frame before it can paint,
            // so pausing freezes the canvas on the last landed image.
            if !shader.read_with(cx, |s, _| s.is_playing()) {
                return;
            }
            let _ = canvas.update(cx, |s, cx| {
                s.edit_document(cx, |d| {
                    d.update_shape(shape_id, |shape| {
                        if let CanvasShape::Image { image: slot, .. } = shape {
                            *slot = image.clone();
                        }
                    })
                })
            });
            shader.update(cx, |s, cx| s.frame_done(Instant::now(), image, cx));
        }
        Err(err) => {
            let _ = cx.update(|cx| {
                crate::capabilities::set(
                    CAPABILITY_SHADERS,
                    crate::capabilities::CapabilityStatus::degraded(err.to_string()),
                    cx,
                );
            });
            shader.update(cx, |s, cx| s.frame_failed(err.to_string(), cx));
        }
    }
}

impl Render for ShaderCanvasPage {
    #[cfg_attr(target_family = "wasm", allow(unused_variables))]
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        #[cfg(target_family = "wasm")]
        {
            self.render_wasm(cx)
        }
        #[cfg(not(target_family = "wasm"))]
        {
            self.render_native(window, cx)
        }
    }
}
