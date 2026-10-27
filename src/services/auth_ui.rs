//! Read-side auth status mirror for the UI. Removal = delete this file and
//! auth_ui.test.rs, then revert the seam touches: the `pub mod auth_ui;` line
//! in services/mod.rs, the `auth_ui` re-export in lib.rs, the initialize line
//! in app/init.rs, the observers and rows in features/pages/diagnostics, the
//! session section observer in features/pages/settings, the SignOut command
//! availability arm in services/commands.rs, and the account menu + sign-in
//! page reads (see their own headers).

use gpui_auth::{AuthEvent, AuthState, TokenStore as _};
use gpui_kit::{App, BorrowAppContext as _, Global, Subscription};

pub const STORE_BACKEND_KEYRING: &str = "OS keyring (gpui-starter.auth)";
pub const STORE_BACKEND_MEMORY: &str = "in-memory (session only)";
const MOCK_AUTHORIZER_PREFIX: &str = "urn:gpui-auth:mock:";

/// Read-only clone of the auth state plus derived account facts. The
/// `demo_authorize_url` carries the mock flow's `state`, so it is memory-only
/// and never appears in a log macro.
#[derive(Clone, Debug)]
pub struct AuthUiSnapshot {
    pub state: AuthState,
    pub provider_id: String,
    pub account_label: Option<String>,
    pub account_email: Option<String>,
    pub expires_at: Option<i64>,
    pub store_backend: &'static str,
    pub demo_authorize_url: Option<String>,
}

impl Default for AuthUiSnapshot {
    fn default() -> Self {
        Self {
            state: AuthState::SignedOut,
            provider_id: String::new(),
            account_label: None,
            account_email: None,
            expires_at: None,
            store_backend: default_backend(),
            demo_authorize_url: None,
        }
    }
}

fn default_backend() -> &'static str {
    if cfg!(target_family = "wasm") {
        STORE_BACKEND_MEMORY
    } else {
        STORE_BACKEND_KEYRING
    }
}

pub struct AuthUiState {
    snapshot: AuthUiSnapshot,
    _subscriptions: Vec<Subscription>,
}

impl Global for AuthUiState {}

/// Boot wiring; must run after `services::auth::install` so the controller
/// entity exists. Idempotent and safe when the controller is absent.
pub fn initialize(cx: &mut App) {
    let mut subscriptions = Vec::new();
    match crate::services::auth::controller(cx) {
        Some(entity) => {
            subscriptions.push(cx.observe(&entity, |_, cx| refresh(cx)));
            subscriptions.push(cx.subscribe(&entity, |_, event: &AuthEvent, cx| {
                if let AuthEvent::AuthorizationRequested { url } = event {
                    cx.update_global::<AuthUiState, _>(|state, _| {
                        state.snapshot.demo_authorize_url = Some(url.clone());
                    });
                    refresh(cx);
                }
            }));
        }
        None => {
            tracing::warn!(
                target: "gpui_starter::auth_ui",
                "auth controller absent; auth surfaces keep the default snapshot"
            );
        }
    }
    cx.set_global(AuthUiState {
        snapshot: AuthUiSnapshot::default(),
        _subscriptions: subscriptions,
    });
    if crate::services::auth::controller(cx).is_some() {
        refresh(cx);
    }
}

pub fn snapshot(cx: &App) -> AuthUiSnapshot {
    cx.try_global::<AuthUiState>()
        .map(|state| state.snapshot.clone())
        .unwrap_or_default()
}

/// RFC3339 (UTC) for an expiry, or None; callers localize the None case.
pub fn format_expiry(expires_at: Option<i64>) -> Option<String> {
    expires_at
        .and_then(|secs| chrono::DateTime::from_timestamp(secs, 0))
        .map(|datetime| datetime.to_rfc3339())
}

/// Route the captured mock authorize URL back through
/// `auth::handle_callback_link`; false when no mock flow is pending.
pub fn complete_demo_sign_in(cx: &mut App) -> bool {
    let Some(url) = snapshot(cx).demo_authorize_url else {
        return false;
    };
    let Some(state) = url.strip_prefix(MOCK_AUTHORIZER_PREFIX) else {
        return false;
    };
    let link = format!("gpui-starter://auth/callback?code=demo&state={state}");
    crate::services::auth::handle_callback_link(&link, cx)
}

fn refresh(cx: &mut App) {
    let Some(next) = next_snapshot(cx) else {
        return;
    };
    tracing::info!(
        target: "gpui_starter::auth_ui",
        state = ?next.state,
        provider_id = %next.provider_id,
        "auth snapshot refreshed"
    );
    cx.update_global::<AuthUiState, _>(|state, _| state.snapshot = next);
}

fn next_snapshot(cx: &mut App) -> Option<AuthUiSnapshot> {
    let entity = crate::services::auth::controller(cx)?;
    let (state, provider_id) = {
        let controller = entity.read(cx);
        (
            controller.state().clone(),
            controller.provider_id().to_string(),
        )
    };
    let previous = snapshot(cx);
    let demo_authorize_url = if matches!(state, AuthState::SigningIn) {
        previous
            .demo_authorize_url
            .filter(|url| url.starts_with(MOCK_AUTHORIZER_PREFIX))
    } else {
        None
    };
    let (account_label, account_email, expires_at) = match &state {
        AuthState::SignedIn { account_label } => {
            let (email, expires_at) = read_account_facts(&provider_id, cx);
            (Some(account_label.clone()), email, expires_at)
        }
        _ => (None, None, None),
    };
    Some(AuthUiSnapshot {
        state,
        provider_id,
        account_label,
        account_email,
        expires_at,
        store_backend: default_backend(),
        demo_authorize_url,
    })
}

fn read_account_facts(provider_id: &str, cx: &mut App) -> (Option<String>, Option<i64>) {
    let store = crate::services::auth::KeyringStore;
    match store.load(provider_id, cx) {
        Ok(Some(tokens)) => (
            tokens
                .account
                .as_ref()
                .and_then(|account| account.email.clone()),
            tokens.expires_at,
        ),
        Ok(None) => (None, None),
        Err(err) => {
            tracing::debug!(
                target: "gpui_starter::auth_ui",
                provider_id,
                error = %err,
                "token store read failed; expiry stays unknown"
            );
            (None, None)
        }
    }
}

#[cfg(test)]
#[path = "auth_ui.test.rs"]
mod auth_ui_test;
