use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use super::*;
use crate::error::AuthError;
use crate::token::{AccountInfo, SecretString, TokenSet};

pub(crate) const TEST_TTL_SECS: i64 = 900;

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
    refresh_rejected: AtomicBool,
    refresh_omits_refresh_token: AtomicBool,
    revoke_fails: AtomicBool,
    observed_now: Mutex<Vec<i64>>,
    refresh_seen: Mutex<Vec<String>>,
}

impl TestBackend {
    pub(crate) fn reject_refresh(&self) {
        self.refresh_rejected.store(true, Ordering::SeqCst);
    }

    /// Next refresh response carries no `refresh_token`.
    pub(crate) fn omit_refresh_token_once(&self) {
        self.refresh_omits_refresh_token
            .store(true, Ordering::SeqCst);
    }

    /// Next revoke call fails; sign-out must still complete.
    pub(crate) fn fail_revoke_once(&self) {
        self.revoke_fails.store(true, Ordering::SeqCst);
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

    fn mint(&self, email: &str, refresh_token: Option<&str>, now_unix: i64) -> TokenSet {
        let seq = self.seq.fetch_add(1, Ordering::SeqCst);
        TokenSet {
            provider_id: EMAIL_PROVIDER_ID.to_string(),
            access_token: SecretString::new(format!("test-access-{seq}")),
            refresh_token: refresh_token.map(SecretString::new),
            id_token: None,
            token_type: "Bearer".to_string(),
            expires_at: Some(now_unix + TEST_TTL_SECS),
            scope: None,
            account: Some(AccountInfo {
                id: Some(seq.to_string()),
                email: Some(email.to_string()),
                display_name: Some(email.to_string()),
                avatar_url: None,
            }),
        }
    }

    fn session(&self, email: &str, refresh_token: &str, now_unix: i64) -> AuthenticatedSession {
        AuthenticatedSession {
            tokens: self.mint(email, Some(refresh_token), now_unix),
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
    ) -> Result<AuthenticatedSession, AuthError> {
        self.observe(now_unix);
        if !self.credentials_match(email, password) {
            return Err(AuthError::InvalidCredentials);
        }
        let refresh = format!("test-refresh-{}", self.seq.load(Ordering::SeqCst));
        Ok(self.session(email, &refresh, now_unix))
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
        let refresh_token = if self
            .refresh_omits_refresh_token
            .swap(false, Ordering::SeqCst)
        {
            None
        } else {
            Some(refresh_token)
        };
        Ok(self.mint("test@example.test", refresh_token, now_unix))
    }

    async fn revoke(&self, _http: reqwest::Client, token: &str) -> Result<(), AuthError> {
        self.refresh_seen
            .lock()
            .unwrap()
            .push(format!("revoke:{token}"));
        if self.revoke_fails.swap(false, Ordering::SeqCst) {
            return Err(AuthError::Network("revoke down".to_string()));
        }
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
    let launch = FlowLaunch {
        authorize_url: String::new(),
        state: SecretString::new("state"),
        pkce: None,
        redirect_uri: String::new(),
    };
    let redirect = RedirectParams {
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
async fn sign_in_returns_the_scripted_session() {
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
    let session = provider
        .sign_in(
            reqwest::Client::new(),
            "user@example.test",
            &SecretString::new("password-8"),
        )
        .await
        .unwrap();
    assert_eq!(session.tokens.provider_id, EMAIL_PROVIDER_ID);
    assert_eq!(session.status.email, "user@example.test");
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
