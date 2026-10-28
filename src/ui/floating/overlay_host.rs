use std::collections::{HashMap, HashSet};

use gpui_kit::component::{ActiveTheme as _, v_flex};
use gpui_kit::{prelude::*, *};

use super::DismissFloating;
use super::registry::{self, FloatingWindowRegistry, OverlayPane};
use super::shell::float_header;
use super::spec::FloatingWindowId;

/// In-window overlay layer host; `AppRoot` owns one and renders it last.
pub struct OverlayFloatHost {
    focus_handle: FocusHandle,
    pane_handles: HashMap<FloatingWindowId, FocusHandle>,
    empty: bool,
}

/// Panes registered as chromeless: `OverlayPane` carries no spec flags, so
/// this id set is the spawn-to-host channel, rewritten on each registration.
#[derive(Default)]
struct ChromelessPanes(HashSet<FloatingWindowId>);

impl Global for ChromelessPanes {}

/// Marks a pane chromeless for the host. Wasm-lane only: native floats are
/// never panes, so the setter exists for the overlay registry and tests.
#[cfg(any(test, target_family = "wasm"))]
pub(crate) fn set_pane_chromeless(id: &FloatingWindowId, enabled: bool, cx: &mut App) {
    let panes = cx.default_global::<ChromelessPanes>();
    if enabled {
        panes.0.insert(id.clone());
    } else {
        panes.0.remove(id);
    }
}

fn pane_is_chromeless(id: &FloatingWindowId, cx: &App) -> bool {
    cx.try_global::<ChromelessPanes>()
        .is_some_and(|panes| panes.0.contains(id))
}

impl OverlayFloatHost {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<FloatingWindowRegistry>(|_, cx| {
            cx.notify();
        })
        .detach();
        Self {
            focus_handle: cx.focus_handle(),
            pane_handles: HashMap::new(),
            empty: true,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.empty
    }
}

impl Render for OverlayFloatHost {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let panes = registry::overlay_panes(cx);
        self.empty = panes.is_empty();
        for pane in &panes {
            self.pane_handles
                .entry(pane.id.clone())
                .or_insert_with(|| cx.focus_handle());
        }
        self.pane_handles
            .retain(|id, _| panes.iter().any(|pane| pane.id == *id));

        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .children(panes.into_iter().map(|pane| self.render_pane(pane, cx)))
    }
}

impl OverlayFloatHost {
    fn render_pane(&self, pane: OverlayPane, cx: &App) -> AnyElement {
        let id = pane.id.clone();
        let body = div()
            .flex_1()
            .min_h_0()
            .overflow_hidden()
            .child(pane.view.clone());
        let mut pane_el = v_flex()
            .id(SharedString::from(format!("floating-pane-{}", pane.id.0)))
            .absolute()
            .top(pane.bounds.origin.y)
            .left(pane.bounds.origin.x)
            .w(pane.bounds.size.width)
            .h(pane.bounds.size.height)
            .key_context("floating")
            .bg(cx.theme().popover)
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .shadow_md()
            .overflow_hidden()
            .on_action(move |_: &DismissFloating, _, cx| {
                registry::remove_entry(&id, cx);
            });
        // Chromeless panes keep the overlay surface and Esc dismissal; only
        // the header strip goes away.
        if pane_is_chromeless(&pane.id, cx) {
            pane_el = pane_el.child(body);
        } else {
            pane_el = pane_el
                .child(float_header(&pane.id, &pane.title, cx))
                .child(body);
        }
        if let Some(handle) = self.pane_handles.get(&pane.id) {
            pane_el = pane_el.track_focus(handle);
        }
        pane_el.into_any_element()
    }
}

impl Focusable for OverlayFloatHost {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

#[cfg(test)]
#[path = "overlay_host.test.rs"]
mod overlay_host_test;
