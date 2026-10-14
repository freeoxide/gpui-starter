use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use gpui::{Context, EventEmitter};

use crate::error::AuthError;
use crate::provider::{AuthIo, AuthProvider, FlowLaunch, RedirectParams};
use crate::store::TokenStore;
use crate::token::{AccountInfo, RefreshPolicy, TokenSet};

#[cfg(not(target_family = "wasm"))]
type BoxedProviderFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
#[cfg(target_family = "wasm")]
type BoxedProviderFuture<T> = Pin<Box<dyn Future<Output = T>>>;

/// Sign-in state machine; `Error(String)` is developer text, not localized
/// product copy.
#[derive(Clone, Debug, PartialEq)]
pub enum AuthState {
    SignedOut,
    SigningIn,
    SignedIn { account_label: String },
    Error(String),
}

/// Transient flow outcomes; steady-state changes ride `cx.notify` instead.
#[derive(Clone, Debug)]
pub enum AuthEvent {
    AuthorizationRequested { url: String },
    FlowCompleted,
    FlowFailed { error: AuthError },
    SignedOut,
}

pub struct AuthController {
    provider: Arc<dyn AuthProvider>,
    store: Arc<dyn TokenStore>,
    io: AuthIo,
    state: AuthState,
    pending: Option<FlowLaunch>,
}

impl EventEmitter<AuthEvent> for AuthController {}

impl AuthController {
    /// Starts `SignedOut`; performs no I/O.
    pub fn new(
        provider: Arc<dyn AuthProvider>,
        store: Arc<dyn TokenStore>,
        io: AuthIo,
        _cx: &mut Context<Self>,
    ) -> Self {
        Self {
            provider,
            store,
            io,
            state: AuthState::SignedOut,
            pending: None,
        }
    }

    pub fn state(&self) -> &AuthState {
        &self.state
    }

    pub fn provider_id(&self) -> &str {
        self.provider.id()
    }

    #[cfg(test)]
    pub(crate) fn pending_is_some(&self) -> bool {
        self.pending.is_some()
    }

    fn launch<T: Send + 'static>(
        &self,
        cx: &mut Context<Self>,
        future: BoxedProviderFuture<Result<T, AuthError>>,
        apply: impl FnOnce(&mut Self, Result<T, AuthError>, &mut Context<Self>) + 'static,
    ) {
        let io = self.io.clone();
        cx.spawn(async move |this, cx| {
            let result = io.drive(future).await;
            if let Err(err) =
                cx.update(|cx| this.update(cx, |controller, cx| apply(controller, result, cx)))
            {
                tracing::debug!(
                    target: "gpui_auth::controller",
                    error = %err,
                    "auth controller released before task completed"
                );
            }
        })
        .detach();
    }

    /// Replace any pending flow and start a new one (`SigningIn`).
    pub fn start_sign_in(&mut self, cx: &mut Context<Self>) {
        self.pending = None;
        self.state = AuthState::SigningIn;
        cx.notify();
        tracing::info!(
            target: "gpui_auth::controller",
            provider_id = self.provider_id(),
            "starting sign-in flow"
        );
        let provider = Arc::clone(&self.provider);
        let http = self.io.http.clone();
        let future = Box::pin(async move { provider.start_flow(http).await });
        self.launch(cx, future, |controller, result, cx| match result {
            Ok(launch) => controller.on_flow_prepared(launch, cx),
            Err(error) => controller.on_flow_failed(error, cx),
        });
    }

    /// Route a raw redirect URL: parse, state-match, exchange, persist.
    /// Consumes the pending flow whether it succeeds or fails.
    pub fn complete_callback(&mut self, raw_redirect: &str, cx: &mut Context<Self>) {
        let Some(launch) = self.pending.take() else {
            self.on_flow_failed(AuthError::NoPendingFlow, cx);
            return;
        };
        let redirect = match RedirectParams::from_url(raw_redirect) {
            Ok(redirect) => redirect,
            Err(error) => {
                self.on_flow_failed(error, cx);
                return;
            }
        };
        if let Some(error) = redirect.error.clone() {
            self.on_flow_failed(
                AuthError::ProviderDenied {
                    error,
                    description: redirect.error_description.clone(),
                },
                cx,
            );
            return;
        }
        if redirect.state.as_deref() != Some(launch.state.expose()) {
            self.on_flow_failed(AuthError::StateMismatch, cx);
            return;
        }
        let provider = Arc::clone(&self.provider);
        let http = self.io.http.clone();
        let future =
            Box::pin(async move { provider.complete_callback(http, redirect, &launch).await });
        self.launch(cx, future, |controller, result, cx| match result {
            Ok(tokens) => controller.on_tokens_minted(tokens, cx),
            Err(error) => controller.on_flow_failed(error, cx),
        });
    }

    /// Drop the pending flow without contacting the provider.
    pub fn cancel(&mut self, cx: &mut Context<Self>) {
        self.pending = None;
        self.state = AuthState::SignedOut;
        cx.notify();
    }

    /// Load persisted tokens; `SignedIn` when the store has them.
    pub fn restore(&mut self, cx: &mut Context<Self>) {
        match self.store.load(self.provider_id(), cx) {
            Ok(Some(tokens)) => {
                let account_label =
                    account_label(tokens.account.as_ref(), self.provider.display_name());
                tracing::info!(
                    target: "gpui_auth::controller",
                    provider_id = self.provider_id(),
                    summary = %tokens.redacted_summary(),
                    "restored persisted tokens"
                );
                self.state = AuthState::SignedIn { account_label };
                cx.notify();
            }
            Ok(None) => {}
            Err(error) => {
                tracing::warn!(
                    target: "gpui_auth::controller",
                    provider_id = self.provider_id(),
                    error = %error,
                    "token restore failed"
                );
            }
        }
    }

    /// Unconditional refresh; callers gate on `TokenSet::needs_refresh`.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let tokens = match self.store.load(self.provider_id(), cx) {
            Ok(Some(tokens)) => tokens,
            Ok(None) => {
                self.on_refresh_refused(AuthError::NoRefreshToken, cx);
                return;
            }
            Err(error) => {
                self.on_refresh_refused(error, cx);
                return;
            }
        };
        let Some(refresh_token) = tokens.refresh_token.clone() else {
            self.on_refresh_refused(AuthError::NoRefreshToken, cx);
            return;
        };
        if self.provider.refresh_policy() == RefreshPolicy::None {
            self.on_refresh_refused(AuthError::NoRefreshToken, cx);
            return;
        }
        let provider = Arc::clone(&self.provider);
        let http = self.io.http.clone();
        let secret = refresh_token.expose().to_string();
        let future = Box::pin(async move { provider.refresh(http, &secret).await });
        self.launch(cx, future, move |controller, result, cx| match result {
            Ok(new_tokens) => controller.on_tokens_refreshed(tokens, new_tokens, cx),
            Err(error) => controller.on_refresh_refused(error, cx),
        });
    }

    /// Best-effort revoke, then delete stored tokens, then `SignedOut`.
    pub fn sign_out(&mut self, cx: &mut Context<Self>) {
        let stored = self.store.load(self.provider_id(), cx).ok().flatten();
        let Some(tokens) = stored else {
            self.finish_sign_out(cx);
            return;
        };
        let provider = Arc::clone(&self.provider);
        let http = self.io.http.clone();
        let secret = tokens.access_token.expose().to_string();
        let future = Box::pin(async move { provider.revoke(http, &secret).await });
        self.launch(cx, future, |controller, result, cx| {
            if let Err(error) = result {
                tracing::warn!(
                    target: "gpui_auth::controller",
                    provider_id = controller.provider_id(),
                    error = %error,
                    "revoke failed; continuing sign-out"
                );
            }
            controller.finish_sign_out(cx);
        });
    }

    fn on_flow_prepared(&mut self, launch: FlowLaunch, cx: &mut Context<Self>) {
        tracing::info!(
            target: "gpui_auth::controller",
            provider_id = self.provider_id(),
            "authorization request ready"
        );
        let url = launch.authorize_url.clone();
        self.pending = Some(launch);
        cx.notify();
        cx.emit(AuthEvent::AuthorizationRequested { url });
    }

    fn on_flow_failed(&mut self, error: AuthError, cx: &mut Context<Self>) {
        tracing::warn!(
            target: "gpui_auth::controller",
            provider_id = self.provider_id(),
            error = %error,
            "auth flow failed"
        );
        self.state = AuthState::Error(error.to_string());
        cx.notify();
        cx.emit(AuthEvent::FlowFailed { error });
    }

    fn on_tokens_minted(&mut self, tokens: TokenSet, cx: &mut Context<Self>) {
        if tokens.provider_id != self.provider_id() {
            self.on_flow_failed(
                AuthError::InvalidTokenResponse(format!(
                    "provider id mismatch: expected {}, got {}",
                    self.provider_id(),
                    tokens.provider_id
                )),
                cx,
            );
            return;
        }
        if let Err(error) = self.store.save(self.provider_id(), &tokens, cx) {
            self.on_flow_failed(error, cx);
            return;
        }
        tracing::info!(
            target: "gpui_auth::controller",
            provider_id = self.provider_id(),
            summary = %tokens.redacted_summary(),
            "sign-in complete"
        );
        let account_label = account_label(tokens.account.as_ref(), self.provider.display_name());
        self.state = AuthState::SignedIn { account_label };
        cx.notify();
        cx.emit(AuthEvent::FlowCompleted);
    }

    fn on_tokens_refreshed(&mut self, old: TokenSet, new_tokens: TokenSet, cx: &mut Context<Self>) {
        if new_tokens.provider_id != self.provider_id() {
            self.on_refresh_refused(
                AuthError::InvalidTokenResponse(format!(
                    "provider id mismatch: expected {}, got {}",
                    self.provider_id(),
                    new_tokens.provider_id
                )),
                cx,
            );
            return;
        }
        let merged = self.provider.refresh_policy().merge(&old, new_tokens);
        if let Err(error) = self.store.save(self.provider_id(), &merged, cx) {
            self.on_refresh_refused(error, cx);
            return;
        }
        tracing::info!(
            target: "gpui_auth::controller",
            provider_id = self.provider_id(),
            summary = %merged.redacted_summary(),
            "token refresh complete"
        );
        let account_label = account_label(merged.account.as_ref(), self.provider.display_name());
        self.state = AuthState::SignedIn { account_label };
        cx.notify();
    }

    fn on_refresh_refused(&mut self, error: AuthError, cx: &mut Context<Self>) {
        tracing::warn!(
            target: "gpui_auth::controller",
            provider_id = self.provider_id(),
            error = %error,
            "token refresh refused"
        );
        self.state = AuthState::Error(error.to_string());
        cx.notify();
    }

    fn finish_sign_out(&mut self, cx: &mut Context<Self>) {
        if let Err(error) = self.store.delete(self.provider_id(), cx) {
            tracing::warn!(
                target: "gpui_auth::controller",
                provider_id = self.provider_id(),
                error = %error,
                "token delete failed during sign-out"
            );
        }
        self.pending = None;
        self.state = AuthState::SignedOut;
        cx.notify();
        cx.emit(AuthEvent::SignedOut);
    }
}

fn account_label(account: Option<&AccountInfo>, fallback: &str) -> String {
    account
        .and_then(|account| {
            account
                .display_name
                .clone()
                .or_else(|| account.email.clone())
        })
        .unwrap_or_else(|| fallback.to_string())
}

#[cfg(all(test, feature = "mock-provider"))]
#[path = "controller.test.rs"]
mod controller_test;
