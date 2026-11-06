// No glob `use gpui_kit::*` here: it would import gpui's `test` proc-macro and
// shadow this file's `#[gpui_kit::test]` attribute.
use gpui_kit::prelude::*;
use gpui_kit::{
    AnyWindowHandle, Entity, FocusHandle, TestAppContext, VisualTestContext, Window, div, px, size,
};

use super::{TourController, TourDefinition, TourHandle, TourStep};
use crate::app::window::RootWindow;
use crate::events::AppEventKind;
use crate::page::Page;
use crate::routes::AppRoute;

#[gpui_kit::test]
fn start_without_handle_is_a_noop(cx: &mut TestAppContext) {
    cx.update(crate::features::tutorial::start);
    assert!(
        cx.read(|cx| {
            cx.try_global::<crate::features::tutorial::target::TargetRegistry>()
                .is_none()
        }),
        "a rejected start must not touch the registry global"
    );
}

struct TourHost {
    outside: FocusHandle,
    tour: Entity<TourController>,
}

impl Render for TourHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .track_focus(&self.outside)
            .child(self.tour.clone())
    }
}

fn step(target: &'static str, route: Option<AppRoute>) -> TourStep {
    TourStep {
        target,
        title_key: "tutorial_step_home_heading_title",
        body_key: "tutorial_step_home_heading_body",
        route,
    }
}

fn three_step_tour() -> TourDefinition {
    TourDefinition {
        id: "test".into(),
        steps: vec![
            step("step-a", Some(AppRoute::page(Page::Home))),
            step("step-b", None),
            step("step-c", Some(AppRoute::page(Page::Gallery))),
        ],
    }
}

fn open_tour_window(
    cx: &mut TestAppContext,
) -> (
    Entity<TourController>,
    FocusHandle,
    AnyWindowHandle,
    VisualTestContext,
) {
    cx.update(gpui_kit::init);
    cx.update(crate::features::tutorial::initialize);
    let window = cx.open_window(size(px(800.), px(600.)), |window, cx| {
        let tour = cx.new(|cx| TourController::new(window, cx));
        TourHost {
            outside: cx.focus_handle(),
            tour,
        }
    });
    let host = window.root(cx).unwrap();
    let (tour, outside) = cx.read(|cx| {
        let host = host.read(cx);
        (host.tour.clone(), host.outside.clone())
    });
    let root: AnyWindowHandle = window.into();
    let mut cx = VisualTestContext::from_window(root, cx);
    cx.update(|window, cx| window.draw(cx).clear(cx));
    (tour, outside, root, cx)
}

fn active_ix(tour: &Entity<TourController>, cx: &VisualTestContext) -> Option<usize> {
    cx.read(|cx| {
        tour.read(cx)
            .active
            .as_ref()
            .map(|active| active.progress.ix())
    })
}

fn active_total(tour: &Entity<TourController>, cx: &VisualTestContext) -> Option<usize> {
    cx.read(|cx| {
        tour.read(cx)
            .active
            .as_ref()
            .map(|active| active.progress.total())
    })
}

fn navigated_routes(cx: &mut VisualTestContext) -> Vec<AppRoute> {
    cx.update(|_, cx| crate::events::drain(cx))
        .into_iter()
        .filter_map(|event| match event.kind {
            AppEventKind::Navigate(route) => Some(route),
            _ => None,
        })
        .collect()
}

#[gpui_kit::test]
fn begin_rejects_empty_tours_and_repeated_begins(cx: &mut TestAppContext) {
    let (tour, _, _, mut cx) = open_tour_window(cx);

    let empty = TourDefinition {
        id: "empty".into(),
        steps: Vec::new(),
    };
    cx.update(|window, cx| tour.update(cx, |tour, cx| tour.begin(empty, window, cx)));
    assert_eq!(
        active_ix(&tour, &cx),
        None,
        "an empty tour must be rejected"
    );

    cx.update(|window, cx| tour.update(cx, |tour, cx| tour.begin(three_step_tour(), window, cx)));
    assert_eq!(active_ix(&tour, &cx), Some(0));

    let five = TourDefinition {
        id: "other".into(),
        steps: vec![step("other", None); 5],
    };
    cx.update(|window, cx| tour.update(cx, |tour, cx| tour.begin(five, window, cx)));
    assert_eq!(
        active_total(&tour, &cx),
        Some(3),
        "a second begin while a tour runs must be ignored"
    );
}

#[gpui_kit::test]
fn advance_saturates_closes_on_last_and_restores_focus(cx: &mut TestAppContext) {
    let (tour, outside, _, mut cx) = open_tour_window(cx);

    cx.update(|window, cx| window.focus(&outside, cx));
    cx.update(|window, cx| tour.update(cx, |tour, cx| tour.begin(three_step_tour(), window, cx)));

    let tour_focus = cx.read(|cx| tour.read(cx).focus_handle.clone());
    assert_eq!(
        cx.update(|window, cx| window.focused(cx)),
        Some(tour_focus),
        "begin must move focus into the overlay"
    );
    assert_eq!(
        navigated_routes(&mut cx),
        vec![AppRoute::page(Page::Home)],
        "entering the first step must navigate its route"
    );

    cx.update(|window, cx| tour.update(cx, |tour, cx| tour.advance(window, cx)));
    assert_eq!(active_ix(&tour, &cx), Some(1));
    assert!(
        navigated_routes(&mut cx).is_empty(),
        "a step without route must not navigate"
    );

    cx.update(|window, cx| tour.update(cx, |tour, cx| tour.go_back(window, cx)));
    assert_eq!(active_ix(&tour, &cx), Some(0));
    assert_eq!(
        navigated_routes(&mut cx),
        vec![AppRoute::page(Page::Home)],
        "re-entering a routed step must navigate again"
    );
    cx.update(|window, cx| tour.update(cx, |tour, cx| tour.go_back(window, cx)));
    assert_eq!(
        active_ix(&tour, &cx),
        Some(0),
        "go_back must saturate on the first step"
    );

    cx.update(|window, cx| tour.update(cx, |tour, cx| tour.advance(window, cx)));
    cx.update(|window, cx| tour.update(cx, |tour, cx| tour.advance(window, cx)));
    assert_eq!(active_ix(&tour, &cx), Some(2));
    assert_eq!(
        navigated_routes(&mut cx),
        vec![AppRoute::page(Page::Gallery)],
        "entering the routed third step must navigate"
    );

    cx.update(|window, cx| tour.update(cx, |tour, cx| tour.advance(window, cx)));
    assert_eq!(
        active_ix(&tour, &cx),
        None,
        "advancing on the last step must close the tour"
    );
    assert_eq!(
        cx.update(|window, cx| window.focused(cx)),
        Some(outside),
        "closing must restore the pre-tour focus"
    );
}

#[gpui_kit::test]
fn start_runs_the_demo_tour_in_the_root_window(cx: &mut TestAppContext) {
    let (tour, _, root, mut cx) = open_tour_window(cx);

    // start re-enters the window via RootWindow; a window-scoped update here
    // would find the slot already taken and report the window missing.
    let app: &TestAppContext = &cx;
    app.update(|cx| {
        cx.set_global(TourHandle(tour.clone()));
        cx.set_global(RootWindow(root));
        super::start(cx);
    });

    assert_eq!(active_ix(&tour, &cx), Some(0));
    assert_eq!(
        active_total(&tour, &cx),
        Some(5),
        "the demo tour has five steps"
    );
    assert_eq!(
        navigated_routes(&mut cx),
        vec![AppRoute::page(Page::Home)],
        "the demo tour's first step navigates Home"
    );
}

#[gpui_kit::test]
async fn tour_keystrokes_drive_the_focused_overlay(cx: &mut TestAppContext) {
    let (tour, outside, _, mut cx) = open_tour_window(cx);

    cx.update(|window, cx| window.focus(&outside, cx));
    cx.update(|window, cx| tour.update(cx, |tour, cx| tour.begin(three_step_tour(), window, cx)));
    // The overlay (key context, focus trap, action handlers) only exists in
    // the drawn tree once a tour is active.
    cx.update(|window, cx| window.draw(cx).clear(cx));

    cx.simulate_keystrokes("right");
    assert_eq!(
        active_ix(&tour, &cx),
        Some(1),
        "right must advance through TourNext"
    );

    cx.simulate_keystrokes("left");
    assert_eq!(
        active_ix(&tour, &cx),
        Some(0),
        "left must go back through TourBack"
    );

    cx.simulate_keystrokes("escape");
    assert_eq!(
        active_ix(&tour, &cx),
        None,
        "escape must dismiss through TourDismiss"
    );
    assert_eq!(
        cx.update(|window, cx| window.focused(cx)),
        Some(outside),
        "the dismissed tour must restore the pre-tour focus"
    );
}
