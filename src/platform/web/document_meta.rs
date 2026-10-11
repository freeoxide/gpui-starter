//! Browser-tab tray equivalent: canvas favicon with unread/connectivity badge
//! plus a `document.title` mirror, driven by the app globals.

use std::cell::RefCell;
use std::sync::Mutex;

use gpui_kit::App;
use wasm_bindgen::{JsCast as _, prelude::Closure};

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

/// Asset-source path of the embedded 64px branded PNG drawn as canvas base.
const BASE_ICON_PATH: &str = "app-icon/hicolor/64x64/apps/gpui-starter.png";

// HtmlImageElement is !Send, so unlike LAST it cannot live in a static
// Mutex; thread-local storage matches the single JS thread.
thread_local! {
    static BASE_IMAGE: RefCell<Option<web_sys::HtmlImageElement>> = RefCell::new(None);
}

pub fn install(cx: &mut App) {
    load_base_image(cx);
    update(cx);
    cx.observe_global::<NotificationInboxState>(|cx| update(cx))
        .detach();
    cx.observe_global::<ConnectivitySnapshot>(|cx| update(cx))
        .detach();
    cx.observe_global::<crate::app_state::AppState>(|cx| update(cx))
        .detach();
}

/// Start the one-time branded-base load (embedded PNG -> blob URL -> image
/// element); completion is async and lands via [`render_favicon`].
fn load_base_image(cx: &App) {
    if BASE_IMAGE.with(|cell| cell.borrow().is_some()) {
        return;
    }
    let Some(window) = web_sys::window() else {
        tracing::warn!(target: LOG, "no window; favicon base not loaded");
        return;
    };
    let Some(document) = window.document() else {
        tracing::warn!(target: LOG, "no document; favicon base not loaded");
        return;
    };
    let Some(bytes) = cx.asset_source().load(BASE_ICON_PATH).ok().flatten() else {
        tracing::warn!(target: LOG, "favicon base missing from asset source");
        return;
    };
    let Ok(element) = document.create_element("img") else {
        tracing::warn!(target: LOG, "img element creation failed");
        return;
    };
    let Ok(image) = element.dyn_into::<web_sys::HtmlImageElement>() else {
        tracing::warn!(target: LOG, "img element cast failed");
        return;
    };
    let parts = js_sys::Array::of1(&js_sys::Uint8Array::from(&bytes[..]));
    let seq: &wasm_bindgen::JsValue = parts.as_ref();
    let Ok(blob) = web_sys::Blob::new_with_u8_slice_sequence(seq) else {
        tracing::warn!(target: LOG, "favicon base blob creation failed");
        return;
    };
    let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) else {
        tracing::warn!(target: LOG, "favicon base object URL failed");
        return;
    };

    let target: &web_sys::EventTarget = image.as_ref();
    let loaded = image.clone();
    let load_url = url.clone();
    let on_load: Closure<dyn FnMut()> = Closure::new(move || {
        let _ = web_sys::Url::revoke_object_url(&load_url);
        let base = loaded.clone();
        BASE_IMAGE.with(|cell| *cell.borrow_mut() = Some(base));
        super::dispatch::dispatch(|cx| {
            if let Ok(mut last) = LAST.lock() {
                *last = None;
            }
            update(cx);
        });
    });
    let fail_url = url.clone();
    let on_error: Closure<dyn FnMut()> = Closure::new(move || {
        let _ = web_sys::Url::revoke_object_url(&fail_url);
        tracing::warn!(target: LOG, "favicon base image failed to load");
    });
    for (event, handler) in [("load", on_load), ("error", on_error)] {
        let registered = target
            .add_event_listener_with_callback(event, handler.as_ref().unchecked_ref())
            .is_ok();
        if registered {
            // Leaked intentionally: must outlive the one-shot load event
            // (house pattern; see connectivity.rs).
            handler.forget();
        } else {
            tracing::warn!(target: LOG, event, "favicon base listener not installed");
        }
    }
    image.set_src(&url);
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
    // None until the branded base loads: the static icon <link> then keeps
    // serving the tab instead of a synthetic canvas mark.
    let image = BASE_IMAGE.with(|cell| cell.borrow().clone())?;
    let document = web_sys::window()?.document()?;
    let canvas: web_sys::HtmlCanvasElement =
        document.create_element("canvas").ok()?.dyn_into().ok()?;
    canvas.set_width(ICON_SIZE);
    canvas.set_height(ICON_SIZE);
    let ctx: web_sys::CanvasRenderingContext2d = canvas.get_context("2d").ok()??.dyn_into().ok()?;
    let size = f64::from(ICON_SIZE);

    ctx.draw_image_with_html_image_element(&image, 0.0, 0.0)
        .ok()?;

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
