use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, FocusTrapElement as _,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};
use gpui_kit::{prelude::*, *};

use crate::accessibility::A11yExt as _;
use crate::features::tutorial::mask;
use crate::features::tutorial::target::{resolve, set_subscriber};
use crate::features::tutorial::tour::{TourDefinition, TourProgress, TourStep, demo_tour};

const TOUR_CONTEXT: &str = "Tour";
const TOUR_VIEWPORT: &str = "tour-viewport";
const LOG: &str = "gpui_starter::tutorial";

#[derive(Action, Clone, PartialEq, Eq, serde::Deserialize)]
#[action(namespace = tutorial, no_json)]
pub struct TourNext;

#[derive(Action, Clone, PartialEq, Eq, serde::Deserialize)]
#[action(namespace = tutorial, no_json)]
pub struct TourBack;

#[derive(Action, Clone, PartialEq, Eq, serde::Deserialize)]
#[action(namespace = tutorial, no_json)]
pub struct TourDismiss;

/// Bind the tour keys (context-scoped; inert unless the overlay is focused).
pub fn initialize(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("right", TourNext, Some(TOUR_CONTEXT)),
        KeyBinding::new("enter", TourNext, Some(TOUR_CONTEXT)),
        KeyBinding::new("left", TourBack, Some(TOUR_CONTEXT)),
        KeyBinding::new("escape", TourDismiss, Some(TOUR_CONTEXT)),
    ]);
}

pub(crate) struct TourHandle(pub Entity<TourController>);

impl Global for TourHandle {}

pub struct TourController {
    active: Option<ActiveTour>,
    focus_handle: FocusHandle,
    restore_focus: Option<FocusHandle>,
}

struct ActiveTour {
    definition: TourDefinition,
    progress: TourProgress,
}

impl TourController {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        set_subscriber(cx.entity().downgrade(), cx);
        Self {
            active: None,
            focus_handle: cx.focus_handle(),
            restore_focus: None,
        }
    }

    fn begin(&mut self, definition: TourDefinition, window: &mut Window, cx: &mut Context<Self>) {
        if self.active.is_some() || definition.steps.is_empty() {
            return;
        }
        let definition_id = definition.id.clone();
        let total = definition.steps.len();
        self.restore_focus = window.focused(cx);
        self.active = Some(ActiveTour {
            definition,
            progress: TourProgress::new(total),
        });
        self.enter_step(window, cx);
        tracing::info!(target: LOG, definition = %definition_id, "tour started");
    }

    fn enter_step(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(route) = self.current_step().and_then(|step| step.route.clone()) {
            crate::events::emit(crate::events::AppEventKind::Navigate(route), cx);
        }
        self.focus_handle.focus(window, cx);
        cx.notify();
    }

    fn advance(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self
            .active
            .as_ref()
            .is_some_and(|tour| tour.progress.is_last())
        {
            self.close(window, cx, "finished");
            return;
        }
        if self
            .active
            .as_mut()
            .is_some_and(|tour| tour.progress.advance())
        {
            self.enter_step(window, cx);
        }
    }

    fn go_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self
            .active
            .as_mut()
            .is_some_and(|tour| tour.progress.go_back())
        {
            self.enter_step(window, cx);
        }
    }

    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>, reason: &'static str) {
        if self.active.take().is_none() {
            return;
        }
        if let Some(handle) = self.restore_focus.take() {
            handle.focus(window, cx);
        }
        cx.notify();
        tracing::info!(target: LOG, reason, "tour closed");
    }

    fn current_step(&self) -> Option<&TourStep> {
        self.active
            .as_ref()
            .map(|tour| &tour.definition.steps[tour.progress.ix()])
    }
}

impl Focusable for TourController {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

pub fn start(cx: &mut App) {
    start_tour(demo_tour(), cx);
}

/// Start `definition` in the root window; a logged no-op when the root
/// window or the controller is not up yet.
pub fn start_tour(definition: TourDefinition, cx: &mut App) {
    let Some(controller) = cx.try_global::<TourHandle>().map(|handle| handle.0.clone()) else {
        tracing::warn!(target: LOG, "tour requested before the root window exists");
        return;
    };
    let Some(root) = crate::app::window::root_window(cx) else {
        tracing::warn!(target: LOG, "tour requested without a root window");
        return;
    };
    let _ = root.update(cx, |_, window, cx| {
        controller.update(cx, |tour, cx| tour.begin(definition, window, cx));
    });
}

/// Close the active tour from App level (same path as Escape/Skip).
pub fn dismiss(cx: &mut App) {
    let Some(controller) = cx.try_global::<TourHandle>().map(|handle| handle.0.clone()) else {
        return;
    };
    let Some(root) = crate::app::window::root_window(cx) else {
        return;
    };
    let _ = root.update(cx, |_, window, cx| {
        controller.update(cx, |tour, cx| tour.close(window, cx, "dismissed"));
    });
}

impl Render for TourController {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(tour) = &self.active else {
            return div().into_any_element();
        };
        let step = &tour.definition.steps[tour.progress.ix()];

        // Renders read prepaint-published bounds one frame behind (gpui lays
        // out the whole tree before prepaint), so an unresolved target here
        // is the first frame after a route-changing step, not a bug.
        let Some(viewport) = resolve(TOUR_VIEWPORT, cx) else {
            return self.overlay(cx, waiting_placeholder());
        };
        let Some(target) = resolve(step.target, cx) else {
            return self.overlay(cx, waiting_placeholder());
        };

        let rem = window.rem_size();
        let pad = rems(0.5).to_pixels(rem);
        let cutout = mask::cutout_bounds(target, pad);
        let card_bounds = mask::place_card(
            target,
            viewport,
            rems(20.).to_pixels(rem),
            rems(12.).to_pixels(rem),
            pad,
        );

        let mask_canvas = canvas(
            |_, _, _| (),
            move |bounds, (), window, cx| {
                mask::paint_spotlight(window, cx, bounds, cutout, window.rem_size());
            },
        )
        .absolute()
        .size_full()
        .top_0()
        .left_0();

        let overlay_root = div()
            .id("tour-overlay")
            .a11y(Role::Dialog, crate::i18n::localize("tutorial_dialog"))
            .occlude()
            .absolute()
            .size_full()
            .top_0()
            .left_0()
            .child(mask_canvas)
            .child(render_card(
                step,
                card_bounds,
                tour.progress.ix(),
                tour.progress.total(),
                tour.progress.is_last(),
                cx,
            ));

        self.overlay(cx, overlay_root)
    }
}

impl TourController {
    // Action dispatch bubbles along the focused handle's ancestor chain
    // only (gpui key_dispatch.rs), and the trap tracks the handle here, so
    // the key context and handlers must live on this same node.
    fn overlay(&self, cx: &Context<Self>, child: impl IntoElement) -> AnyElement {
        div()
            .focus_trap("tour", &self.focus_handle)
            .key_context(TOUR_CONTEXT)
            .on_action(cx.listener(|this, _: &TourNext, window, cx| this.advance(window, cx)))
            .on_action(cx.listener(|this, _: &TourBack, window, cx| this.go_back(window, cx)))
            .on_action(
                cx.listener(|this, _: &TourDismiss, window, cx| {
                    this.close(window, cx, "dismissed")
                }),
            )
            .absolute()
            .size_full()
            .top_0()
            .left_0()
            .child(child)
            .into_any_element()
    }
}

// Transparent input blocker for the frame between a route-changing step and
// its target's first publish; that publish re-renders the full overlay.
fn waiting_placeholder() -> impl IntoElement {
    div().id("tour-waiting").size_full().occlude()
}

fn render_card(
    step: &TourStep,
    card_bounds: Bounds<Pixels>,
    ix: usize,
    total: usize,
    is_last: bool,
    cx: &Context<TourController>,
) -> impl IntoElement {
    let title = crate::i18n::localize(step.title_key);
    let body = crate::i18n::localize(step.body_key);
    let step_num = (ix + 1).to_string();
    let total_num = total.to_string();
    let status_label = crate::i18n::localize_with_args(
        "tutorial_step_status",
        &[
            ("step", step_num.as_str()),
            ("total", total_num.as_str()),
            ("title", title.as_str()),
        ],
    );

    let card = v_flex()
        .id("tour-card")
        .absolute()
        .left(card_bounds.origin.x)
        .top(card_bounds.origin.y)
        .w_80()
        .p_4()
        .gap_3()
        .bg(cx.theme().popover)
        .border_1()
        .border_color(cx.theme().border)
        .rounded(cx.theme().radius_lg)
        .shadow_lg()
        .child(
            div()
                .id("tour-card-title")
                .a11y(Role::Heading, title.clone())
                .aria_level(2)
                .text_sm()
                .font_weight(FontWeight::SEMIBOLD)
                .child(title),
        )
        .child(
            div()
                .id("tour-card-body")
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(body),
        )
        .child(
            h_flex()
                .id("tour-dots")
                .gap_1()
                .items_center()
                .children((0..total).map(|dot_ix| {
                    div().size_1().rounded_full().bg(if dot_ix == ix {
                        cx.theme().primary
                    } else {
                        cx.theme().muted
                    })
                }))
                .child(div().flex_1())
                .child(
                    div()
                        .id("tour-status")
                        .a11y(Role::Status, status_label)
                        .a11y_live(accesskit::Live::Polite)
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("{}/{}", ix + 1, total)),
                ),
        )
        .child(
            h_flex()
                .gap_2()
                .justify_end()
                .child(
                    Button::new("tour-skip")
                        .ghost()
                        .label(crate::i18n::localize("tutorial_skip"))
                        .on_click(
                            cx.listener(|this, _, window, cx| this.close(window, cx, "skipped")),
                        ),
                )
                .child(div().flex_1())
                .child(
                    Button::new("tour-back")
                        .outline()
                        .disabled(ix == 0)
                        .label(crate::i18n::localize("tutorial_back"))
                        .on_click(cx.listener(|this, _, window, cx| this.go_back(window, cx))),
                )
                .child(
                    Button::new("tour-next")
                        .primary()
                        .label(crate::i18n::localize(if is_last {
                            "tutorial_finish"
                        } else {
                            "tutorial_next"
                        }))
                        .on_click(cx.listener(|this, _, window, cx| this.advance(window, cx))),
                ),
        );

    card.with_animation(
        ("tour-card", ix),
        Animation::new(cx.theme().motion_tokens().duration_fast),
        |card, delta| card.opacity(delta),
    )
}

#[cfg(test)]
#[path = "controller.test.rs"]
mod controller_test;
