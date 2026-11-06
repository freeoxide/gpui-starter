use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use gpui::{Context, EventEmitter};

use crate::email::{EmailAccountStatus, PasswordAuth};
use crate::error::AuthError;
use crate::provider::{AuthIo, AuthProvider, FlowLaunch, RedirectParams};
use crate::store::TokenStore;
use crate::token::{AccountInfo, RefreshPolicy, SecretString, TokenSet};

#[cfg(not(target_family = "wasm"))]
type BoxedProviderFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
#[cfg(target_family = "wasm")]
type BoxedProviderFuture<T> = Pin<Box<dyn Future<Output = T>>>;

/// Sign-in state machine; `Error(AuthError)` is developer text, not
/// localized product copy.
#[derive(Clone, Debug, PartialEq)]
pub enum AuthState {
    SignedOut,
    SigningIn,
    SigningUp,
    SignedIn { account_label: String },
    Error(AuthError),
}

/// Transient flow outcomes; steady-state changes ride `cx.notify` instead.
#[derive(Clone, Debug)]
pub enum AuthEvent {
    AuthorizationRequested { url: String },
    FlowCompleted,
    FlowFailed { error: AuthError },
    SignedOut,
    AccountCreated { email: String },
}

pub struct AuthController {
    provider: Arc<dyn AuthProvider>,
    store: Arc<dyn TokenStore>,
    io: AuthIo,
    state: AuthState,
    pending: Option<FlowLaunch>,
    password: Option<Arc<dyn PasswordAuth>>,
    clock: Option<Arc<dyn Fn() -> i64 + Send + Sync>>,
    email_status: Option<EmailAccountStatus>,
    cached_tokens: Option<TokenSet>,
    generation: u64,
    refreshing: bool,
    warned_no_clock: bool,
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
            password: None,
            clock: None,
            email_status: None,
            cached_tokens: None,
            generation: 0,
            refreshing: false,
            warned_no_clock: false,
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

    /// Bumped whenever flow state resets; completions stamped with an older
    /// generation are dropped so they cannot overwrite the new state.
    fn invalidate_in_flight(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.refreshing = false;
    }

    /// Cache-first read so steady state never touches the store; misses
    /// load in a spawned task (keyring I/O must stay off the UI thread).
    fn with_stored_tokens(
        &mut self,
        cx: &mut Context<Self>,
        then: impl FnOnce(&mut Self, Result<Option<TokenSet>, AuthError>, &mut Context<Self>) + 'static,
    ) {
        if let Some(cached) = &self.cached_tokens {
            let cached = Ok(Some(cached.clone()));
            then(self, cached, cx);
            return;
        }
        let provider_id = self.provider_id().to_string();
        let store = Arc::clone(&self.store);
        self.launch(
            cx,
            Box::pin(async move { store.load(&provider_id).await }),
            move |controller, loaded, cx| {
                if let Ok(Some(tokens)) = &loaded {
                    controller.cached_tokens = Some(tokens.clone());
                }
                then(controller, loaded, cx);
            },
        );
    }

    fn launch<T: Send + 'static>(
        &self,
        cx: &mut Context<Self>,
        future: BoxedProviderFuture<Result<T, AuthError>>,
        apply: impl FnOnce(&mut Self, Result<T, AuthError>, &mut Context<Self>) + 'static,
    ) {
        let io = self.io.clone();
        let generation = self.generation;
        cx.spawn(async move |this, cx| {
            let result = io.drive(future).await;
            if let Err(err) = cx.update(|cx| {
                this.update(cx, |controller, cx| {
                    if controller.generation != generation {
                        return;
                    }
                    apply(controller, result, cx);
                })
            }) {
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
        self.invalidate_in_flight();
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
            Ok(tokens) => controller.on_tokens(tokens, cx),
            Err(error) => controller.on_flow_failed(error, cx),
        });
    }

    /// Drop the pending flow without contacting the provider. Any provider
    /// call already in flight is discarded on completion.
    pub fn cancel(&mut self, cx: &mut Context<Self>) {
        self.invalidate_in_flight();
        self.pending = None;
        self.state = AuthState::SignedOut;
        cx.notify();
    }

    /// Load persisted tokens; `SignedIn` when the store has them.
    pub fn restore(&mut self, cx: &mut Context<Self>) {
        self.with_stored_tokens(cx, |controller, loaded, cx| match loaded {
            Ok(Some(tokens)) => {
                let account_label =
                    account_label(tokens.account.as_ref(), controller.provider.display_name());
                tracing::info!(
                    target: "gpui_auth::controller",
                    provider_id = controller.provider_id(),
                    summary = %tokens.redacted_summary(),
                    "restored persisted tokens"
                );
                controller.state = AuthState::SignedIn { account_label };
                cx.notify();
            }
            Ok(None) => {}
            Err(error) => {
                tracing::warn!(
                    target: "gpui_auth::controller",
                    provider_id = controller.provider_id(),
                    error = %error,
                    "token restore failed"
                );
            }
        });
    }

    /// Unconditional refresh; callers gate on `TokenSet::needs_refresh`.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.do_refresh(cx);
    }

    /// Refresh only when the stored set expires within `margin_secs`.
    /// Without a clock it warns once and no-ops; no stored row is a no-op.
    pub fn refresh_if_needed(&mut self, margin_secs: i64, cx: &mut Context<Self>) {
        let Some(clock) = self.clock.clone() else {
            if !self.warned_no_clock {
                self.warned_no_clock = true;
                tracing::warn!(
                    target: "gpui_auth::controller",
                    provider_id = self.provider_id(),
                    "refresh_if_needed without a clock; skipping"
                );
            }
            return;
        };
        self.with_stored_tokens(cx, move |controller, stored, cx| {
            let Ok(Some(tokens)) = stored else {
                return;
            };
            if !tokens.needs_refresh(clock(), margin_secs) {
                return;
            }
            controller.do_refresh(cx);
        });
    }

    fn do_refresh(&mut self, cx: &mut Context<Self>) {
        if self.refreshing {
            return;
        }
        self.with_stored_tokens(cx, |controller, stored, cx| {
            // The load hop can race a second do_refresh; only one refresh.
            if controller.refreshing {
                return;
            }
            let tokens = match stored {
                Ok(Some(tokens)) => tokens,
                Ok(None) => {
                    controller.on_refresh_refused(AuthError::NoRefreshToken, cx);
                    return;
                }
                Err(error) => {
                    controller.on_refresh_refused(error, cx);
                    return;
                }
            };
            let Some(refresh_token) = tokens.refresh_token.clone() else {
                controller.on_refresh_refused(AuthError::NoRefreshToken, cx);
                return;
            };
            if controller.provider.refresh_policy() == RefreshPolicy::None {
                controller.on_refresh_refused(AuthError::NoRefreshToken, cx);
                return;
            }
            let provider = Arc::clone(&controller.provider);
            let http = controller.io.http.clone();
            // The secret moves into the future as a SecretString so the copy
            // wipes on drop instead of living on as a plain String.
            let future =
                Box::pin(async move { provider.refresh(http, refresh_token.expose()).await });
            controller.refreshing = true;
            controller.launch(cx, future, move |controller, result, cx| {
                controller.refreshing = false;
                match result {
                    Ok(new_tokens) => controller.on_tokens_refreshed(tokens, new_tokens, cx),
                    Err(error) => controller.on_refresh_refused(error, cx),
                }
            });
        });
    }

    /// Best-effort revoke, then delete stored tokens, then `SignedOut`.
    /// Any provider call already in flight is discarded on completion.
    pub fn sign_out(&mut self, cx: &mut Context<Self>) {
        self.invalidate_in_flight();
        self.with_stored_tokens(cx, |controller, stored, cx| {
            let Some(tokens) = stored.ok().flatten() else {
                controller.finish_sign_out(cx);
                return;
            };
            let provider = Arc::clone(&controller.provider);
            let http = controller.io.http.clone();
            let secret = tokens.access_token;
            let future = Box::pin(async move { provider.revoke(http, secret.expose()).await });
            controller.launch(cx, future, |controller, result, cx| {
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
        });
    }

    /// Register the credential seam; without it every password method
    /// errors with `PasswordAuthUnavailable`. A second call replaces.
    pub fn enable_password(&mut self, auth: Arc<dyn PasswordAuth>) {
        self.password = Some(auth);
    }

    /// Unix-seconds clock for expiry checks; `refresh_if_needed` no-ops
    /// without it.
    pub fn set_clock(&mut self, clock: Arc<dyn Fn() -> i64 + Send + Sync>) {
        self.clock = Some(clock);
    }

    pub fn email_status(&self) -> Option<&EmailAccountStatus> {
        self.email_status.as_ref()
    }

    /// Non-secret account facts (email, token expiry) from the cached
    /// token set; never touches the store, so UI mirrors can call it
    /// freely. `None` while signed out.
    pub fn account_facts(&self) -> (Option<String>, Option<i64>) {
        self.cached_tokens
            .as_ref()
            .map(|tokens| {
                (
                    tokens
                        .account
                        .as_ref()
                        .and_then(|account| account.email.clone()),
                    tokens.expires_at,
                )
            })
            .unwrap_or_default()
    }

    fn password_auth(&mut self, cx: &mut Context<Self>) -> Option<Arc<dyn PasswordAuth>> {
        let Some(auth) = self.password.clone() else {
            self.state = AuthState::Error(AuthError::PasswordAuthUnavailable);
            cx.notify();
            return None;
        };
        Some(auth)
    }

    /// Create the account, then back to `SignedOut` with `AccountCreated`.
    /// A no-op (warn log) while `SignedIn`.
    pub fn sign_up_with_password(
        &mut self,
        email: String,
        password: SecretString,
        cx: &mut Context<Self>,
    ) {
        if matches!(self.state, AuthState::SignedIn { .. }) {
            tracing::warn!(
                target: "gpui_auth::controller",
                provider_id = self.provider_id(),
                "sign-up ignored while signed in"
            );
            return;
        }
        let Some(auth) = self.password_auth(cx) else {
            return;
        };
        self.invalidate_in_flight();
        self.state = AuthState::SigningUp;
        cx.notify();
        let http = self.io.http.clone();
        let created_email = email.clone();
        let future = Box::pin(async move { auth.sign_up(http, &email, &password).await });
        self.launch(cx, future, move |controller, result, cx| match result {
            Ok(status) => {
                controller.email_status = Some(status);
                controller.state = AuthState::SignedOut;
                cx.notify();
                cx.emit(AuthEvent::AccountCreated {
                    email: created_email,
                });
            }
            Err(error) => controller.on_flow_failed(error, cx),
        });
    }

    /// Password sign-in: straight to `SignedIn`.
    pub fn sign_in_with_password(
        &mut self,
        email: String,
        password: SecretString,
        cx: &mut Context<Self>,
    ) {
        let Some(auth) = self.password_auth(cx) else {
            return;
        };
        self.invalidate_in_flight();
        self.state = AuthState::SigningIn;
        cx.notify();
        tracing::info!(
            target: "gpui_auth::controller",
            provider_id = self.provider_id(),
            "password sign-in started"
        );
        let http = self.io.http.clone();
        let future = Box::pin(async move { auth.sign_in(http, &email, &password).await });
        self.launch(cx, future, |controller, result, cx| match result {
            Ok(session) => {
                controller.email_status = Some(session.status);
                controller.on_tokens(session.tokens, cx);
            }
            Err(error) => controller.on_flow_failed(error, cx),
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
        self.state = AuthState::Error(error.clone());
        cx.notify();
        cx.emit(AuthEvent::FlowFailed { error });
    }

    /// Shared persistence path: provider check, store save in a spawned
    /// task, `SignedIn` once the save lands.
    fn on_tokens(&mut self, tokens: TokenSet, cx: &mut Context<Self>) {
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
        let provider_id = self.provider_id().to_string();
        let store = Arc::clone(&self.store);
        let persisted = tokens.clone();
        self.launch(
            cx,
            Box::pin(async move { store.save(&provider_id, &persisted).await }),
            move |controller, result, cx| match result {
                Ok(()) => {
                    tracing::info!(
                        target: "gpui_auth::controller",
                        provider_id = controller.provider_id(),
                        summary = %tokens.redacted_summary(),
                        "sign-in complete"
                    );
                    controller.cached_tokens = Some(tokens.clone());
                    let account_label =
                        account_label(tokens.account.as_ref(), controller.provider.display_name());
                    controller.state = AuthState::SignedIn { account_label };
                    cx.notify();
                    cx.emit(AuthEvent::FlowCompleted);
                }
                Err(error) => controller.on_flow_failed(error, cx),
            },
        );
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
        let provider_id = self.provider_id().to_string();
        let store = Arc::clone(&self.store);
        let persisted = merged.clone();
        self.launch(
            cx,
            Box::pin(async move { store.save(&provider_id, &persisted).await }),
            move |controller, result, cx| match result {
                Ok(()) => {
                    tracing::info!(
                        target: "gpui_auth::controller",
                        provider_id = controller.provider_id(),
                        summary = %merged.redacted_summary(),
                        "token refresh complete"
                    );
                    controller.cached_tokens = Some(merged.clone());
                    let account_label =
                        account_label(merged.account.as_ref(), controller.provider.display_name());
                    controller.state = AuthState::SignedIn { account_label };
                    cx.notify();
                }
                Err(error) => controller.on_refresh_refused(error, cx),
            },
        );
    }

    fn on_refresh_refused(&mut self, error: AuthError, cx: &mut Context<Self>) {
        tracing::warn!(
            target: "gpui_auth::controller",
            provider_id = self.provider_id(),
            error = %error,
            "token refresh refused"
        );
        self.state = AuthState::Error(error);
        cx.notify();
    }

    fn finish_sign_out(&mut self, cx: &mut Context<Self>) {
        self.invalidate_in_flight();
        self.pending = None;
        self.email_status = None;
        self.cached_tokens = None;
        self.state = AuthState::SignedOut;
        cx.notify();
        let provider_id = self.provider_id().to_string();
        let store = Arc::clone(&self.store);
        self.launch(
            cx,
            Box::pin(async move { store.delete(&provider_id).await }),
            |controller, result, cx| {
                if let Err(error) = result {
                    tracing::warn!(
                        target: "gpui_auth::controller",
                        provider_id = controller.provider_id(),
                        error = %error,
                        "token delete failed during sign-out"
                    );
                }
                cx.emit(AuthEvent::SignedOut);
            },
        );
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
