//! Wasm connectivity bridge: `navigator.onLine` + online/offline events feed
//! the [`ConnectivitySnapshot`] global via the dispatch queue.

use gpui_kit::BorrowAppContext as _;
use wasm_bindgen::{JsCast as _, JsValue, prelude::Closure};

use crate::connectivity::{ConnectivitySnapshot, ConnectivityState};

const LOG: &str = "gpui_starter::web::connectivity";

/// Current `navigator.onLine`; `false` when the API is missing, since a
/// missing signal must not read as connectivity.
pub fn navigator_online() -> bool {
    web_sys::window()
        .map(|window| window.navigator().on_line())
        .unwrap_or(false)
}

/// Install the `online`/`offline` window listeners (from [`super::install`]);
/// each event re-reads `navigator.onLine` and updates the snapshot global.
pub fn install_listeners() {
    let Some(window) = web_sys::window() else {
        tracing::warn!(target: LOG, "no window; connectivity listeners not installed");
        return;
    };
    let target: &web_sys::EventTarget = window.as_ref();
    for event in ["online", "offline"] {
        let handler: Closure<dyn FnMut(JsValue)> = Closure::new(|_event: JsValue| {
            let online = navigator_online();
            super::dispatch::dispatch(move |cx| {
                cx.update_global::<ConnectivitySnapshot, _>(|next, _cx| {
                    next.state = if online {
                        ConnectivityState::Online
                    } else {
                        ConnectivityState::Offline
                    };
                    next.interfaces = Vec::new();
                    next.last_error = if online {
                        None
                    } else {
                        Some("browser reports navigator.onLine = false".to_string())
                    };
                });
                tracing::info!(
                    target: LOG,
                    online,
                    "connectivity snapshot updated from browser event"
                );
            });
        });
        let registered = target
            .add_event_listener_with_callback(event, handler.as_ref().unchecked_ref())
            .is_ok();
        if registered {
            // Leaked intentionally: the listener must live for the page
            // lifetime (house pattern; see the module docs).
            handler.forget();
        } else {
            tracing::warn!(target: LOG, event, "could not install connectivity listener");
        }
    }
}
