//! Removable auth foundation wiring. `install`, `controller`,
//! `handle_callback_link`, and `KeyringStore` are the only public surface.
//! Removal = delete this file, delete crates/gpui-auth/, and revert the seam
//! touches: the members + gpui-auth + keyring-feature lines in Cargo.toml
//! with a regenerated Cargo.lock, the `pub mod auth;` line in
//! services/mod.rs, the `auth` re-export in lib.rs, the install line in
//! app/init.rs, and the two `handle_callback_link` guards in the AppRoot
//! event drain.

use std::sync::Arc;

use gpui_auth::{
    AuthController, AuthError, AuthEvent, AuthIo, AuthState, MockProvider, TokenSet, TokenStore,
};
use gpui_kit::{App, AppContext as _, Entity, Global, Subscription};

use crate::services::session::SessionState;

const KEYRING_SERVICE: &str = "gpui-starter.auth";

/// App-side [`TokenStore`] over `secure_storage`: service
/// `gpui-starter.auth`, key = provider id, value = serde_json(`TokenSet`).
/// A corrupt row reads as a miss (signed out), never a hard failure.
pub struct KeyringStore;

impl TokenStore for KeyringStore {
    fn save(&self, provider_id: &str, tokens: &TokenSet, cx: &mut App) -> Result<(), AuthError> {
        let json =
            serde_json::to_string(tokens).map_err(|err| AuthError::Storage(err.to_string()))?;
        crate::services::secure_storage::set_secret(KEYRING_SERVICE, provider_id, &json, cx)
            .map_err(|err| AuthError::Storage(err.to_string()))
    }

    fn load(&self, provider_id: &str, cx: &mut App) -> Result<Option<TokenSet>, AuthError> {
        let Some(json) =
            crate::services::secure_storage::get_secret(KEYRING_SERVICE, provider_id, cx)
                .map_err(|err| AuthError::Storage(err.to_string()))?
        else {
            return Ok(None);
        };
        match serde_json::from_str::<TokenSet>(&json) {
            Ok(tokens) => Ok(Some(tokens)),
            Err(err) => {
                tracing::warn!(
                    target: "gpui_starter::auth",
                    provider_id,
                    error = %err,
                    "corrupt token row; treating as signed out"
                );
                Ok(None)
            }
        }
    }

    #[cfg(not(target_family = "wasm"))]
    fn delete(&self, provider_id: &str, cx: &mut App) -> Result<(), AuthError> {
        match crate::services::secure_storage::delete_secret(KEYRING_SERVICE, provider_id, cx) {
            Ok(()) => Ok(()),
            // Typed miss detection: the Display text never spells "NoEntry"
            // (keyring-3.6.3/src/error.rs:68), so string matching cannot see it.
            Err(crate::services::secure_storage::SecureStorageError::DeleteFailed {
                source: keyring::Error::NoEntry,
                ..
            }) => Ok(()),
            Err(err) => Err(AuthError::Storage(err.to_string())),
        }
    }

    #[cfg(target_family = "wasm")]
    fn delete(&self, provider_id: &str, cx: &mut App) -> Result<(), AuthError> {
        crate::services::secure_storage::delete_secret(KEYRING_SERVICE, provider_id, cx)
            .map_err(|err| AuthError::Storage(err.to_string()))
    }
}

struct AuthHandleInner {
    controller: Entity<AuthController>,
    _session_observer: Subscription,
    _browser_subscriber: Subscription,
}

pub struct AuthHandle(AuthHandleInner);

impl Global for AuthHandle {}

/// Boot wiring: globals must already be up (tokio runtime, secure storage,
/// session). Idempotent; a second call replaces the handle.
pub fn install(cx: &mut App) {
    #[cfg(not(target_family = "wasm"))]
    let io = match crate::services::tokio_runtime::runtime_and_client(cx) {
        Some((runtime, http)) => AuthIo {
            http,
            runtime: Some(runtime),
        },
        None => {
            tracing::warn!(
                target: "gpui_starter::auth",
                "tokio runtime global absent; auth provider futures run on the GPUI executor"
            );
            AuthIo {
                http: reqwest::Client::new(),
                runtime: None,
            }
        }
    };
    #[cfg(target_family = "wasm")]
    let io = AuthIo {
        http: reqwest::Client::new(),
    };

    #[cfg(not(target_family = "wasm"))]
    let store = Arc::new(KeyringStore);
    #[cfg(target_family = "wasm")]
    let store = Arc::new(gpui_auth::InMemoryStore::new());

    install_with(store, io, cx);
}

pub(crate) fn install_with(store: Arc<dyn TokenStore>, io: AuthIo, cx: &mut App) {
    let provider = Arc::new(MockProvider::new());
    let controller = cx.new(|cx| AuthController::new(provider, store, io, cx));

    let session_observer = cx.observe(&controller, |controller, cx| {
        let mapped = match controller.read(cx).state() {
            AuthState::SignedOut => SessionState::SignedOut,
            AuthState::SigningIn => SessionState::SigningIn,
            AuthState::SignedIn { account_label } => SessionState::SignedIn {
                account_label: account_label.clone(),
            },
            AuthState::Error(message) => SessionState::Error(message.clone()),
        };
        crate::services::session::set_state(mapped, cx);
    });
    let browser_subscriber = cx.subscribe(&controller, |_, event: &AuthEvent, cx| match event {
        AuthEvent::AuthorizationRequested { url } => {
            #[cfg(not(target_family = "wasm"))]
            {
                if let Err(err) = crate::services::desktop_actions::open_url(url, cx) {
                    tracing::warn!(
                        target: "gpui_starter::auth",
                        error = %err,
                        "failed to open authorization URL"
                    );
                }
            }
            #[cfg(target_family = "wasm")]
            cx.open_url(url);
        }
        AuthEvent::FlowFailed { error } => {
            tracing::warn!(target: "gpui_starter::auth", error = %error, "auth flow failed");
        }
        AuthEvent::FlowCompleted | AuthEvent::SignedOut => {}
    });

    cx.set_global(AuthHandle(AuthHandleInner {
        controller: controller.clone(),
        _session_observer: session_observer,
        _browser_subscriber: browser_subscriber,
    }));
    set_capabilities(cx);
    // After the wiring above: restore's notify must reach the session
    // observer, and it flushes at this update's boundary.
    controller.update(cx, |controller, cx| controller.restore(cx));
}

fn set_capabilities(cx: &mut App) {
    crate::capabilities::set(
        "auth_email",
        crate::capabilities::CapabilityStatus::supported_enabled(),
        cx,
    );
    #[cfg(not(target_family = "wasm"))]
    crate::capabilities::set(
        "auth_social",
        crate::capabilities::CapabilityStatus::supported_enabled(),
        cx,
    );
    #[cfg(target_family = "wasm")]
    crate::capabilities::set(
        "auth_social",
        crate::capabilities::CapabilityStatus {
            supported: false,
            enabled: true,
            degraded: true,
            reason: Some(
                "social login needs the web redirect path from plan 23; browser loopback flows do not run on wasm"
                    .into(),
            ),
            last_error: None,
        },
        cx,
    );
}

pub fn controller(cx: &App) -> Option<Entity<AuthController>> {
    cx.try_global::<AuthHandle>()
        .map(|handle| handle.0.controller.clone())
}

/// Consume `gpui-starter://auth/callback` links. Returns true when the link
/// was routed to the controller (caller skips route parsing).
pub fn handle_callback_link(link: &str, cx: &mut App) -> bool {
    let Ok(url) = url::Url::parse(link) else {
        return false;
    };
    if url.scheme() != crate::routes::APP_URL_SCHEME || url.host_str() != Some("auth") {
        return false;
    }
    let is_callback = url
        .path_segments()
        .map(|mut segments| segments.find(|segment| !segment.is_empty()) == Some("callback"))
        .unwrap_or(false);
    if !is_callback {
        return false;
    }
    match controller(cx) {
        Some(entity) => {
            entity.update(cx, |controller, cx| controller.complete_callback(link, cx));
            true
        }
        None => {
            tracing::warn!(
                target: "gpui_starter::auth",
                "auth callback link arrived before auth install; falling through to routing"
            );
            false
        }
    }
}

#[cfg(test)]
#[path = "auth.test.rs"]
mod auth_test;
