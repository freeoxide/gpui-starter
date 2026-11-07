//! Wasm connectivity bridge: `navigator.onLine` + online/offline events feed
//! the [`crate::connectivity::ConnectivitySnapshot`] global via the dispatch queue.

use wasm_bindgen::{JsCast as _, JsValue, prelude::Closure};

const LOG: &str = "gpui_starter::web::connectivity";

/// Current `navigator.onLine`; `false` when the API is missing, since a
/// missing signal must not read as connectivity.
pub fn navigator_online() -> bool {
    web_sys::window()
        .map(|window| window.navigator().on_line())
        .unwrap_or(false)
}

/// Install the `online`/`offline` window listeners (from [`super::install`]);
/// each event funnels into `services::connectivity::check_now`.
pub fn install_listeners() {
    let Some(window) = web_sys::window() else {
        tracing::warn!(target: LOG, "no window; connectivity listeners not installed");
        return;
    };
    let target: &web_sys::EventTarget = window.as_ref();
    for event in ["online", "offline"] {
        let handler: Closure<dyn FnMut(JsValue)> = Closure::new(|_event: JsValue| {
            super::dispatch::dispatch(crate::connectivity::check_now);
        });
        let registered = target
            .add_event_listener_with_callback(event, handler.as_ref().unchecked_ref())
            .is_ok();
        if registered {
            // Leaked for the page lifetime: the listener stays registered
            // until the page unloads, so the closure must never be freed.
            handler.forget();
        } else {
            tracing::warn!(target: LOG, event, "could not install connectivity listener");
        }
    }
}
