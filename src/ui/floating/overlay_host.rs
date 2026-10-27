use std::collections::HashMap;

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
            })
            .child(float_header(&pane.id, &pane.title, cx))
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .overflow_hidden()
                    .child(pane.view.clone()),
            );
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
