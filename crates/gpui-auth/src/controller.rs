use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use gpui::{Context, EventEmitter};

use crate::email::{
    EmailAccountStatus, PasswordAuth, SignInOutcome, TotpChallenge, TotpEnrollment,
};
use crate::error::AuthError;
use crate::provider::{AuthIo, AuthProvider, FlowLaunch, RedirectParams};
use crate::store::TokenStore;
use crate::token::{AccountInfo, RefreshPolicy, SecretString, TokenSet};

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
    SigningUp,
    AwaitingChallenge {
        email: String,
        error: Option<String>,
    },
    SignedIn {
        account_label: String,
    },
    Error(String),
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
    pending_challenge: Option<TotpChallenge>,
    email_status: Option<EmailAccountStatus>,
    pending_enrollment: Option<TotpEnrollment>,
    enrollment_error: Option<String>,
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
            pending_challenge: None,
            email_status: None,
            pending_enrollment: None,
            enrollment_error: None,
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
            Ok(tokens) => controller.on_tokens(tokens, cx),
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
        let Ok(Some(tokens)) = self.store.load(self.provider_id(), cx) else {
            return;
        };
        if !tokens.needs_refresh(clock(), margin_secs) {
            return;
        }
        self.do_refresh(cx);
    }

    fn do_refresh(&mut self, cx: &mut Context<Self>) {
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

    pub fn totp_enrollment(&self) -> Option<&TotpEnrollment> {
        self.pending_enrollment.as_ref()
    }

    pub fn totp_enrollment_error(&self) -> Option<&str> {
        self.enrollment_error.as_deref()
    }

    fn password_auth(&mut self, cx: &mut Context<Self>) -> Option<Arc<dyn PasswordAuth>> {
        let Some(auth) = self.password.clone() else {
            self.state = AuthState::Error(AuthError::PasswordAuthUnavailable.to_string());
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

    /// Password sign-in: straight to `SignedIn`, or `AwaitingChallenge`
    /// when the backend demands TOTP.
    pub fn sign_in_with_password(
        &mut self,
        email: String,
        password: SecretString,
        cx: &mut Context<Self>,
    ) {
        let Some(auth) = self.password_auth(cx) else {
            return;
        };
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
            Ok(SignInOutcome::Authenticated(session)) => {
                controller.email_status = Some(session.status);
                controller.on_tokens(session.tokens, cx);
            }
            Ok(SignInOutcome::TotpRequired(challenge)) => {
                controller.state = AuthState::AwaitingChallenge {
                    email: challenge.email.clone(),
                    error: None,
                };
                controller.pending_challenge = Some(challenge);
                cx.notify();
            }
            Err(error) => controller.on_flow_failed(error, cx),
        });
    }

    /// Answer the pending TOTP challenge; a wrong code keeps it with `error`
    /// set, an expired one drops it back to `SignedOut`.
    pub fn submit_totp(&mut self, code: String, cx: &mut Context<Self>) {
        let Some(auth) = self.password_auth(cx) else {
            return;
        };
        let Some(challenge) = self.pending_challenge.clone() else {
            self.state = AuthState::Error(AuthError::NoPendingChallenge.to_string());
            cx.notify();
            return;
        };
        let http = self.io.http.clone();
        let challenge_email = challenge.email.clone();
        let future = Box::pin(async move { auth.submit_totp(http, &challenge, &code).await });
        self.launch(cx, future, move |controller, result, cx| match result {
            Ok(session) => {
                controller.pending_challenge = None;
                controller.email_status = Some(session.status);
                controller.on_tokens(session.tokens, cx);
            }
            Err(AuthError::TotpFailed) => {
                controller.state = AuthState::AwaitingChallenge {
                    email: challenge_email,
                    error: Some(AuthError::TotpFailed.to_string()),
                };
                cx.notify();
            }
            Err(AuthError::InvalidChallenge) => {
                controller.pending_challenge = None;
                controller.state = AuthState::SignedOut;
                cx.notify();
            }
            Err(error) => {
                controller.pending_challenge = None;
                controller.on_flow_failed(error, cx);
            }
        });
    }

    /// Drop the pending challenge and return to `SignedOut`.
    pub fn cancel_challenge(&mut self, cx: &mut Context<Self>) {
        self.pending_challenge = None;
        self.state = AuthState::SignedOut;
        cx.notify();
    }

    /// Fetch a new TOTP enrollment round for the signed-in account.
    /// Requires `SignedIn` and an account email on record.
    pub fn enroll_totp(&mut self, cx: &mut Context<Self>) {
        if !matches!(self.state, AuthState::SignedIn { .. }) {
            tracing::warn!(
                target: "gpui_auth::controller",
                provider_id = self.provider_id(),
                "TOTP enrollment requires a signed-in account"
            );
            return;
        }
        let Some(auth) = self.password_auth(cx) else {
            return;
        };
        let email = self
            .email_status
            .as_ref()
            .map(|status| status.email.clone())
            .or_else(|| {
                self.store
                    .load(self.provider_id(), cx)
                    .ok()
                    .flatten()
                    .and_then(|tokens| tokens.account.and_then(|account| account.email))
            });
        let Some(email) = email else {
            tracing::warn!(
                target: "gpui_auth::controller",
                provider_id = self.provider_id(),
                "TOTP enrollment skipped: no account email on record"
            );
            return;
        };
        let http = self.io.http.clone();
        let future = Box::pin(async move { auth.begin_totp_enrollment(http, &email).await });
        self.launch(cx, future, |controller, result, cx| match result {
            Ok(enrollment) => {
                controller.pending_enrollment = Some(enrollment);
                controller.enrollment_error = None;
                cx.notify();
            }
            Err(error) => {
                controller.enrollment_error = Some(error.to_string());
                cx.notify();
            }
        });
    }

    /// Confirm the pending enrollment with a code; failure keeps the round
    /// and records `enrollment_error`.
    pub fn confirm_totp_enrollment(&mut self, code: String, cx: &mut Context<Self>) {
        let Some(auth) = self.password_auth(cx) else {
            return;
        };
        let Some(enrollment) = self.pending_enrollment.clone() else {
            self.state = AuthState::Error(AuthError::NoPendingChallenge.to_string());
            cx.notify();
            return;
        };
        let http = self.io.http.clone();
        let future = Box::pin(async move {
            auth.confirm_totp_enrollment(http, &enrollment.email, &code)
                .await
        });
        self.launch(cx, future, |controller, result, cx| match result {
            Ok(()) => {
                controller.pending_enrollment = None;
                controller.enrollment_error = None;
                cx.notify();
            }
            Err(error) => {
                controller.enrollment_error = Some(error.to_string());
                cx.notify();
            }
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

    /// Shared persistence path: provider check, store save, `SignedIn`.
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
        self.pending_challenge = None;
        self.email_status = None;
        self.pending_enrollment = None;
        self.enrollment_error = None;
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
