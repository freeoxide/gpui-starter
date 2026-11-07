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
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        cx.observe_global::<FloatingWindowRegistry>(|_, cx| {
            cx.notify();
        })
        .detach();
        // Pane bounds are resolved once at spawn; a resized window must
        // re-clamp them, so every bounds change re-renders the host.
        cx.observe_window_bounds(window, |_, _, cx| cx.notify())
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
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let panes = registry::overlay_panes(cx);
        self.empty = panes.is_empty();
        for pane in &panes {
            if self.pane_handles.contains_key(&pane.id) {
                continue;
            }
            let handle = cx.focus_handle();
            if pane.focus {
                let focus = handle.clone();
                // Deferred one frame like the command palette; per-frame
                // re-focus would fight the pane's own content.
                window.defer(cx, move |window, cx| focus.focus(window, cx));
            }
            self.pane_handles.insert(pane.id.clone(), handle);
        }
        let dropped: Vec<FocusHandle> = self
            .pane_handles
            .iter()
            .filter(|(id, _)| !panes.iter().any(|pane| pane.id == **id))
            .map(|(_, handle)| handle.clone())
            .collect();
        self.pane_handles
            .retain(|id, _| panes.iter().any(|pane| pane.id == *id));
        // A pane that dies holding the keyboard strands Esc; the host root
        // takes the focus back so keystrokes never end up nowhere.
        for handle in dropped {
            if handle.is_focused(window) {
                self.focus_handle.focus(window, cx);
            }
        }

        let viewport = window.viewport_size();
        div().absolute().top_0().left_0().size_full().children(
            panes
                .into_iter()
                .map(|pane| self.render_pane(pane, viewport, cx)),
        )
    }
}

impl OverlayFloatHost {
    fn render_pane(&self, pane: OverlayPane, viewport: Size<Pixels>, cx: &App) -> AnyElement {
        let id = pane.id.clone();
        let origin = clamped_origin(pane.bounds, viewport);
        let body = div().flex_1().min_h_0().overflow_hidden().child(pane.view);
        let mut pane_el = v_flex()
            .id(SharedString::from(format!("floating-pane-{}", pane.id.0)))
            .absolute()
            .top(origin.y)
            .left(origin.x)
            .w(pane.bounds.size.width)
            .h(pane.bounds.size.height)
            .key_context("floating")
            .bg(cx.theme().popover)
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .shadow_md()
            .overflow_hidden()
            // Scroll, hover and cursor stop here, not on the page behind.
            .occlude()
            .on_action(move |_: &DismissFloating, _, cx| {
                registry::remove_entry(&id, cx);
            });
        // Chromeless panes keep the overlay surface and Esc dismissal; only
        // the header strip goes away.
        if pane.chromeless {
            pane_el = pane_el.child(body);
        } else {
            pane_el = pane_el.child(float_header(&pane.title, cx)).child(body);
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

/// Pane bounds are spawn-time facts; a shrunk window pulls stranded origins
/// back so the pane stays fully inside the viewport.
fn clamped_origin(bounds: Bounds<Pixels>, viewport: Size<Pixels>) -> Point<Pixels> {
    let max_x = (viewport.width - bounds.size.width).max(px(0.));
    let max_y = (viewport.height - bounds.size.height).max(px(0.));
    point(
        bounds.origin.x.max(px(0.)).min(max_x),
        bounds.origin.y.max(px(0.)).min(max_y),
    )
}

#[cfg(test)]
#[path = "overlay_host.test.rs"]
mod overlay_host_test;
