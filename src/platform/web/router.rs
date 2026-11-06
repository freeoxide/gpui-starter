//! Hash deep links: `location.hash` ↔ [`AppRoute`] with History API sync;
//! every movement funnels into Navigate events or history entries.

use std::sync::Mutex;

use wasm_bindgen::{JsCast as _, JsValue, prelude::Closure};

use crate::{events::AppEventKind, routes::AppRoute};

const LOG: &str = "gpui_starter::web::router";

/// Last hash this module pushed or dispatched. Browsers fire `hashchange`
/// and `popstate` for one back/forward traversal; the second event is a no-op.
static LAST_HASH: Mutex<Option<String>> = Mutex::new(None);

/// Install hash routing at app init; an empty hash means no deep link and the
/// configured startup route wins.
pub fn install() {
    let hash = current_hash();
    if hash.is_empty() || hash == "#" {
        tracing::debug!(
            target: LOG,
            "no location.hash at boot; keeping configured route"
        );
    } else {
        match AppRoute::from_hash(&hash) {
            Ok(route) => {
                replace_history(&route);
                tracing::info!(
                    target: LOG,
                    hash = %hash,
                    route = ?route,
                    "booting from hash deep link"
                );
                // Queue, not inline: correct whether or not the window (and
                // its Navigate observer) exists yet.
                super::dispatch::dispatch(move |cx| {
                    crate::app_state::update_config(cx, |config| {
                        config.active_route = route.clone();
                    });
                    crate::events::emit(AppEventKind::Navigate(route), cx);
                });
            }
            Err(err) => {
                tracing::warn!(
                    target: LOG,
                    hash = %hash,
                    error = %err,
                    "invalid location.hash at boot; keeping configured route"
                );
            }
        }
    }
    install_listeners();
}

/// Wasm arm of `AppRoot::set_route`: push only when the hash differs, since
/// external navigation has already moved history.
pub fn push_route_hash(route: &AppRoute) {
    let hash = route.to_hash();
    if hash == current_hash() {
        return;
    }
    let url = current_url_with_hash(&hash);
    match history() {
        Ok(history) => {
            if let Err(err) = history.push_state_with_url(&JsValue::NULL, "", Some(&url)) {
                tracing::warn!(
                    target: LOG,
                    error = ?err,
                    hash = %hash,
                    "history.pushState failed"
                );
            } else if let Ok(mut last) = LAST_HASH.lock() {
                *last = Some(hash);
            }
        }
        Err(err) => tracing::warn!(target: LOG, error = ?err, "history API unavailable"),
    }
}

fn install_listeners() {
    let Some(window) = web_sys::window() else {
        tracing::warn!(target: LOG, "no window; hash routing listeners not installed");
        return;
    };
    let target: &web_sys::EventTarget = window.as_ref();
    for event in ["hashchange", "popstate"] {
        let handler: Closure<dyn FnMut(JsValue)> =
            Closure::new(|_event: JsValue| on_external_navigation());
        let registered = target
            .add_event_listener_with_callback(event, handler.as_ref().unchecked_ref())
            .is_ok();
        if registered {
            // Leaked for the page lifetime (see web::connectivity).
            handler.forget();
        } else {
            tracing::warn!(target: LOG, event, "could not install hash routing listener");
        }
    }
}

/// External URL movement (back/forward/typed hash): parse the new hash and
/// emit a Navigate event through the dispatch queue.
fn on_external_navigation() {
    let hash = current_hash();
    let redispatched = matches!(
        LAST_HASH.lock().as_deref(),
        Ok(Some(last)) if *last == hash
    );
    if redispatched {
        return;
    }
    if let Ok(mut last) = LAST_HASH.lock() {
        *last = Some(hash.clone());
    }
    match AppRoute::from_hash(&hash) {
        Ok(route) => {
            tracing::info!(
                target: LOG,
                hash = %hash,
                route = ?route,
                "external hash navigation"
            );
            super::dispatch::dispatch(move |cx| {
                crate::events::emit(AppEventKind::Navigate(route), cx);
            });
        }
        Err(err) => {
            tracing::warn!(
                target: LOG,
                hash = %hash,
                error = %err,
                "ignoring invalid location.hash change"
            );
        }
    }
}

fn replace_history(route: &AppRoute) {
    let url = current_url_with_hash(&route.to_hash());
    match history() {
        Ok(history) => {
            if let Err(err) = history.replace_state_with_url(&JsValue::NULL, "", Some(&url)) {
                tracing::warn!(
                    target: LOG,
                    error = ?err,
                    "history.replaceState failed (boot URL not canonicalized)"
                );
            }
        }
        Err(err) => tracing::warn!(target: LOG, error = ?err, "history API unavailable"),
    }
}

fn history() -> Result<web_sys::History, JsValue> {
    web_sys::window()
        .ok_or_else(|| JsValue::from_str("no window"))
        .and_then(|window| window.history())
}

/// Current `location.hash` including the leading `#` (empty when absent).
fn current_hash() -> String {
    web_sys::window()
        .and_then(|window| {
            window
                .location()
                .hash()
                .ok()
                .map(|hash| hash.trim().to_string())
        })
        .unwrap_or_default()
}

/// Absolute-path URL (pathname + search + new hash) for push/replaceState.
fn current_url_with_hash(hash: &str) -> String {
    let Some(window) = web_sys::window() else {
        return hash.to_string();
    };
    let location = window.location();
    let pathname = location.pathname().unwrap_or_else(|_| "/".into());
    let search = location.search().unwrap_or_default();
    format!("{pathname}{search}{hash}")
}
