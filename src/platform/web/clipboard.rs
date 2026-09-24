//! Wasm clipboard bridge: `navigator.clipboard.writeText`, fire-and-forget —
//! writes dispatch, rejections log, `Ok` returns immediately.

use js_sys::Promise;
use wasm_bindgen::{JsValue, prelude::Closure};

const LOG: &str = "gpui_starter::web::clipboard";

/// Whether `navigator.clipboard` exists (secure contexts only).
pub fn is_available() -> bool {
    clipboard_handle().is_some()
}

pub fn write_text_fire_and_forget(text: &str) {
    let Some(clipboard) = clipboard_handle() else {
        tracing::warn!(
            target: LOG,
            "navigator.clipboard unavailable (insecure context?); text write dropped"
        );
        return;
    };
    log_rejection(&clipboard.write_text(text), "clipboard.writeText");
}

/// `navigator.clipboard`, probed with `Reflect::has`: browsers leave the
/// property `undefined` in insecure contexts despite the web-sys typing.
fn clipboard_handle() -> Option<web_sys::Clipboard> {
    let navigator = web_sys::window()?.navigator();
    let has_clipboard = js_sys::Reflect::has(
        &JsValue::from(navigator.clone()),
        &JsValue::from_str("clipboard"),
    )
    .unwrap_or(false);
    if has_clipboard {
        Some(navigator.clipboard())
    } else {
        None
    }
}

/// Attach a rejection logger; the closure is leaked so it outlives the
/// pending Promise, with nothing to reclaim afterwards.
fn log_rejection(promise: &Promise, what: &'static str) {
    let on_reject: Closure<dyn FnMut(JsValue)> = Closure::new(move |err: JsValue| {
        tracing::warn!(
            target: LOG,
            what,
            error = ?err,
            "browser promise rejected"
        );
    });
    let _ = promise.catch(&on_reject);
    on_reject.forget();
}
