#[cfg(not(target_family = "wasm"))]
use std::sync::Arc;

use gpui_kit::component::{ActiveTheme as _, v_flex};
#[cfg(not(target_family = "wasm"))]
use gpui_kit::component::{
    Selectable as _,
    button::{Button, ButtonGroup},
    h_flex,
};
use gpui_kit::{prelude::*, *};

use crate::accessibility::A11yExt as _;

use super::state::WgpuPageState;

#[cfg(not(target_family = "wasm"))]
use crate::platform::clock::{self, Duration, Instant};

#[cfg(not(target_family = "wasm"))]
use super::CAPABILITY_WGPU;
#[cfg(not(target_family = "wasm"))]
use super::demos::{DemoId, RenderJob, SceneRenderer};
#[cfg(not(target_family = "wasm"))]
use super::engine::WgpuEngine;
#[cfg(not(target_family = "wasm"))]
use super::state::PARTICLE_COUNTS;

/// Backstop deadline for a lost init: the fenced init bounds the attempt
/// itself, and this watchdog surfaces a timeout if its result never lands.
#[cfg(not(target_family = "wasm"))]
const INIT_TIMEOUT: Duration = Duration::from_secs(15);

#[cfg(test)]
thread_local! {
    /// Test seam for the init task: park it forever, or delay it past the
    /// watchdog deadline before running the real init.
    static INIT_SEAM: std::cell::Cell<InitSeam> =
        const { std::cell::Cell::new(InitSeam::Real) };
}

#[cfg(test)]
#[derive(Clone, Copy)]
enum InitSeam {
    Real,
    Park,
    Delay(Duration),
}

#[cfg(test)]
fn set_init_seam(seam: InitSeam) {
    INIT_SEAM.with(|slot| slot.set(seam));
}

#[cfg(test)]
thread_local! {
    /// Test seam for the fenced frame: Wedge parks the render work and
    /// WedgeCreate the renderer build, tripping the real fence.
    static FRAME_SEAM: std::cell::Cell<FrameSeam> =
        const { std::cell::Cell::new(FrameSeam::Real) };
}

#[cfg(test)]
#[derive(Clone, Copy)]
enum FrameSeam {
    Real,
    Wedge,
    WedgeCreate,
}

#[cfg(test)]
fn set_frame_seam(seam: FrameSeam) {
    FRAME_SEAM.with(|slot| slot.set(seam));
}

pub struct WgpuPage {
    // The native render loop is the only reader; wasm carries it unused.
    #[cfg_attr(target_family = "wasm", allow(dead_code))]
    state: Entity<WgpuPageState>,
    #[cfg(not(target_family = "wasm"))]
    engine: Option<Arc<WgpuEngine>>,
    #[cfg(not(target_family = "wasm"))]
    renderer: Option<Arc<std::sync::Mutex<SceneRenderer>>>,
    #[cfg(not(target_family = "wasm"))]
    init_started: bool,
    #[cfg(not(target_family = "wasm"))]
    init_task: Option<Task<()>>,
    #[cfg(not(target_family = "wasm"))]
    init_watchdog: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl WgpuPage {
    /// Constructs the page and its state entity. GPU-free: on native the
    /// engine init starts at the first render, not here.
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|cx| WgpuPageState::new(cx));
        Self {
            state,
            #[cfg(not(target_family = "wasm"))]
            engine: None,
            #[cfg(not(target_family = "wasm"))]
            renderer: None,
            #[cfg(not(target_family = "wasm"))]
            init_started: false,
            #[cfg(not(target_family = "wasm"))]
            init_task: None,
            #[cfg(not(target_family = "wasm"))]
            init_watchdog: None,
            _subscriptions: Vec::new(),
        }
    }

    #[cfg(not(target_family = "wasm"))]
    fn start_engine(&mut self, cx: &mut Context<Self>) {
        // The fence joins each init attempt with a timeout and detaches a
        // wedged one; the drops stop the watchdog and free any poisoned engine.
        self.init_task = None;
        self.init_watchdog = None;
        self.engine = None;
        self.renderer = None;
        self.state.update(cx, |s, cx| s.clear_error(cx));
        #[cfg(test)]
        let seam = INIT_SEAM.with(std::cell::Cell::get);
        self.init_task = Some(cx.spawn(async move |this, cx| {
            #[cfg(test)]
            if let InitSeam::Delay(delay) = seam {
                cx.background_executor().timer(delay).await;
            }
            let engine = cx
                .background_executor()
                .spawn(async move {
                    #[cfg(test)]
                    if matches!(seam, InitSeam::Park) {
                        std::future::pending::<()>().await;
                    }
                    WgpuEngine::new_fenced()
                })
                .await;
            match engine {
                Ok(engine) => {
                    let engine = Arc::new(engine);
                    let _ = this.update(cx, |page, cx| {
                        page.engine = Some(engine);
                        page.state.update(cx, |s, cx| s.clear_error(cx));
                        crate::capabilities::set(
                            CAPABILITY_WGPU,
                            crate::capabilities::CapabilityStatus::supported_enabled(),
                            cx,
                        );
                        cx.notify();
                    });
                }
                Err(err) => {
                    let _ = cx.update(|cx| {
                        crate::capabilities::set(
                            CAPABILITY_WGPU,
                            crate::capabilities::CapabilityStatus::error(err.to_string()),
                            cx,
                        );
                    });
                    let _ = this.update(cx, |page, cx| {
                        page.state
                            .update(cx, |s, cx| s.frame_failed(err.to_string(), cx))
                    });
                }
            }
        }));
        self.init_watchdog = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(INIT_TIMEOUT).await;
            let _ = this.update(cx, |page, cx| {
                // A landed attempt carries an engine or an error; only a
                // hung init still has neither when the deadline passes.
                if page.engine.is_some() || page.state.read(cx).runtime_error().is_some() {
                    return;
                }
                let err = anyhow::anyhow!(
                    "wgpu adapter/device init timed out after {}s",
                    INIT_TIMEOUT.as_secs()
                );
                crate::capabilities::set(
                    CAPABILITY_WGPU,
                    crate::capabilities::CapabilityStatus::error(err.to_string()),
                    cx,
                );
                page.state
                    .update(cx, |s, cx| s.frame_failed(err.to_string(), cx));
                cx.notify();
            });
        }));
    }

    fn heading() -> Stateful<Div> {
        div()
            .id("wgpu-title")
            .a11y(Role::Heading, crate::i18n::localize("wgpu_title"))
            .aria_level(1)
            .text_xl()
            .font_weight(FontWeight::BOLD)
            .child(crate::i18n::localize("wgpu_title"))
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
                            .child(crate::i18n::localize("wgpu_unavailable_wasm")),
                    ),
            )
    }

    #[cfg(not(target_family = "wasm"))]
    fn render_native(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Lazy engine start: construction stays GPU-free; the first visit
        // (this render) kicks exactly one init attempt.
        if !self.init_started {
            self.init_started = true;
            self.start_engine(cx);
        }
        let (playing, demo_ix, particle_count, fps, engine_ms, error, displayed, frames) = {
            let state = self.state.read(cx);
            (
                state.is_playing(),
                state.demo_ix(),
                state.particle_count(),
                state.fps(),
                state.engine_ms(),
                state.runtime_error().map(str::to_string),
                state.displayed().cloned(),
                state.frames_rendered(),
            )
        };
        let adapter_label = self
            .engine
            .as_ref()
            .map(|engine| engine.adapter_label().to_string());

        let play_label = crate::i18n::localize(if playing { "wgpu_pause" } else { "wgpu_play" });
        let header = h_flex()
            .w_full()
            .flex_wrap()
            .justify_between()
            .items_center()
            .child(Self::heading())
            .child(
                h_flex().gap_2().flex_shrink_0().child(
                    Button::new("wgpu-toggle-play")
                        .outline()
                        .label(play_label)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.state.update(cx, |s, cx| s.toggle_playing(cx));
                        })),
                ),
            );

        let demo_labels = [
            crate::i18n::localize("wgpu_demo_particles"),
            crate::i18n::localize("wgpu_demo_mesh"),
            crate::i18n::localize("wgpu_demo_mesh_post"),
        ];
        let demo_ids = [
            "wgpu-demo-particles",
            "wgpu-demo-mesh",
            "wgpu-demo-mesh-post",
        ];
        let demos =
            ButtonGroup::new("wgpu-demos")
                .children(demo_labels.into_iter().zip(demo_ids).enumerate().map(
                    |(ix, (label, id))| {
                        Button::new(SharedString::from(id))
                            .label(label)
                            .selected(demo_ix == ix)
                    },
                ))
                .on_click(cx.listener(|this, clicks: &Vec<usize>, _, cx| {
                    if let Some(&ix) = clicks.first() {
                        this.state.update(cx, |s, cx| s.set_demo_ix(ix, cx));
                    }
                }));

        let count_buttons = ButtonGroup::new("wgpu-count")
            .outline()
            .children((0..PARTICLE_COUNTS.len()).map(|ix| {
                Button::new(SharedString::from(format!("wgpu-count-{ix}")))
                    .label(crate::i18n::localize(match ix {
                        0 => "wgpu_particles_100k",
                        1 => "wgpu_particles_500k",
                        _ => "wgpu_particles_1m",
                    }))
                    .selected(PARTICLE_COUNTS[ix] == particle_count)
            }))
            .on_click(cx.listener(|this, clicks: &Vec<usize>, _, cx| {
                if let Some(&ix) = clicks.first().filter(|ix| **ix < PARTICLE_COUNTS.len()) {
                    this.state
                        .update(cx, |s, cx| s.set_particle_count(PARTICLE_COUNTS[ix], cx));
                }
            }));

        let viewport_border = |div: Div| {
            div.relative()
                .flex_1()
                .min_h_0()
                .rounded(cx.theme().radius_lg)
                .overflow_hidden()
                .border_1()
                .border_color(cx.theme().border)
        };

        let content = match (error.as_deref(), displayed) {
            (Some(err), _) => viewport_border(div())
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
                                .child(crate::i18n::localize("wgpu_error_title")),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().danger)
                                .child(err.to_string()),
                        )
                        .child(
                            Button::new("wgpu-retry")
                                .outline()
                                .label(crate::i18n::localize("wgpu_retry"))
                                .on_click(cx.listener(|this, _, _, cx| this.start_engine(cx))),
                        ),
                )
                // The ticker keeps draining retired frames while the error panel
                // shows; its kick gate refuses to start frames (runtime_error).
                .child(self.render_ticker(cx)),
            (None, Some(image)) => viewport_border(div())
                .child(
                    div()
                        .id("wgpu-frame")
                        .role(Role::Image)
                        .aria_label(crate::i18n::localize("wgpu_surface_label"))
                        .size_full()
                        .child(img(image).size_full().object_fit(ObjectFit::Contain)),
                )
                .when(frames > 0, |viewport| {
                    viewport.child(self.render_stats_overlay(
                        cx,
                        fps,
                        engine_ms,
                        adapter_label.as_deref(),
                    ))
                })
                .child(self.render_ticker(cx)),
            (None, None) => viewport_border(div())
                .child(
                    div()
                        .size_full()
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_color(cx.theme().muted_foreground)
                        .child(div().text_sm().child(crate::i18n::localize("wgpu_waiting"))),
                )
                .child(self.render_ticker(cx)),
        };

        v_flex()
            .size_full()
            .p_6()
            .gap_4()
            .child(header)
            .child(demos)
            .when(demo_ix == 0, |page| page.child(count_buttons))
            .child(content)
    }

    #[cfg(not(target_family = "wasm"))]
    fn render_stats_overlay(
        &self,
        cx: &mut Context<Self>,
        fps: u32,
        engine_ms: f32,
        adapter_label: Option<&str>,
    ) -> Div {
        let stats = h_flex()
            .gap_3()
            .child(crate::i18n::localize_with_args(
                "wgpu_fps",
                &[("fps", &fps.to_string())],
            ))
            .child(crate::i18n::localize_with_args(
                "wgpu_ms",
                &[("ms", &format!("{engine_ms:.1}"))],
            ));
        let stats = match adapter_label {
            Some(label) => stats.child(label.to_string()),
            None => stats,
        };
        div()
            .absolute()
            .top(rems(0.5))
            .left(rems(0.5))
            .px_2()
            .py_1()
            .rounded_md()
            .bg(cx.theme().popover)
            .border_1()
            .border_color(cx.theme().border)
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(stats)
    }

    #[cfg(not(target_family = "wasm"))]
    fn render_ticker(&self, cx: &Context<Self>) -> Canvas<()> {
        let state = self.state.clone();
        let page = cx.weak_entity();
        let engine = self.engine.clone();
        canvas(
            move |_, _, _| (),
            move |bounds, _, window, cx| {
                for image in state.update(cx, |s, _| s.take_stale_frames()) {
                    let _ = window.drop_image(image);
                }
                let scale = window.scale_factor();
                let physical = (
                    (f32::from(bounds.size.width) * scale) as u32,
                    (f32::from(bounds.size.height) * scale) as u32,
                );
                if physical.0 > 0 && physical.1 > 0 {
                    state.update(cx, |s, cx| s.viewport_changed(physical, cx));
                }

                let Some(engine) = engine else {
                    return;
                };
                let eligible = {
                    let snapshot = state.read(cx);
                    snapshot.is_playing()
                        && !snapshot.is_frame_in_flight()
                        && snapshot.runtime_error().is_none()
                        && snapshot.pending_size() != (0, 0)
                };
                if !eligible {
                    return;
                }

                window.on_next_frame(move |_, cx| {
                    let Some(page) = page.upgrade() else {
                        return;
                    };
                    let still_eligible = {
                        let snapshot = state.read(cx);
                        snapshot.is_playing()
                            && !snapshot.is_frame_in_flight()
                            && snapshot.runtime_error().is_none()
                            && snapshot.pending_size() != (0, 0)
                    };
                    if !still_eligible {
                        return;
                    }
                    let now = clock::Instant::now();
                    state.update(cx, |s, cx| s.mark_frame_started(now, cx));
                    cx.spawn(async move |cx| {
                        frame_task(page, state, engine, cx).await;
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
    page: Entity<WgpuPage>,
    state: Entity<WgpuPageState>,
    engine: Arc<WgpuEngine>,
    cx: &mut AsyncApp,
) {
    let now = Instant::now();
    let wait = state.read_with(cx, |s, _| s.pacing_wait(now));
    if !wait.is_zero() {
        cx.background_executor().timer(wait).await;
    }

    // Snapshot after the wait: dt must span the paced frame period, not the
    // microseconds between the kick and this task's first line.
    let now = Instant::now();
    let (playing, demo_ix, particle_count, size, dt, elapsed) = state.read_with(cx, |s, _| {
        (
            s.is_playing(),
            s.demo_ix(),
            s.particle_count(),
            s.pending_size(),
            s.frame_dt(now),
            s.elapsed_at(now),
        )
    });
    let (width, height) = size;
    if !playing || width == 0 || height == 0 {
        state.update(cx, |s, cx| s.cancel_frame(cx));
        return;
    }

    let job = RenderJob {
        demo: DemoId::from_ix(demo_ix),
        particle_count,
        dt,
        elapsed,
        width,
        height,
    };
    // Read on the caller thread before the spawns so the workers never
    // touch the thread-local seam.
    #[cfg(test)]
    let frame_seam = FRAME_SEAM.with(std::cell::Cell::get);
    let renderer = match page.read_with(cx, |p, _| p.renderer.clone()) {
        Some(renderer) => renderer,
        None => {
            // Creation compiles pipelines in the driver; a wedge must
            // poison and reach the panel, not park the frame task.
            let builder = engine.clone();
            let for_new = engine.clone();
            let created = cx
                .background_executor()
                .spawn(async move {
                    builder.render_fenced(move || {
                        #[cfg(test)]
                        if matches!(frame_seam, FrameSeam::WedgeCreate) {
                            // Stand-in for a pipeline compile wedged inside
                            // the driver; the fence trips while this parks.
                            let (_tx, rx) = std::sync::mpsc::channel::<()>();
                            let _ = rx.recv();
                        }
                        SceneRenderer::new(for_new, &job)
                    })
                })
                .await;
            match created {
                Ok(renderer) => {
                    let renderer = Arc::new(std::sync::Mutex::new(renderer));
                    let _ = page.update(cx, |p, _| p.renderer = Some(renderer.clone()));
                    renderer
                }
                Err(err) => {
                    frame_failed(&state, err, cx);
                    return;
                }
            }
        }
    };

    let rendered = cx
        .background_executor()
        .spawn(async move {
            engine.render_fenced(move || {
                #[cfg(test)]
                if matches!(frame_seam, FrameSeam::Wedge) {
                    // Stand-in for a readback wedged inside the driver; the
                    // fence trips while this parks.
                    let (_tx, rx) = std::sync::mpsc::channel::<()>();
                    let _ = rx.recv();
                }
                let started = Instant::now();
                let mut guard = renderer.lock().expect("wgpu renderer mutex");
                guard
                    .render(job)
                    .map(|bytes| (bytes, started.elapsed().as_secs_f32() * 1000.0))
            })
        })
        .await;

    match rendered {
        Ok((bytes, engine_ms)) => {
            let image = super::frame::bgra_to_render_image(width, height, bytes);
            state.update(cx, |s, cx| {
                s.frame_done(Instant::now(), image, engine_ms, cx);
            });
        }
        Err(err) => frame_failed(&state, err, cx),
    }
}

#[cfg(not(target_family = "wasm"))]
fn frame_failed(state: &Entity<WgpuPageState>, err: anyhow::Error, cx: &mut AsyncApp) {
    let _ = cx.update(|cx| {
        crate::capabilities::set(
            CAPABILITY_WGPU,
            crate::capabilities::CapabilityStatus::degraded(err.to_string()),
            cx,
        );
    });
    state.update(cx, |s, cx| s.frame_failed(err.to_string(), cx));
}

#[cfg(all(test, not(target_family = "wasm")))]
#[path = "page.test.rs"]
mod page_test;

impl Render for WgpuPage {
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
