//! Removable infinite-canvas page over the ui::canvas engine.
//! `InfiniteCanvasPage` is the only public surface. Removal = delete this
//! directory and revert its six seam touches: the `mod`/`pub use` lines in
//! features/pages/mod.rs, the Page::InfiniteCanvas arms and all() entry in
//! shell/sidebar.rs, the VALID_HOSTS entry in shell/route.rs, the AppRoot
//! wiring in shell/root/app_root/state.rs, the page count in
//! tests/e2e_navigation.rs, and the nav_infinite_canvas/canvas_* keys in
//! locales/en.yml and locales/zh-CN.yml.

mod content;
mod explore_tool;
mod minimap;
mod perf;
mod viewport;

#[cfg(test)]
#[path = "content.test.rs"]
mod content_test;

#[cfg(test)]
#[path = "explore_tool.test.rs"]
mod explore_tool_test;

#[cfg(test)]
#[path = "minimap.test.rs"]
mod minimap_test;

#[cfg(test)]
#[path = "perf.test.rs"]
mod perf_test;

#[cfg(test)]
#[path = "viewport.test.rs"]
mod viewport_test;

use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::{ActiveTheme as _, ElementExt as _, IconName, h_flex, v_flex};
use gpui_kit::{prelude::*, *};

use crate::accessibility::A11yExt as _;
use crate::ui::canvas::{BuiltinTool, Canvas, CanvasGrid, CanvasState, LayerId};

use content::{NoteField, build_field, note_palette};
use explore_tool::{ExploreTool, LABEL_LOD_ZOOM, clamp_zoom, page_factor};
use viewport::interaction_probe;

pub(crate) const PRESETS: [usize; 3] = [1_000, 5_000, 20_000];
const DEFAULT_PRESET_IX: usize = 1;

pub struct InfiniteCanvasPage {
    state: Entity<CanvasState>,
    field: NoteField,
    layers: (LayerId, LayerId),
    palette: [Hsla; 5],
    viewport: Bounds<Pixels>,
    minimap_bounds: Bounds<Pixels>,
    minimap_dots: std::rc::Rc<std::cell::RefCell<minimap::MinimapDots>>,
    space_pan: bool,
    preset_ix: usize,
    _subscriptions: Vec<Subscription>,
    focus_handle: FocusHandle,
}

impl InfiniteCanvasPage {
    /// Mounts the engine, builds the default 5,000-note field, installs
    /// ExploreTool. Constructed once by AppRoot; nothing else required.
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let field = NoteField {
            seed: 0x06C0FFEE,
            count: 5_000,
        };
        let state = cx.new(|_| CanvasState::new());
        let palette = note_palette(cx);
        let layers = state.update(cx, |state, cx| build_field(state, &field, cx));
        state.update(cx, |state, cx| {
            state.set_tool(Box::new(ExploreTool::new()), cx)
        });
        let subscription = cx.observe(&state, |this, _, cx| {
            this.reconcile_lod(cx);
            cx.notify();
        });
        Self {
            state,
            field,
            layers,
            palette,
            viewport: Bounds::default(),
            minimap_bounds: Bounds::default(),
            minimap_dots: Default::default(),
            space_pan: false,
            preset_ix: DEFAULT_PRESET_IX,
            _subscriptions: vec![subscription],
            focus_handle: cx.focus_handle(),
        }
    }

    fn set_space_pan(&mut self, active: bool, cx: &mut Context<Self>) {
        if self.space_pan == active {
            return;
        }
        self.space_pan = active;
        if active {
            self.state
                .update(cx, |state, cx| state.set_builtin_tool(BuiltinTool::Pan, cx));
        } else {
            self.state.update(cx, |state, cx| {
                state.set_tool(Box::new(ExploreTool::new()), cx)
            });
        }
        cx.notify();
    }

    fn reconcile_lod(&mut self, cx: &mut Context<Self>) {
        let desired = self.state.read(cx).zoom() >= LABEL_LOD_ZOOM;
        let labels = self.layers.1;
        let stale = self
            .state
            .read(cx)
            .document()
            .layer(labels)
            .is_some_and(|layer| layer.is_visible() != desired);
        if stale {
            self.state
                .update(cx, |state, cx| state.set_layer_visible(labels, desired, cx));
        }
    }

    fn store_viewport(&mut self, bounds: Bounds<Pixels>) {
        if self.viewport != bounds {
            self.viewport = bounds;
        }
    }

    fn store_minimap_bounds(&mut self, bounds: Bounds<Pixels>) {
        if self.minimap_bounds != bounds {
            self.minimap_bounds = bounds;
        }
    }

    fn zoom_view(&mut self, factor: f32, cx: &mut Context<Self>) {
        if self.viewport.size.width <= px(0.) {
            return;
        }
        let anchor = point(
            self.viewport.size.width * 0.5,
            self.viewport.size.height * 0.5,
        );
        let applied = page_factor(self.state.read(cx).zoom(), factor);
        if applied != 1.0 {
            self.state
                .update(cx, |state, cx| state.zoom_at(anchor, applied, cx));
        }
    }

    fn fit_view(&mut self, cx: &mut Context<Self>) {
        if self.viewport.size.width <= px(0.) {
            return;
        }
        let span = self.field.span();
        let viewport = self.viewport;
        self.state.update(cx, |state, cx| {
            state.zoom_to_fit(span, viewport.size, cx);
            let clamped = clamp_zoom(state.zoom());
            if clamped != state.zoom() {
                state.set_zoom(clamped, cx);
                let local_center = point(viewport.size.width * 0.5, viewport.size.height * 0.5);
                let target_pan = local_center - span.center() * state.zoom();
                let delta = target_pan - state.transform().pan;
                if delta.x != px(0.) || delta.y != px(0.) {
                    state.pan_by(delta, cx);
                }
            }
        });
    }

    fn reset_view(&mut self, cx: &mut Context<Self>) {
        self.state.update(cx, |state, cx| state.reset_view(cx));
    }

    fn toggle_grid(&mut self, cx: &mut Context<Self>) {
        let grid = self.state.read(cx).grid();
        self.state.update(cx, |state, cx| {
            state.set_grid(
                CanvasGrid {
                    visible: !grid.visible,
                    spacing: grid.spacing,
                },
                cx,
            )
        });
    }

    fn regenerate(&mut self, count: usize, cx: &mut Context<Self>) {
        let started = crate::platform::clock::Instant::now();
        let field = NoteField {
            seed: self.field.seed.wrapping_add(0x9E3779B9),
            count,
        };
        self.palette = note_palette(cx);
        self.layers = self
            .state
            .update(cx, |state, cx| build_field(state, &field, cx));
        self.field = field;
        self.preset_ix = PRESETS
            .iter()
            .position(|&preset| preset == count)
            .unwrap_or(self.preset_ix);
        let gen_us = started.elapsed().as_micros() as u64;
        perf::store_gen_us(gen_us);
        tracing::info!(
            target: "gpui_starter::canvas_page",
            count,
            gen_us,
            "canvas field regenerated"
        );
        cx.notify();
    }
}

fn icon_button(
    id: &'static str,
    icon: IconName,
    key: &'static str,
    on_click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> Button {
    Button::new(id)
        .ghost()
        .icon(icon)
        .tooltip(crate::i18n::localize(key))
        .accessibility_label(crate::i18n::localize(key))
        .on_click(on_click)
}

impl Render for InfiniteCanvasPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let started = crate::platform::clock::Instant::now();
        let page = cx.entity();
        let (zoom, grid) = {
            let state = self.state.read(cx);
            (state.zoom(), state.grid())
        };
        let preset_ix = self.preset_ix;

        let controls = h_flex()
            .id("canvas-controls")
            .a11y(Role::Group, crate::i18n::localize("canvas_zoom_level"))
            .gap_1()
            .child(icon_button(
                "canvas-zoom-out",
                IconName::Minus,
                "canvas_zoom_out",
                cx.listener(|this, _: &ClickEvent, _, cx| this.zoom_view(1.0 / 1.25, cx)),
            ))
            .child(
                div()
                    .id("canvas-zoom-level")
                    .w_12()
                    .text_center()
                    .text_sm()
                    .child(format!("{}%", (zoom * 100.).round())),
            )
            .child(icon_button(
                "canvas-zoom-in",
                IconName::Plus,
                "canvas_zoom_in",
                cx.listener(|this, _: &ClickEvent, _, cx| this.zoom_view(1.25, cx)),
            ))
            .child(icon_button(
                "canvas-fit",
                IconName::Maximize,
                "canvas_zoom_to_fit",
                cx.listener(|this, _: &ClickEvent, _, cx| this.fit_view(cx)),
            ))
            .child(icon_button(
                "canvas-reset",
                IconName::Map,
                "canvas_reset_view",
                cx.listener(|this, _: &ClickEvent, _, cx| this.reset_view(cx)),
            ))
            .child(
                icon_button(
                    "canvas-grid",
                    IconName::LayoutDashboard,
                    "canvas_toggle_grid",
                    cx.listener(|this, _: &ClickEvent, _, cx| this.toggle_grid(cx)),
                )
                .toggled(grid.visible),
            );

        let presets = h_flex()
            .gap_1()
            .children(PRESETS.iter().enumerate().map(|(ix, &count)| {
                Button::new(SharedString::from(format!("canvas-preset-{ix}")))
                    .outline()
                    .compact()
                    .label(format!("{}k", count / 1_000))
                    .toggled(ix == preset_ix)
                    .on_click(
                        cx.listener(move |this, _: &ClickEvent, _, cx| this.regenerate(count, cx)),
                    )
            }));

        let stats = div()
            .text_xs()
            .text_color(cx.theme().muted_foreground)
            .child(crate::i18n::localize_with_args(
                "canvas_items",
                &[("count", &self.field.count.to_string())],
            ));

        let hud = v_flex()
            .p_2()
            .gap_1()
            .border_b_1()
            .border_color(cx.theme().border)
            .flex_shrink_0()
            .child(
                h_flex()
                    .w_full()
                    .gap_2()
                    .items_center()
                    .justify_between()
                    .child(controls)
                    .child(
                        h_flex()
                            .gap_1()
                            .items_center()
                            .child(presets)
                            .child(icon_button(
                                "canvas-regen",
                                IconName::RefreshCw,
                                "canvas_regenerate",
                                cx.listener(|this, _: &ClickEvent, _, cx| {
                                    this.regenerate(PRESETS[this.preset_ix], cx)
                                }),
                            ))
                            .child(stats),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(crate::i18n::localize("canvas_pan_hint")),
                    )
                    .when(cfg!(debug_assertions), |row| {
                        row.child(
                            div()
                                .text_xs()
                                .text_color(cx.theme().muted_foreground)
                                .child(format!(
                                    "Frame: {:.2}ms · Page: {:.2}ms · Gen: {:.2}ms",
                                    crate::root::last_frame_time_us() as f64 / 1000.,
                                    perf::last_page_render_us() as f64 / 1000.,
                                    perf::last_gen_us() as f64 / 1000.,
                                )),
                        )
                    }),
            );

        let tree = v_flex()
            .id("infinite-canvas")
            .size_full()
            .min_h_0()
            .child(hud)
            .child(
                div()
                    .id("canvas-area")
                    .flex_1()
                    .min_h_0()
                    .relative()
                    .track_focus(&self.focus_handle)
                    .on_prepaint({
                        let page = page.clone();
                        move |bounds, _, cx| {
                            page.update(cx, |this, _| this.store_viewport(bounds));
                        }
                    })
                    .child(Canvas::new("infinite-canvas", self.state.clone()))
                    .child(interaction_probe(page.clone(), self.state.clone()))
                    .child(minimap::minimap(
                        page.clone(),
                        self.minimap_dots.clone(),
                        self.state.clone(),
                        cx,
                    )),
            );

        let render_us = started.elapsed().as_micros() as u64;
        perf::store_page_render_us(render_us);
        if perf::is_slow_page_render(render_us) {
            tracing::warn!(
                target: "gpui_starter::canvas_page",
                render_us,
                "slow canvas page render"
            );
        }
        tree
    }
}
