use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use argon2::{Algorithm, Argon2, Params, Version};
use base64::Engine as _;

use crate::email::{AuthBackend, AuthenticatedSession, EMAIL_PROVIDER_ID, EmailAccountStatus};
use crate::error::AuthError;
use crate::token::{AccountInfo, SecretString, TokenSet};

pub const ACCESS_TTL_SECS: i64 = 900;
const SESSION_MAX_AGE_SECS: i64 = 30 * 24 * 60 * 60;

const URL_SAFE_NO_PAD: base64::engine::GeneralPurpose =
    base64::engine::general_purpose::URL_SAFE_NO_PAD;

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().expect("demo backend lock poisoned")
}

fn argon_params(m_cost: u32, t_cost: u32, p_cost: u32) -> Params {
    Params::new(m_cost, t_cost, p_cost, None).expect("cost parameters within argon2 floors")
}

fn validate_email(email: &str) -> Result<String, AuthError> {
    let email = email.trim().to_lowercase();
    let shape_valid = (3..=254).contains(&email.len())
        && email.matches('@').count() == 1
        && email.split_once('@').is_some_and(|(local, domain)| {
            !local.is_empty() && !domain.is_empty() && domain.contains('.')
        });
    if shape_valid {
        Ok(email)
    } else {
        Err(AuthError::InvalidEmail)
    }
}

fn validate_password(password: &SecretString) -> Result<(), AuthError> {
    if (8..=128).contains(&password.expose().len()) {
        Ok(())
    } else {
        Err(AuthError::WeakPassword)
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    a.len() == b.len() && a.iter().zip(b).fold(0u8, |diff, (x, y)| diff | (x ^ y)) == 0
}

fn random_token(prefix: &str) -> Result<String, AuthError> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|err| AuthError::Entropy(err.to_string()))?;
    Ok(format!("{prefix}-{}", URL_SAFE_NO_PAD.encode(bytes)))
}

struct DemoUser {
    id: String,
    password: String,
    verified: bool,
}

struct DemoSession {
    email: String,
    account_id: String,
    access: String,
    access_exp: i64,
    created_at: i64,
}

pub struct DemoBackend {
    users: Mutex<HashMap<String, DemoUser>>,
    sessions: Mutex<HashMap<String, DemoSession>>,
    by_access: Mutex<HashMap<String, String>>,
    seq: AtomicU64,
    argon2: Argon2<'static>,
}

fn token_set(
    access: &str,
    refresh: &str,
    expires_at: i64,
    email: &str,
    account_id: &str,
) -> TokenSet {
    TokenSet {
        provider_id: EMAIL_PROVIDER_ID.to_string(),
        access_token: SecretString::new(access),
        refresh_token: Some(SecretString::new(refresh)),
        id_token: None,
        token_type: "Bearer".to_string(),
        expires_at: Some(expires_at),
        scope: None,
        account: Some(AccountInfo {
            id: Some(account_id.to_string()),
            email: Some(email.to_string()),
            display_name: Some(email.to_string()),
            avatar_url: None,
        }),
    }
}

impl DemoBackend {
    /// 8 MiB / 2 passes / p=1 Argon2id: a real hash, tolerable in dev.
    pub fn new() -> Self {
        Self::with_params(argon_params(8 * 1024, 2, 1))
    }

    /// Custom cost parameters; tests run at the 8 KiB floor for speed.
    pub fn with_params(params: Params) -> Self {
        Self {
            users: Mutex::new(HashMap::new()),
            sessions: Mutex::new(HashMap::new()),
            by_access: Mutex::new(HashMap::new()),
            seq: AtomicU64::new(0),
            argon2: Argon2::new(Algorithm::default(), Version::default(), params),
        }
    }

    fn next_seq(&self) -> u64 {
        self.seq.fetch_add(1, Ordering::SeqCst)
    }

    fn hash_with(argon2: &Argon2<'static>, password: &SecretString) -> Result<String, AuthError> {
        let mut salt = [0u8; 16];
        getrandom::fill(&mut salt).map_err(|err| AuthError::Entropy(err.to_string()))?;
        let mut hash = [0u8; 32];
        argon2
            .hash_password_into(password.expose().as_bytes(), &salt, &mut hash)
            .map_err(|err| AuthError::Entropy(err.to_string()))?;
        Ok(format!(
            "v1${}${}",
            URL_SAFE_NO_PAD.encode(salt),
            URL_SAFE_NO_PAD.encode(hash)
        ))
    }

    fn verify_with(argon2: &Argon2<'static>, password: &SecretString, stored: &str) -> bool {
        let Some((salt_b64, hash_b64)) = stored
            .strip_prefix("v1$")
            .and_then(|rest| rest.split_once('$'))
        else {
            return false;
        };
        let (Ok(salt), Ok(expected)) = (
            URL_SAFE_NO_PAD.decode(salt_b64),
            URL_SAFE_NO_PAD.decode(hash_b64),
        ) else {
            return false;
        };
        let mut actual = vec![0u8; expected.len()];
        argon2
            .hash_password_into(password.expose().as_bytes(), &salt, &mut actual)
            .is_ok()
            && constant_time_eq(&actual, &expected)
    }

    /// Argon2id is deliberate CPU work; keep it off the async worker. With no
    /// ambient runtime (tests, GPUI-executor fallback) it runs inline.
    async fn argon2_offload<T>(work: impl FnOnce() -> T + Send + 'static) -> Result<T, AuthError>
    where
        T: Send + 'static,
    {
        match tokio::runtime::Handle::try_current() {
            Ok(handle) => handle
                .spawn_blocking(work)
                .await
                .map_err(|err| AuthError::Entropy(err.to_string())),
            Err(_) => Ok(work()),
        }
    }

    async fn hash_password(&self, password: SecretString) -> Result<String, AuthError> {
        let argon2 = self.argon2.clone();
        Self::argon2_offload(move || Self::hash_with(&argon2, &password)).await?
    }

    async fn verify_password(
        &self,
        password: SecretString,
        stored: String,
    ) -> Result<bool, AuthError> {
        let argon2 = self.argon2.clone();
        Self::argon2_offload(move || Self::verify_with(&argon2, &password, &stored)).await
    }

    fn account_status(&self, email: &str) -> EmailAccountStatus {
        let verified = lock(&self.users)
            .get(email)
            .map(|user| user.verified)
            .unwrap_or(false);
        EmailAccountStatus {
            email: email.to_string(),
            verified,
        }
    }

    fn create_session(
        &self,
        email: &str,
        account_id: &str,
        now: i64,
    ) -> Result<TokenSet, AuthError> {
        let access = random_token("demo-access")?;
        let refresh = random_token("demo-refresh")?;
        let access_exp = now + ACCESS_TTL_SECS;
        self.sweep_expired_sessions(now);
        lock(&self.sessions).insert(
            refresh.clone(),
            DemoSession {
                email: email.to_string(),
                account_id: account_id.to_string(),
                access: access.clone(),
                access_exp,
                created_at: now,
            },
        );
        lock(&self.by_access).insert(access.clone(), refresh.clone());
        Ok(token_set(&access, &refresh, access_exp, email, account_id))
    }

    /// Sessions have no other reaper; sweep on insert so abandoned rows
    /// cannot accumulate for the process lifetime.
    fn sweep_expired_sessions(&self, now: i64) {
        let live: HashSet<String> = {
            let mut sessions = lock(&self.sessions);
            sessions.retain(|_, session| now < session.created_at + SESSION_MAX_AGE_SECS);
            sessions.keys().cloned().collect()
        };
        lock(&self.by_access).retain(|_, refresh| live.contains(refresh.as_str()));
    }

    fn refresh_impl(&self, refresh_token: &str, now: i64) -> Result<TokenSet, AuthError> {
        let access = random_token("demo-access")?;
        let (old_access, email, account_id, access_exp) = {
            let mut sessions = lock(&self.sessions);
            if sessions
                .get(refresh_token)
                .is_some_and(|session| now >= session.created_at + SESSION_MAX_AGE_SECS)
            {
                sessions.remove(refresh_token);
                drop(sessions);
                lock(&self.by_access).retain(|_, refresh| refresh.as_str() != refresh_token);
                return Err(AuthError::RefreshRejected);
            }
            let Some(session) = sessions.get_mut(refresh_token) else {
                return Err(AuthError::RefreshRejected);
            };
            let old_access = session.access.clone();
            session.access = access.clone();
            session.access_exp = now + ACCESS_TTL_SECS;
            (
                old_access,
                session.email.clone(),
                session.account_id.clone(),
                session.access_exp,
            )
        };
        let mut by_access = lock(&self.by_access);
        by_access.remove(&old_access);
        by_access.insert(access.clone(), refresh_token.to_string());
        Ok(token_set(
            &access,
            refresh_token,
            access_exp,
            &email,
            &account_id,
        ))
    }

    fn revoke_impl(&self, token: &str) {
        if let Some(refresh) = lock(&self.by_access).remove(token) {
            lock(&self.sessions).remove(&refresh);
        }
    }
}

impl Default for DemoBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl AuthBackend for DemoBackend {
    fn id(&self) -> &str {
        "demo"
    }

    async fn sign_up(
        &self,
        _http: reqwest::Client,
        email: &str,
        password: &SecretString,
        _now_unix: i64,
    ) -> Result<EmailAccountStatus, AuthError> {
        let email = validate_email(email)?;
        validate_password(password)?;
        // Hash before taking the lock so concurrent map reads never stall
        // on Argon2.
        let hash = self.hash_password(password.clone()).await?;
        let mut users = lock(&self.users);
        if users.contains_key(&email) {
            return Err(AuthError::EmailTaken);
        }
        users.insert(
            email.clone(),
            DemoUser {
                id: self.next_seq().to_string(),
                password: hash,
                verified: false,
            },
        );
        Ok(EmailAccountStatus {
            email,
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
        let email = validate_email(email)?;
        let (account_id, stored_hash) = {
            let users = lock(&self.users);
            let user = users.get(&email).ok_or(AuthError::InvalidCredentials)?;
            (user.id.clone(), user.password.clone())
        };
        // Verify against the cloned hash, outside the users lock.
        if !self.verify_password(password.clone(), stored_hash).await? {
            return Err(AuthError::InvalidCredentials);
        }
        let tokens = self.create_session(&email, &account_id, now_unix)?;
        let status = self.account_status(&email);
        Ok(AuthenticatedSession { tokens, status })
    }

    async fn refresh(
        &self,
        _http: reqwest::Client,
        refresh_token: &str,
        now_unix: i64,
    ) -> Result<TokenSet, AuthError> {
        self.refresh_impl(refresh_token, now_unix)
    }

    async fn revoke(&self, _http: reqwest::Client, token: &str) -> Result<(), AuthError> {
        self.revoke_impl(token);
        Ok(())
    }
}

#[cfg(test)]
#[path = "demo_backend.test.rs"]
mod demo_backend_test;
