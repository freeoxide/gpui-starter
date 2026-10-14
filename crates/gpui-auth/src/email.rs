//! Removable email+password auth. `AuthBackend`, `PasswordAuth`, and
//! `EmailPasswordProvider` are the crate surface; `DemoBackend` is the native
//! demo implementation. Removal = delete email.rs/demo_backend.rs, revert the
//! AuthError/AuthState/AuthEvent additions and the controller's password
//! methods in this crate, restore 21's three-arg install_with in
//! src/services/auth.rs, drop the argon2/hmac/sha1 manifest lines, and (if
//! Phase 6 landed) remove the auth-dev-server member and justfile recipe.

use std::sync::Arc;

use crate::error::AuthError;
use crate::provider::AuthProvider;
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

/// Sign-in outcome: a ready session, or a TOTP challenge to answer first.
#[derive(Clone, Debug)]
#[allow(clippy::large_enum_variant)]
pub enum SignInOutcome {
    Authenticated(AuthenticatedSession),
    TotpRequired(TotpChallenge),
}

/// Opaque pending-challenge handle; `email` is carried for UI copy.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TotpChallenge {
    pub id: String,
    pub email: String,
}

/// One enrollment round: base32 secret for manual entry plus the otpauth URL.
/// Debug prints `[redacted]` for both secret-bearing fields (the URL embeds
/// the secret).
#[derive(Clone)]
pub struct TotpEnrollment {
    pub email: String,
    pub secret_base32: String,
    pub otpauth_url: String,
}

impl std::fmt::Debug for TotpEnrollment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TotpEnrollment")
            .field("email", &self.email)
            .field("secret_base32", &"[redacted]")
            .field("otpauth_url", &"[redacted]")
            .finish()
    }
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
    ) -> Result<SignInOutcome, AuthError>;
    async fn verify_totp(
        &self,
        http: reqwest::Client,
        challenge: &TotpChallenge,
        code: &str,
        now_unix: i64,
    ) -> Result<AuthenticatedSession, AuthError>;
    async fn begin_totp_enrollment(
        &self,
        http: reqwest::Client,
        email: &str,
    ) -> Result<TotpEnrollment, AuthError>;
    async fn confirm_totp_enrollment(
        &self,
        http: reqwest::Client,
        email: &str,
        code: &str,
        now_unix: i64,
    ) -> Result<(), AuthError>;
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
    ) -> Result<SignInOutcome, AuthError>;
    async fn submit_totp(
        &self,
        http: reqwest::Client,
        challenge: &TotpChallenge,
        code: &str,
    ) -> Result<AuthenticatedSession, AuthError>;
    async fn begin_totp_enrollment(
        &self,
        http: reqwest::Client,
        email: &str,
    ) -> Result<TotpEnrollment, AuthError>;
    async fn confirm_totp_enrollment(
        &self,
        http: reqwest::Client,
        email: &str,
        code: &str,
    ) -> Result<(), AuthError>;
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

    async fn start_flow(
        &self,
        _http: reqwest::Client,
    ) -> Result<crate::provider::FlowLaunch, AuthError> {
        Err(AuthError::NoBrowserFlow)
    }

    async fn complete_callback(
        &self,
        _http: reqwest::Client,
        _redirect: crate::provider::RedirectParams,
        _launch: &crate::provider::FlowLaunch,
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
    ) -> Result<SignInOutcome, AuthError> {
        self.backend
            .sign_in(http, email, password, self.now())
            .await
    }

    async fn submit_totp(
        &self,
        http: reqwest::Client,
        challenge: &TotpChallenge,
        code: &str,
    ) -> Result<AuthenticatedSession, AuthError> {
        self.backend
            .verify_totp(http, challenge, code, self.now())
            .await
    }

    async fn begin_totp_enrollment(
        &self,
        http: reqwest::Client,
        email: &str,
    ) -> Result<TotpEnrollment, AuthError> {
        self.backend.begin_totp_enrollment(http, email).await
    }

    async fn confirm_totp_enrollment(
        &self,
        http: reqwest::Client,
        email: &str,
        code: &str,
    ) -> Result<(), AuthError> {
        self.backend
            .confirm_totp_enrollment(http, email, code, self.now())
            .await
    }
}

#[cfg(test)]
#[path = "email.test.rs"]
pub(crate) mod email_test;
