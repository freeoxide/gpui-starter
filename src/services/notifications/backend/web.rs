//! Wasm notification backend: the browser Web Notifications API behind the
//! shared [`NotificationBackend`] trait (Send-safe via `flume` of `String`s).

use async_trait::async_trait;
use js_sys::Reflect;
use wasm_bindgen::{JsValue, prelude::Closure};

use super::{NotificationBackend, web_permission_to_state, web_tag_for_request};
use crate::notifications::{
    NotificationBackendKind, NotificationCapabilities, NotificationPermissionState,
    NotificationRequest,
};

const LOG: &str = "gpui_starter::notifications::web";

/// Notification icon served by the wasm harness; if the file is missing the
/// browser silently falls back to its default icon — never an error.
const NOTIFICATION_ICON_URL: &str = "/icons/icon-192.png";

pub struct WebNotificationBackend;

impl WebNotificationBackend {
    /// Construct after probing that the browser exposes the API; `Err(reason)`
    /// becomes the service's degrade reason (very old browsers land here).
    pub fn new() -> Result<Self, String> {
        if notification_constructor().is_some() {
            tracing::info!(target: LOG, "Web Notification API available");
            Ok(Self)
        } else {
            let reason =
                "Web Notification API unavailable (window.Notification is undefined)".to_string();
            tracing::warn!(target: LOG, %reason, "Web Notification API missing");
            Err(reason)
        }
    }
}

#[async_trait]
impl NotificationBackend for WebNotificationBackend {
    fn kind(&self) -> NotificationBackendKind {
        NotificationBackendKind::Web
    }

    fn capabilities(&self) -> NotificationCapabilities {
        // Permission is readable/requestable and a granted constructor shows an
        // OS banner; interactive actions and real push are not advertised.
        NotificationCapabilities {
            can_request_permission: true,
            can_read_permission_state: true,
            can_send_immediate_native: true,
            can_send_interactive: false,
            requires_packaged_runtime: false,
        }
    }

    async fn refresh_permission_state(&self) -> NotificationPermissionState {
        match permission_string() {
            Some(permission) => web_permission_to_state(&permission),
            None => NotificationPermissionState::Unavailable(
                "Web Notification API unavailable".to_string(),
            ),
        }
    }

    async fn request_permission(&self) -> NotificationPermissionState {
        let Some(receiver) = spawn_permission_request() else {
            return NotificationPermissionState::Unavailable(
                "Web Notification API unavailable".to_string(),
            );
        };
        match receiver.recv_async().await {
            Ok(Ok(permission)) => web_permission_to_state(&permission),
            Ok(Err(err)) => {
                NotificationPermissionState::Unavailable(format!("requestPermission: {err}"))
            }
            // Unreachable in practice: the leaked senders keep the channel
            // alive for the page lifetime; mapped defensively anyway.
            Err(_) => NotificationPermissionState::Unknown,
        }
    }

    async fn send(&self, request: &NotificationRequest) -> anyhow::Result<()> {
        if notification_constructor().is_none() {
            anyhow::bail!("Web Notification API unavailable (window.Notification undefined)");
        }
        // Fail fast when unauthorized: constructing a Notification without
        // permission throws a bare TypeError that explains nothing.
        let permission = permission_string()
            .ok_or_else(|| anyhow::anyhow!("web notification permission could not be read"))?;
        if permission != "granted" {
            let state = web_permission_to_state(&permission);
            anyhow::bail!(
                "web notification permission is {} — request it from Settings → Notifications",
                state.label()
            );
        }

        let options = web_sys::NotificationOptions::new();
        web_sys::NotificationOptions::set_body(&options, &request.body);
        web_sys::NotificationOptions::set_icon(&options, NOTIFICATION_ICON_URL);
        if let Some(tag) = web_tag_for_request(request) {
            web_sys::NotificationOptions::set_tag(&options, &tag);
        }
        // Browsers play their default sound unless `silent` is set; forcing a
        // custom sound is not possible on the web.
        if !request.play_sound {
            web_sys::NotificationOptions::set_silent(&options, Some(true));
        }

        match web_sys::Notification::new_with_options(&request.title, &options) {
            // The Rust handle may drop freely: dropping is not `close()` —
            // the banner stays up until the user dismisses it.
            Ok(_notification) => {
                tracing::info!(target: LOG, "web notification shown");
                Ok(())
            }
            Err(err) => {
                let message = js_value_message(&err);
                tracing::warn!(target: LOG, error = %message, "new Notification() failed");
                anyhow::bail!("web notification rejected: {message}");
            }
        }
    }
}

/// The global `Notification` constructor, or `None` without the API (also
/// covers a missing `window`). `Reflect::get` keeps this a pure existence probe.
fn notification_constructor() -> Option<JsValue> {
    let window = web_sys::window()?;
    let constructor =
        Reflect::get(&JsValue::from(window), &JsValue::from_str("Notification")).ok()?;
    if constructor.is_undefined() || constructor.is_null() {
        None
    } else {
        Some(constructor)
    }
}

/// Read `Notification.permission` via `Reflect::get` — the typed getter is
/// gated behind a web-sys feature not in this crate's enabled union.
fn permission_string() -> Option<String> {
    let constructor = notification_constructor()?;
    Reflect::get(&constructor, &JsValue::from_str("permission"))
        .ok()
        .and_then(|value| value.as_string())
}

fn spawn_permission_request() -> Option<flume::Receiver<Result<String, String>>> {
    notification_constructor()?;
    let promise = match web_sys::Notification::request_permission() {
        Ok(promise) => promise,
        Err(err) => {
            tracing::warn!(
                target: LOG,
                error = %js_value_message(&err),
                "requestPermission() call failed"
            );
            return None;
        }
    };

    let (tx, rx) = flume::unbounded::<Result<String, String>>();
    let tx_resolve = tx.clone();
    let on_resolve: Closure<dyn FnMut(JsValue)> = Closure::new(move |value: JsValue| {
        let _ = tx_resolve.send(Ok(value.as_string().unwrap_or_default()));
    });
    let on_reject: Closure<dyn FnMut(JsValue)> = Closure::new(move |err: JsValue| {
        let _ = tx.send(Err(js_value_message(&err)));
    });
    let _ = promise.then2(&on_resolve, &on_reject);
    // Leaked (house pattern) to outlive the frame; the leaked senders also
    // keep the channel alive so the receiver never sees a disconnect.
    on_resolve.forget();
    on_reject.forget();
    Some(rx)
}

fn js_value_message(value: &JsValue) -> String {
    value.as_string().unwrap_or_else(|| format!("{value:?}"))
}
