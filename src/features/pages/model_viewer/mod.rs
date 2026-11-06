//! Removable 3D model-viewer page. `ModelViewerPage` is the only public
//! surface; the OBJ parser, viewer state, and native renderer stay private
//! to this module. Removal = delete this directory and revert its eight
//! seam touches: the `mod model_viewer;` line and the
//! `pub use model_viewer::ModelViewerPage;` re-export in
//! features/pages/mod.rs, the Page::ModelViewer arms and all() entry in
//! shell/page.rs, the VALID_HOSTS entry in shell/route.rs, the AppRoot
//! wiring in shell/root/app_root/state.rs, the page count in
//! tests/e2e_navigation.rs, and the model_viewer_* and nav_model_viewer
//! keys in both locale files. The wgpu bridge it mounts is the shaders
//! page's dependency, not this feature's.

mod obj;
#[cfg(not(target_family = "wasm"))]
mod renderer;
mod state;

use state::ViewerState;

use gpui_kit::component::{ActiveTheme as _, v_flex};
#[cfg(not(target_family = "wasm"))]
use gpui_kit::component::{Disableable as _, button::Button, h_flex};
use gpui_kit::{prelude::*, *};

use crate::accessibility::A11yExt as _;
#[cfg(not(target_family = "wasm"))]
use crate::features::wgpu::WgpuEngine;
#[cfg(not(target_family = "wasm"))]
use crate::platform::clock::{Duration, Instant};
#[cfg(not(target_family = "wasm"))]
use std::sync::{Arc, Mutex};

/// Backstop deadline for a lost init: the fenced engine init and renderer
/// build bound the attempt; this watchdog catches a result that never lands.
#[cfg(not(target_family = "wasm"))]
const INIT_TIMEOUT: Duration = Duration::from_secs(15);
/// Screen-space pixels per radian of orbit.
#[cfg(not(target_family = "wasm"))]
const ORBIT_SENSITIVITY: f32 = 0.01;

#[cfg(not(target_family = "wasm"))]
const LOG: &str = "gpui_starter::model_viewer";

/// The page's whole native-only state behind one wasm fence: the engine and
/// renderer, their init tasks, and the orbit-drag anchor.
#[cfg(not(target_family = "wasm"))]
#[derive(Default)]
struct NativeRuntime {
    engine: Option<Arc<WgpuEngine>>,
    // The engine Arc this clone fences frames against lives in each task.
    renderer: Option<Arc<Mutex<renderer::ModelRenderer>>>,
    init_started: bool,
    init_task: Option<Task<()>>,
    init_watchdog: Option<Task<()>>,
    orbit_drag: Option<Point<Pixels>>,
    _open_task: Option<Task<()>>,
}

pub struct ModelViewerPage {
    state: Entity<ViewerState>,
    #[cfg(not(target_family = "wasm"))]
    native: NativeRuntime,
    _subscriptions: Vec<Subscription>,
}

impl ModelViewerPage {
    /// Constructs the page and mounts its state. GPU-free: on native the
    /// engine init starts at the first render, not here.
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|cx| ViewerState::new(cx));
        let _subscriptions = vec![cx.observe(&state, |_, _, cx| cx.notify())];
        Self {
            state,
            #[cfg(not(target_family = "wasm"))]
            native: NativeRuntime::default(),
            _subscriptions,
        }
    }

    #[cfg(not(target_family = "wasm"))]
    fn start_runtime(&mut self, cx: &mut Context<Self>) {
        // Engine init and the renderer build both run behind the fence (a
        // wedged thread detaches); the drops stop the watchdog and free the engine.
        self.native.init_task = None;
        self.native.init_watchdog = None;
        self.native.renderer = None;
        self.native.engine = None;
        self.state.update(cx, |s, cx| s.clear_error(cx));
        let state = self.state.clone();
        self.native.init_task = Some(cx.spawn(async move |this, cx| {
            let mesh = state.read_with(cx, |s, _| s.mesh_for_restart());
            let built = cx
                .background_executor()
                .spawn(async move {
                    let engine = Arc::new(WgpuEngine::new_fenced()?);
                    let builder = engine.clone();
                    // Pipeline compiles run inside the driver; the fence
                    // bounds them so a wedge cannot pin this executor thread.
                    let renderer = builder.render_fenced(move || {
                        anyhow::Ok(Arc::new(Mutex::new(renderer::ModelRenderer::new(
                            engine.clone(),
                            &mesh,
                        )?)))
                    })?;
                    anyhow::Ok((builder, renderer))
                })
                .await;
            let _ = this.update(cx, |page, cx| match built {
                Ok((engine, renderer)) => {
                    page.native.engine = Some(engine);
                    page.native.renderer = Some(renderer);
                    // A slow success landing after the watchdog must dismiss
                    // the timeout panel, not stack a renderer under it.
                    page.state.update(cx, |s, cx| s.clear_error(cx));
                    cx.notify();
                }
                Err(err) => {
                    page.state
                        .update(cx, |s, cx| s.frame_failed(err.to_string(), cx));
                    cx.notify();
                }
            });
        }));
        self.native.init_watchdog = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(INIT_TIMEOUT).await;
            let _ = this.update(cx, |page, cx| {
                // A landed attempt carries a renderer or an error; only a
                // hung init still has neither when the deadline passes.
                if page.native.renderer.is_some() || page.state.read(cx).runtime_error().is_some() {
                    return;
                }
                let err = anyhow::anyhow!(
                    "wgpu adapter/device init timed out after {}s",
                    INIT_TIMEOUT.as_secs()
                );
                page.state
                    .update(cx, |s, cx| s.frame_failed(err.to_string(), cx));
                cx.notify();
            });
        }));
    }

    #[cfg(not(target_family = "wasm"))]
    fn open_model(&mut self, cx: &mut Context<Self>) {
        let state = self.state.clone();
        self.native._open_task = Some(cx.spawn(async move |this, cx| {
            let Some(path) = crate::desktop_actions::pick_file(cx).await else {
                return;
            };
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default();
            let _ = state.update(cx, |s, cx| s.set_loading(true, cx));
            let bytes = cx
                .background_spawn(async move { std::fs::read(path).map_err(|e| e.to_string()) })
                .await;
            let parsed = match bytes {
                Ok(bytes) => {
                    cx.background_spawn(async move {
                        obj::parse(&String::from_utf8_lossy(&bytes)).map_err(|e| e.to_string())
                    })
                    .await
                }
                Err(message) => Err(message),
            };
            let _ = this.update(cx, |page, cx| {
                page.state.update(cx, |s, cx| s.set_loading(false, cx));
                match parsed {
                    Ok(mesh) => {
                        page.state
                            .update(cx, |s, cx| s.install_mesh(Arc::new(mesh), name, cx));
                    }
                    Err(message) => {
                        tracing::warn!(target: LOG, error = %message, "model load failed");
                        crate::error_surface::report(
                            crate::i18n::localize_with_args(
                                "model_viewer_open_failed",
                                &[("error", message.as_str())],
                            ),
                            crate::errors::AppErrorSeverity::Warning,
                            crate::error_surface::ErrorCategory::System,
                            vec![crate::error_surface::ErrorAction::Dismiss],
                            cx,
                        );
                    }
                }
                cx.notify();
            });
        }));
    }

    fn heading() -> Stateful<Div> {
        div()
            .id("model-viewer-title")
            .a11y(Role::Heading, crate::i18n::localize("model_viewer_title"))
            .aria_level(1)
            .text_xl()
            .font_weight(FontWeight::BOLD)
            .child(crate::i18n::localize("model_viewer_title"))
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
                            .child(crate::i18n::localize("model_viewer_unavailable_wasm")),
                    ),
            )
    }

    #[cfg(not(target_family = "wasm"))]
    fn render_native(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Lazy engine start: construction stays GPU-free; the first visit
        // (this render) kicks exactly one init attempt.
        if !self.native.init_started {
            self.native.init_started = true;
            self.start_runtime(cx);
        }
        let (auto_rotate, loading, error, frames, fps, displayed, has_model, mesh_name, mesh_stats) = {
            let state = self.state.read(cx);
            (
                state.is_auto_rotating(),
                state.is_loading(),
                state.runtime_error().map(str::to_string),
                state.frames_rendered(),
                state.fps(),
                state.displayed().cloned(),
                state.has_loaded_model(),
                state.mesh_name().to_string(),
                state.mesh_stats(),
            )
        };

        let spin_label = crate::i18n::localize(if auto_rotate {
            "model_viewer_pause"
        } else {
            "model_viewer_play"
        });
        let header = h_flex()
            .w_full()
            .flex_wrap()
            .justify_between()
            .items_center()
            .child(Self::heading())
            .child(
                h_flex()
                    .gap_2()
                    .flex_shrink_0()
                    .child(
                        Button::new("model-viewer-open")
                            .outline()
                            .label(crate::i18n::localize(if loading {
                                "model_viewer_loading"
                            } else {
                                "model_viewer_open"
                            }))
                            .disabled(loading)
                            .on_click(cx.listener(|this, _, _, cx| this.open_model(cx))),
                    )
                    .child(
                        Button::new("model-viewer-spin")
                            .outline()
                            .label(spin_label)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.update(cx, |s, cx| s.toggle_auto_rotate(cx));
                            })),
                    )
                    .child(
                        Button::new("model-viewer-reset")
                            .outline()
                            .label(crate::i18n::localize("model_viewer_reset"))
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.state.update(cx, |s, cx| s.reset_view(cx));
                            })),
                    ),
            );

        let display_name = if has_model {
            mesh_name
        } else {
            crate::i18n::localize("model_viewer_default_name")
        };
        let stats = crate::i18n::localize_with_args(
            "model_viewer_stats",
            &[
                ("name", display_name.as_str()),
                ("tris", mesh_stats.triangles.to_string().as_str()),
                ("verts", mesh_stats.vertices.to_string().as_str()),
            ],
        );
        let stats_row = div()
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .child(stats);

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
            crate::i18n::localize("model_viewer_waiting")
        } else {
            crate::i18n::localize_with_args(
                "model_viewer_fps",
                &[("fps", fps.to_string().as_str())],
            )
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

        let a11y_label = crate::i18n::localize("model_viewer_viewport");
        let image_layer = match displayed {
            Some(image) => img(image)
                .size_full()
                .object_fit(ObjectFit::Contain)
                .into_any_element(),
            None => div().size_full().into_any_element(),
        };

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
                                .child(crate::i18n::localize("model_viewer_error_title")),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().danger)
                                .child(err.to_string()),
                        )
                        .child(
                            Button::new("model-viewer-retry")
                                .outline()
                                .label(crate::i18n::localize("model_viewer_retry"))
                                .on_click(cx.listener(|this, _, _, cx| this.start_runtime(cx))),
                        ),
                )
                // The ticker stays mounted on error: it keeps draining retired
                // frames, and its kick gate refuses frames while error is set.
                .child(self.render_ticker(cx))
                .into_any_element(),
            None => viewport_border(div())
                .id("model-viewer-viewport")
                .a11y(Role::Image, a11y_label)
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, event: &MouseDownEvent, _, _| {
                        this.native.orbit_drag = Some(event.position);
                    }),
                )
                .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                    let Some(last) = this.native.orbit_drag else {
                        return;
                    };
                    let delta = event.position - last;
                    this.native.orbit_drag = Some(event.position);
                    this.state.update(cx, |s, cx| {
                        s.orbit_by(
                            delta.x.as_f32() * ORBIT_SENSITIVITY,
                            -delta.y.as_f32() * ORBIT_SENSITIVITY,
                            cx,
                        );
                    });
                }))
                .on_mouse_up(
                    MouseButton::Left,
                    cx.listener(|this, _: &MouseUpEvent, _, _| {
                        this.native.orbit_drag = None;
                    }),
                )
                .on_scroll(cx.listener(|this, event: &ScrollEvent, _, cx| {
                    let delta_lines = match event.delta {
                        ScrollDelta::Lines(delta) => delta.y,
                        ScrollDelta::Pixels(delta) => delta.y.as_f32() / 40.,
                    };
                    let factor = (-delta_lines * 0.1).exp();
                    this.state.update(cx, |s, cx| s.zoom_by(factor, cx));
                }))
                .child(image_layer)
                .child(fps_overlay)
                .child(self.render_ticker(cx))
                .into_any_element(),
        };

        v_flex()
            .size_full()
            .p_6()
            .gap_4()
            .child(header)
            .child(stats_row)
            .child(content)
    }

    #[cfg(not(target_family = "wasm"))]
    fn render_ticker(&self, cx: &Context<Self>) -> Canvas<()> {
        let state = self.state.clone();
        let page = cx.weak_entity();
        canvas(
            move |_, _, _| (),
            move |bounds, _, window, cx| {
                let stale = state.update(cx, |s, _| s.take_stale_frames());
                for image in stale {
                    let _ = window.drop_image(image);
                }
                let scale = window.scale_factor();
                if bounds.size.width > px(0.) && bounds.size.height > px(0.) {
                    let physical = (
                        (bounds.size.width * scale).as_f32() as u32,
                        (bounds.size.height * scale).as_f32() as u32,
                    );
                    state.update(cx, |s, cx| {
                        s.viewport_changed(physical, cx);
                    });
                }

                let eligible = {
                    let snapshot = state.read(cx);
                    snapshot.wants_frame()
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
                    let (Some(engine), Some(renderer)) = (
                        page_view.native.engine.clone(),
                        page_view.native.renderer.clone(),
                    ) else {
                        return;
                    };
                    let still_eligible = {
                        let snapshot = state.read(cx);
                        snapshot.wants_frame()
                            && !snapshot.is_frame_in_flight()
                            && snapshot.runtime_error().is_none()
                            && snapshot.viewport_is_sized()
                    };
                    if !still_eligible {
                        return;
                    }
                    let now = Instant::now();
                    state.update(cx, |s, cx| s.mark_frame_started(now, cx));
                    cx.spawn(async move |cx| {
                        frame_task(state, engine, renderer, cx).await;
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
    state: Entity<ViewerState>,
    engine: Arc<WgpuEngine>,
    renderer: Arc<Mutex<renderer::ModelRenderer>>,
    cx: &mut AsyncApp,
) {
    let wait = state.read_with(cx, |s, _| s.pacing_wait(Instant::now()));
    if !wait.is_zero() {
        cx.background_executor().timer(wait).await;
    }
    if !state.read_with(cx, |s, _| s.wants_frame()) {
        return;
    }
    let job = state.read_with(cx, |s, _| s.take_render_job());
    let (width, height) = (job.width, job.height);
    let frame = cx
        .background_executor()
        .spawn(async move {
            engine.render_fenced(move || {
                let mut guard = renderer
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner);
                guard.render(job)
            })
        })
        .await;

    match frame {
        Ok(bytes) => {
            let image = crate::features::wgpu::rgba_to_render_image(width, height, bytes);
            let _ = state.update(cx, |s, cx| s.frame_done(Instant::now(), image, cx));
        }
        Err(err) => {
            let _ = state.update(cx, |s, cx| s.frame_failed(err.to_string(), cx));
        }
    }
}

impl Render for ModelViewerPage {
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

#[cfg(all(test, not(target_family = "wasm")))]
#[path = "mod.test.rs"]
mod model_viewer_test;
