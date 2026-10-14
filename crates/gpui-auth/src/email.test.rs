use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use super::*;
use crate::error::AuthError;
use crate::token::{AccountInfo, SecretString, TokenSet};

pub(crate) const TEST_TTL_SECS: i64 = 900;
const TEST_SECRET_BASE32: &str = "GEZDGNBVGY3TQOJQGEZDGNBVGY3TQOJQ";

pub(crate) fn fixed_clock(at: i64) -> (Arc<AtomicI64>, Arc<dyn Fn() -> i64 + Send + Sync>) {
    let atomic = Arc::new(AtomicI64::new(at));
    let reader = atomic.clone();
    let clock: Arc<dyn Fn() -> i64 + Send + Sync> = Arc::new(move || reader.load(Ordering::SeqCst));
    (atomic, clock)
}

#[derive(Default)]
pub(crate) struct TestBackend {
    users: Mutex<HashMap<String, String>>,
    seq: AtomicU64,
    totp_required: AtomicBool,
    totp_fail: AtomicBool,
    invalid_challenge: AtomicBool,
    confirm_fail: AtomicBool,
    refresh_rejected: AtomicBool,
    observed_now: Mutex<Vec<i64>>,
    refresh_seen: Mutex<Vec<String>>,
}

impl TestBackend {
    pub(crate) fn require_totp(&self) {
        self.totp_required.store(true, Ordering::SeqCst);
    }

    pub(crate) fn fail_totp_once(&self) {
        self.totp_fail.store(true, Ordering::SeqCst);
    }

    pub(crate) fn fail_challenge_once(&self) {
        self.invalid_challenge.store(true, Ordering::SeqCst);
    }

    pub(crate) fn fail_confirm_once(&self) {
        self.confirm_fail.store(true, Ordering::SeqCst);
    }

    pub(crate) fn reject_refresh(&self) {
        self.refresh_rejected.store(true, Ordering::SeqCst);
    }

    pub(crate) fn observed_now(&self) -> Vec<i64> {
        self.observed_now.lock().unwrap().clone()
    }

    pub(crate) fn refresh_seen(&self) -> Vec<String> {
        self.refresh_seen.lock().unwrap().clone()
    }

    fn observe(&self, now_unix: i64) {
        self.observed_now.lock().unwrap().push(now_unix);
    }

    fn mint(&self, email: &str, refresh_token: &str, now_unix: i64) -> TokenSet {
        let seq = self.seq.fetch_add(1, Ordering::SeqCst);
        TokenSet {
            provider_id: EMAIL_PROVIDER_ID.to_string(),
            access_token: SecretString::new(format!("test-access-{seq}")),
            refresh_token: Some(SecretString::new(refresh_token)),
            id_token: None,
            token_type: "Bearer".to_string(),
            expires_at: Some(now_unix + TEST_TTL_SECS),
            scope: None,
            account: Some(AccountInfo {
                id: Some(seq.to_string()),
                email: Some(email.to_string()),
                display_name: Some(email.to_string()),
            }),
        }
    }

    fn session(&self, email: &str, refresh_token: &str, now_unix: i64) -> AuthenticatedSession {
        AuthenticatedSession {
            tokens: self.mint(email, refresh_token, now_unix),
            status: EmailAccountStatus {
                email: email.to_string(),
                verified: false,
            },
        }
    }

    fn credentials_match(&self, email: &str, password: &SecretString) -> bool {
        self.users
            .lock()
            .unwrap()
            .get(email)
            .is_some_and(|stored| stored == password.expose())
    }
}

pub(crate) fn challenge_for(email: &str) -> TotpChallenge {
    TotpChallenge {
        id: "test-ch-1".to_string(),
        email: email.to_string(),
    }
}

pub(crate) fn enrollment_for(email: &str) -> TotpEnrollment {
    TotpEnrollment {
        email: email.to_string(),
        secret_base32: TEST_SECRET_BASE32.to_string(),
        otpauth_url: format!("otpauth://totp/gpui-starter:{email}?secret={TEST_SECRET_BASE32}"),
    }
}

#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
impl AuthBackend for TestBackend {
    fn id(&self) -> &str {
        "test"
    }

    async fn sign_up(
        &self,
        _http: reqwest::Client,
        email: &str,
        password: &SecretString,
        now_unix: i64,
    ) -> Result<EmailAccountStatus, AuthError> {
        self.observe(now_unix);
        let mut users = self.users.lock().unwrap();
        if users.contains_key(email) {
            return Err(AuthError::EmailTaken);
        }
        users.insert(email.to_string(), password.expose().to_string());
        Ok(EmailAccountStatus {
            email: email.to_string(),
            verified: false,
        })
    }

    async fn sign_in(
        &self,
        _http: reqwest::Client,
        email: &str,
        password: &SecretString,
        now_unix: i64,
    ) -> Result<SignInOutcome, AuthError> {
        self.observe(now_unix);
        if !self.credentials_match(email, password) {
            return Err(AuthError::InvalidCredentials);
        }
        if self.totp_required.load(Ordering::SeqCst) {
            return Ok(SignInOutcome::TotpRequired(challenge_for(email)));
        }
        let refresh = format!("test-refresh-{}", self.seq.load(Ordering::SeqCst));
        Ok(SignInOutcome::Authenticated(
            self.session(email, &refresh, now_unix),
        ))
    }

    async fn verify_totp(
        &self,
        _http: reqwest::Client,
        challenge: &TotpChallenge,
        _code: &str,
        now_unix: i64,
    ) -> Result<AuthenticatedSession, AuthError> {
        self.observe(now_unix);
        if challenge.id != "test-ch-1" || self.invalid_challenge.swap(false, Ordering::SeqCst) {
            return Err(AuthError::InvalidChallenge);
        }
        if self.totp_fail.swap(false, Ordering::SeqCst) {
            return Err(AuthError::TotpFailed);
        }
        Ok(self.session(&challenge.email, "test-refresh-totp", now_unix))
    }

    async fn begin_totp_enrollment(
        &self,
        _http: reqwest::Client,
        email: &str,
    ) -> Result<TotpEnrollment, AuthError> {
        if !self.users.lock().unwrap().contains_key(email) {
            return Err(AuthError::InvalidCredentials);
        }
        Ok(enrollment_for(email))
    }

    async fn confirm_totp_enrollment(
        &self,
        _http: reqwest::Client,
        _email: &str,
        _code: &str,
        now_unix: i64,
    ) -> Result<(), AuthError> {
        self.observe(now_unix);
        if self.confirm_fail.swap(false, Ordering::SeqCst) {
            return Err(AuthError::TotpFailed);
        }
        Ok(())
    }

    async fn refresh(
        &self,
        _http: reqwest::Client,
        refresh_token: &str,
        now_unix: i64,
    ) -> Result<TokenSet, AuthError> {
        self.observe(now_unix);
        self.refresh_seen
            .lock()
            .unwrap()
            .push(refresh_token.to_string());
        if self.refresh_rejected.load(Ordering::SeqCst) {
            return Err(AuthError::RefreshRejected);
        }
        Ok(self.mint("test@example.test", refresh_token, now_unix))
    }

    async fn revoke(&self, _http: reqwest::Client, token: &str) -> Result<(), AuthError> {
        self.refresh_seen
            .lock()
            .unwrap()
            .push(format!("revoke:{token}"));
        Ok(())
    }
}

pub(crate) fn email_provider(
    backend: &Arc<TestBackend>,
    clock: Arc<dyn Fn() -> i64 + Send + Sync>,
) -> Arc<EmailPasswordProvider> {
    Arc::new(EmailPasswordProvider::new(
        backend.clone() as Arc<dyn AuthBackend>,
        clock,
    ))
}

#[test]
fn provider_id_is_email() {
    assert_eq!(EMAIL_PROVIDER_ID, "email");
}

#[gpui::test]
async fn browser_flow_methods_report_no_browser_flow() {
    let (_clock, clock_fn) = fixed_clock(0);
    let provider = email_provider(&Arc::new(TestBackend::default()), clock_fn);
    match provider.start_flow(reqwest::Client::new()).await {
        Err(AuthError::NoBrowserFlow) => {}
        Err(other) => panic!("expected NoBrowserFlow, got {other}"),
        Ok(_) => panic!("expected NoBrowserFlow, got Ok"),
    }
    let launch = crate::provider::FlowLaunch {
        authorize_url: String::new(),
        state: SecretString::new("state"),
        pkce: None,
        redirect_uri: String::new(),
    };
    let redirect = crate::provider::RedirectParams {
        code: None,
        state: None,
        error: None,
        error_description: None,
    };
    match provider
        .complete_callback(reqwest::Client::new(), redirect, &launch)
        .await
    {
        Err(AuthError::NoBrowserFlow) => {}
        Err(other) => panic!("expected NoBrowserFlow, got {other}"),
        Ok(_) => panic!("expected NoBrowserFlow, got Ok"),
    }
}

#[gpui::test]
async fn refresh_forwards_the_token_to_the_backend() {
    let (_clock, clock_fn) = fixed_clock(0);
    let backend = Arc::new(TestBackend::default());
    let provider = email_provider(&backend, clock_fn);
    let tokens = provider
        .refresh(reqwest::Client::new(), "tok-7")
        .await
        .unwrap();
    assert_eq!(tokens.access_token.expose(), "test-access-0");
    assert_eq!(backend.refresh_seen(), vec!["tok-7".to_string()]);
}

#[gpui::test]
async fn sign_in_returns_the_scripted_outcome() {
    let (_clock, clock_fn) = fixed_clock(0);
    let backend = Arc::new(TestBackend::default());
    let provider = email_provider(&backend, clock_fn);
    provider
        .sign_up(
            reqwest::Client::new(),
            "user@example.test",
            &SecretString::new("password-8"),
        )
        .await
        .unwrap();
    match provider
        .sign_in(
            reqwest::Client::new(),
            "user@example.test",
            &SecretString::new("password-8"),
        )
        .await
        .unwrap()
    {
        SignInOutcome::Authenticated(session) => {
            assert_eq!(session.tokens.provider_id, EMAIL_PROVIDER_ID);
            assert_eq!(session.status.email, "user@example.test");
        }
        SignInOutcome::TotpRequired(_) => panic!("expected an authenticated session"),
    }
}

#[gpui::test]
async fn the_injected_clock_supplies_now_unix() {
    let (_clock, clock_fn) = fixed_clock(123_456);
    let backend = Arc::new(TestBackend::default());
    let provider = email_provider(&backend, clock_fn);
    provider
        .sign_up(
            reqwest::Client::new(),
            "user@example.test",
            &SecretString::new("password-8"),
        )
        .await
        .unwrap();
    provider
        .sign_in(
            reqwest::Client::new(),
            "user@example.test",
            &SecretString::new("password-8"),
        )
        .await
        .unwrap();
    assert_eq!(backend.observed_now(), vec![123_456, 123_456]);
}
