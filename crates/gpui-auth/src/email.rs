//! Email+password auth over `AuthBackend`; `DemoBackend` (native) is the
//! demo implementation.

use std::sync::Arc;

use crate::error::AuthError;
use crate::provider::{AuthProvider, FlowLaunch, RedirectParams};
use crate::token::{RefreshPolicy, SecretString, TokenSet};

/// Provider id for the email+password flow; also the store key.
pub const EMAIL_PROVIDER_ID: &str = "email";

/// Account state as the backend reports it. `verified` is a modeled state in
/// the demo (nothing is sent); it never gates sign-in.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EmailAccountStatus {
    pub email: String,
    pub verified: bool,
}

/// Tokens plus account status from one successful credential exchange.
#[derive(Clone, Debug)]
pub struct AuthenticatedSession {
    pub tokens: TokenSet,
    pub status: EmailAccountStatus,
}

/// App-owned backend seam. Every method takes `now_unix` (unix seconds) so
/// implementations stay deterministic; none touch GPUI contexts.
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
pub trait AuthBackend: Send + Sync {
    fn id(&self) -> &str;
    async fn sign_up(
        &self,
        http: reqwest::Client,
        email: &str,
        password: &SecretString,
        now_unix: i64,
    ) -> Result<EmailAccountStatus, AuthError>;
    async fn sign_in(
        &self,
        http: reqwest::Client,
        email: &str,
        password: &SecretString,
        now_unix: i64,
    ) -> Result<AuthenticatedSession, AuthError>;
    async fn refresh(
        &self,
        http: reqwest::Client,
        refresh_token: &str,
        now_unix: i64,
    ) -> Result<TokenSet, AuthError>;
    async fn revoke(&self, http: reqwest::Client, token: &str) -> Result<(), AuthError>;
}

/// Controller-facing credential seam. Same Send split as `AuthProvider`.
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
pub trait PasswordAuth: Send + Sync {
    async fn sign_up(
        &self,
        http: reqwest::Client,
        email: &str,
        password: &SecretString,
    ) -> Result<EmailAccountStatus, AuthError>;
    async fn sign_in(
        &self,
        http: reqwest::Client,
        email: &str,
        password: &SecretString,
    ) -> Result<AuthenticatedSession, AuthError>;
}

/// `AuthProvider` adapter over an [`AuthBackend`]: no browser flow, tokens
/// and credential calls delegated to the backend with the injected clock.
pub struct EmailPasswordProvider {
    backend: Arc<dyn AuthBackend>,
    clock: Arc<dyn Fn() -> i64 + Send + Sync>,
}

impl EmailPasswordProvider {
    /// `clock` supplies unix seconds; the host injects a real clock.
    pub fn new(backend: Arc<dyn AuthBackend>, clock: Arc<dyn Fn() -> i64 + Send + Sync>) -> Self {
        Self { backend, clock }
    }

    fn now(&self) -> i64 {
        (self.clock)()
    }
}

#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
impl AuthProvider for EmailPasswordProvider {
    fn id(&self) -> &str {
        EMAIL_PROVIDER_ID
    }

    fn display_name(&self) -> &str {
        "Email"
    }

    fn refresh_policy(&self) -> RefreshPolicy {
        RefreshPolicy::ReuseUntilDead
    }

    async fn start_flow(&self, _http: reqwest::Client) -> Result<FlowLaunch, AuthError> {
        Err(AuthError::NoBrowserFlow)
    }

    async fn complete_callback(
        &self,
        _http: reqwest::Client,
        _redirect: RedirectParams,
        _launch: &FlowLaunch,
    ) -> Result<TokenSet, AuthError> {
        Err(AuthError::NoBrowserFlow)
    }

    async fn refresh(
        &self,
        http: reqwest::Client,
        refresh_token: &str,
    ) -> Result<TokenSet, AuthError> {
        self.backend.refresh(http, refresh_token, self.now()).await
    }

    async fn revoke(&self, http: reqwest::Client, access_token: &str) -> Result<(), AuthError> {
        self.backend.revoke(http, access_token).await
    }
}

#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
impl PasswordAuth for EmailPasswordProvider {
    async fn sign_up(
        &self,
        http: reqwest::Client,
        email: &str,
        password: &SecretString,
    ) -> Result<EmailAccountStatus, AuthError> {
        self.backend
            .sign_up(http, email, password, self.now())
            .await
    }

    async fn sign_in(
        &self,
        http: reqwest::Client,
        email: &str,
        password: &SecretString,
    ) -> Result<AuthenticatedSession, AuthError> {
        self.backend
            .sign_in(http, email, password, self.now())
            .await
    }
}

#[cfg(test)]
#[path = "email.test.rs"]
pub(crate) mod email_test;
