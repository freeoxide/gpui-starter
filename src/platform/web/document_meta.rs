//! Browser-tab tray equivalent: canvas favicon with unread/connectivity badge
//! plus a `document.title` mirror, driven by the app globals.

use std::sync::Mutex;

use gpui_kit::App;
use wasm_bindgen::JsCast as _;

use crate::{
    connectivity::{ConnectivitySnapshot, ConnectivityState},
    notifications::inbox::NotificationInboxState,
    routes::AppRoute,
};

const LOG: &str = "gpui_starter::web::document_meta";

/// Attribute marking the favicon `<link>` this module owns.
const FAVICON_MARKER: &str = "data-app-favicon";

/// Canvas side length in pixels (browsers downscale as needed).
const ICON_SIZE: u32 = 64;

/// Last applied favicon inputs — skips canvas redraws on no-op updates.
static LAST: Mutex<Option<(usize, ConnectivityState)>> = Mutex::new(None);

pub fn install(cx: &mut App) {
    update(cx);
    cx.observe_global::<NotificationInboxState>(|cx| update(cx))
        .detach();
    cx.observe_global::<ConnectivitySnapshot>(|cx| update(cx))
        .detach();
    cx.observe_global::<crate::app_state::AppState>(|cx| update(cx))
        .detach();
}

/// Recompute title + favicon from the current globals and apply them.
fn update(cx: &App) {
    let route_title = crate::app_state::config_handle(cx)
        .map(|config| config.active_route.localized_title())
        .unwrap_or_else(|| AppRoute::default().localized_title());
    let unread = crate::notifications::inbox::unread_count(cx);
    let state = crate::connectivity::snapshot(cx).state;

    set_title(&compose_title(&route_title, unread, &state));

    let changed = match LAST.lock().as_deref() {
        Ok(Some(last)) => *last != (unread, state.clone()),
        _ => true,
    };
    if !changed {
        return;
    }
    if let Some(data_url) = render_favicon(unread, &state) {
        apply_favicon(&data_url);
    }
    if let Ok(mut last) = LAST.lock() {
        *last = Some((unread, state));
    }
}

/// `"<app> — <route> [(unread)] [— Offline|Filtered]"`.
fn compose_title(route_title: &str, unread: usize, state: &ConnectivityState) -> String {
    let mut title = format!("{} — {}", env!("CARGO_PKG_NAME"), route_title);
    if unread > 0 {
        title.push_str(&format!(" ({unread})"));
    }
    match state {
        ConnectivityState::Offline => {
            title.push_str(&format!(
                " — {}",
                crate::i18n::localize("tab_title_offline")
            ));
        }
        ConnectivityState::CaptiveOrFiltered => {
            title.push_str(&format!(
                " — {}",
                crate::i18n::localize("tab_title_filtered")
            ));
        }
        _ => {}
    }
    title
}

fn set_title(title: &str) {
    if let Some(document) = web_sys::window().and_then(|window| window.document()) {
        document.set_title(title);
    }
}

fn render_favicon(unread: usize, state: &ConnectivityState) -> Option<String> {
    let document = web_sys::window()?.document()?;
    let canvas: web_sys::HtmlCanvasElement =
        document.create_element("canvas").ok()?.dyn_into().ok()?;
    canvas.set_width(ICON_SIZE);
    canvas.set_height(ICON_SIZE);
    let ctx: web_sys::CanvasRenderingContext2d = canvas.get_context("2d").ok()??.dyn_into().ok()?;
    let size = f64::from(ICON_SIZE);

    // Base square (app blue).
    ctx.set_fill_style_str("#2563eb");
    ctx.fill_rect(0.0, 0.0, size, size);

    // Connectivity dot, bottom-right, white ring for contrast on any tab bar.
    let dot = state_color(state);
    ctx.begin_path();
    ctx.set_fill_style_str("#ffffff");
    ctx.arc(size - 15.0, size - 15.0, 13.0, 0.0, std::f64::consts::TAU)
        .ok()?;
    ctx.close_path();
    ctx.fill();
    ctx.begin_path();
    ctx.set_fill_style_str(dot);
    ctx.arc(size - 15.0, size - 15.0, 10.0, 0.0, std::f64::consts::TAU)
        .ok()?;
    ctx.close_path();
    ctx.fill();

    // Unread badge, top-right (red disc + white count).
    if unread > 0 {
        let (label, font) = if unread > 99 {
            ("99+".to_string(), "bold 13px sans-serif")
        } else {
            (unread.to_string(), "bold 17px sans-serif")
        };
        let (cx_, cy_) = (size - 17.0, 17.0);
        ctx.begin_path();
        ctx.set_fill_style_str("#ffffff");
        ctx.arc(cx_, cy_, 16.0, 0.0, std::f64::consts::TAU).ok()?;
        ctx.close_path();
        ctx.fill();
        ctx.begin_path();
        ctx.set_fill_style_str("#dc2626");
        ctx.arc(cx_, cy_, 14.0, 0.0, std::f64::consts::TAU).ok()?;
        ctx.close_path();
        ctx.fill();
        ctx.set_font(font);
        ctx.set_text_align("center");
        ctx.set_text_baseline("middle");
        ctx.set_fill_style_str("#ffffff");
        let _ = ctx.fill_text(&label, cx_, cy_ + 1.0);
    }

    canvas.to_data_url().ok()
}

/// Create (or reuse) the `<link rel="icon">` and point it at `data_url`.
fn apply_favicon(data_url: &str) {
    let Some(document) = web_sys::window().and_then(|window| window.document()) else {
        tracing::debug!(target: LOG, "no document; favicon not applied");
        return;
    };
    let selector = format!("link[{}]", FAVICON_MARKER);
    let link: web_sys::HtmlLinkElement = match document
        .query_selector(&selector)
        .ok()
        .flatten()
        .and_then(|element| element.dyn_into::<web_sys::HtmlLinkElement>().ok())
    {
        Some(link) => link,
        None => {
            let Some(head) = document.head() else {
                tracing::warn!(target: LOG, "no <head>; favicon link not created");
                return;
            };
            let Ok(element) = document.create_element("link") else {
                tracing::warn!(target: LOG, "link element creation failed");
                return;
            };
            let Ok(link) = element.dyn_into::<web_sys::HtmlLinkElement>() else {
                tracing::warn!(target: LOG, "link element cast failed");
                return;
            };
            link.set_rel("icon");
            link.set_type("image/png");
            let element: &web_sys::Element = link.as_ref();
            let _ = element.set_attribute(FAVICON_MARKER, "1");
            let head_node: &web_sys::Node = head.as_ref();
            let link_node: &web_sys::Node = link.as_ref();
            if head_node.append_child(link_node).is_err() {
                tracing::warn!(target: LOG, "appending favicon link failed");
                return;
            }
            link
        }
    };
    link.set_href(data_url);
}

fn state_color(state: &ConnectivityState) -> &'static str {
    match state {
        ConnectivityState::Online => "#22c55e",
        ConnectivityState::Offline => "#ef4444",
        ConnectivityState::CaptiveOrFiltered => "#f59e0b",
        ConnectivityState::Unknown => "#94a3b8",
    }
}
