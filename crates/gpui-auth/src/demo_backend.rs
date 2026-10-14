use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use argon2::{Algorithm, Argon2, Params, Version};
use base64::Engine as _;
use hmac::{Hmac, Mac};
use sha1::Sha1;

use crate::email::{
    AuthBackend, AuthenticatedSession, EMAIL_PROVIDER_ID, EmailAccountStatus, SignInOutcome,
    TotpChallenge, TotpEnrollment,
};
use crate::error::AuthError;
use crate::token::{AccountInfo, SecretString, TokenSet};

pub const ACCESS_TTL_SECS: i64 = 900;
pub const CHALLENGE_TTL_SECS: i64 = 300;

const URL_SAFE_NO_PAD: base64::engine::GeneralPurpose =
    base64::engine::general_purpose::URL_SAFE_NO_PAD;
const BASE32_ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
const TOTP_STEP_SECS: i64 = 30;
const TOTP_SKEW_STEPS: i64 = 1;
const TOTP_ISSUER: &str = "gpui-starter";

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

fn hotp_u32(secret: &[u8], counter: u64) -> u32 {
    let mut mac = Hmac::<Sha1>::new_from_slice(secret).expect("hmac accepts any key length");
    mac.update(&counter.to_be_bytes());
    let digest = mac.finalize().into_bytes();
    let offset = usize::from(digest[19] & 0x0f);
    (((u32::from(digest[offset]) & 0x7f) << 24)
        | (u32::from(digest[offset + 1]) << 16)
        | (u32::from(digest[offset + 2]) << 8)
        | u32::from(digest[offset + 3]))
        % 1_000_000
}

fn totp_code(secret: &[u8], unix: i64) -> String {
    format!(
        "{:06}",
        hotp_u32(secret, unix.div_euclid(TOTP_STEP_SECS) as u64)
    )
}

fn base32_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len().div_ceil(5) * 8);
    let mut buffer = 0u32;
    let mut bits = 0u32;
    for byte in bytes {
        buffer = (buffer << 8) | u32::from(*byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            out.push(BASE32_ALPHABET[((buffer >> bits) & 0x1f) as usize] as char);
        }
    }
    if bits > 0 {
        out.push(BASE32_ALPHABET[((buffer << (5 - bits)) & 0x1f) as usize] as char);
    }
    out
}

struct TotpSecret([u8; 20]);

impl TotpSecret {
    fn verify(&self, code: &str, unix: i64) -> bool {
        (-TOTP_SKEW_STEPS..=TOTP_SKEW_STEPS).any(|delta| {
            constant_time_eq(
                totp_code(&self.0, unix + delta * TOTP_STEP_SECS).as_bytes(),
                code.as_bytes(),
            )
        })
    }

    fn otpauth_url(&self, email: &str) -> Result<String, AuthError> {
        let mut url = url::Url::parse(&format!("otpauth://totp/{TOTP_ISSUER}:{email}"))
            .map_err(|err| AuthError::InvalidTokenResponse(err.to_string()))?;
        url.query_pairs_mut()
            .append_pair("secret", &base32_encode(&self.0))
            .append_pair("issuer", TOTP_ISSUER)
            .append_pair("algorithm", "SHA1")
            .append_pair("digits", "6")
            .append_pair("period", "30");
        Ok(url.to_string())
    }
}

struct DemoUser {
    password: String,
    verified: bool,
    totp: Option<TotpSecret>,
}

struct DemoSession {
    email: String,
    account_id: String,
    access: String,
    access_exp: i64,
}

struct DemoChallenge {
    email: String,
    expires_at: i64,
}

pub struct DemoBackend {
    users: Mutex<HashMap<String, DemoUser>>,
    sessions: Mutex<HashMap<String, DemoSession>>,
    by_access: Mutex<HashMap<String, String>>,
    challenges: Mutex<HashMap<String, DemoChallenge>>,
    pending_totp: Mutex<HashMap<String, TotpSecret>>,
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
            challenges: Mutex::new(HashMap::new()),
            pending_totp: Mutex::new(HashMap::new()),
            seq: AtomicU64::new(0),
            argon2: Argon2::new(Algorithm::default(), Version::default(), params),
        }
    }

    fn next_seq(&self) -> u64 {
        self.seq.fetch_add(1, Ordering::SeqCst)
    }

    fn hash_password(&self, password: &SecretString) -> Result<String, AuthError> {
        let mut salt = [0u8; 16];
        getrandom::fill(&mut salt).map_err(|err| AuthError::Entropy(err.to_string()))?;
        let mut hash = [0u8; 32];
        self.argon2
            .hash_password_into(password.expose().as_bytes(), &salt, &mut hash)
            .map_err(|err| AuthError::Entropy(err.to_string()))?;
        Ok(format!(
            "v1${}${}",
            URL_SAFE_NO_PAD.encode(salt),
            URL_SAFE_NO_PAD.encode(hash)
        ))
    }

    fn verify_password(&self, password: &SecretString, stored: &str) -> bool {
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
        self.argon2
            .hash_password_into(password.expose().as_bytes(), &salt, &mut actual)
            .is_ok()
            && constant_time_eq(&actual, &expected)
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

    fn create_session(&self, email: &str, now: i64) -> TokenSet {
        let seq = self.next_seq();
        let access = format!("demo-access-{seq}");
        let refresh = format!("demo-refresh-{seq}");
        let access_exp = now + ACCESS_TTL_SECS;
        let session = DemoSession {
            email: email.to_string(),
            account_id: seq.to_string(),
            access: access.clone(),
            access_exp,
        };
        lock(&self.sessions).insert(refresh.clone(), session);
        lock(&self.by_access).insert(access.clone(), refresh.clone());
        token_set(&access, &refresh, access_exp, email, &seq.to_string())
    }

    fn sign_up_impl(
        &self,
        email: &str,
        password: &SecretString,
        _now: i64,
    ) -> Result<EmailAccountStatus, AuthError> {
        let email = validate_email(email)?;
        validate_password(password)?;
        let mut users = lock(&self.users);
        if users.contains_key(&email) {
            return Err(AuthError::EmailTaken);
        }
        let password = self.hash_password(password)?;
        users.insert(
            email.clone(),
            DemoUser {
                password,
                verified: false,
                totp: None,
            },
        );
        Ok(EmailAccountStatus {
            email,
            verified: false,
        })
    }

    fn sign_in_impl(
        &self,
        email: &str,
        password: &SecretString,
        now: i64,
    ) -> Result<SignInOutcome, AuthError> {
        let email = validate_email(email)?;
        let totp_enrolled = {
            let users = lock(&self.users);
            let user = users.get(&email).ok_or(AuthError::InvalidCredentials)?;
            if !self.verify_password(password, &user.password) {
                return Err(AuthError::InvalidCredentials);
            }
            user.totp.is_some()
        };
        if totp_enrolled {
            let id = format!("demo-ch-{}", self.next_seq());
            lock(&self.challenges).insert(
                id.clone(),
                DemoChallenge {
                    email: email.clone(),
                    expires_at: now + CHALLENGE_TTL_SECS,
                },
            );
            return Ok(SignInOutcome::TotpRequired(TotpChallenge { id, email }));
        }
        let tokens = self.create_session(&email, now);
        let status = self.account_status(&email);
        Ok(SignInOutcome::Authenticated(AuthenticatedSession {
            tokens,
            status,
        }))
    }

    fn verify_totp_impl(
        &self,
        challenge: &TotpChallenge,
        code: &str,
        now: i64,
    ) -> Result<AuthenticatedSession, AuthError> {
        let email = {
            let challenges = lock(&self.challenges);
            let entry = challenges
                .get(&challenge.id)
                .ok_or(AuthError::InvalidChallenge)?;
            if now >= entry.expires_at {
                return Err(AuthError::InvalidChallenge);
            }
            entry.email.clone()
        };
        let code_ok = {
            let users = lock(&self.users);
            users
                .get(&email)
                .and_then(|user| user.totp.as_ref())
                .is_some_and(|totp| totp.verify(code, now))
        };
        if !code_ok {
            return Err(AuthError::TotpFailed);
        }
        lock(&self.challenges).remove(&challenge.id);
        let tokens = self.create_session(&email, now);
        let status = self.account_status(&email);
        Ok(AuthenticatedSession { tokens, status })
    }

    fn begin_totp_enrollment_impl(&self, email: &str) -> Result<TotpEnrollment, AuthError> {
        let email = validate_email(email)?;
        {
            let users = lock(&self.users);
            let user = users.get(&email).ok_or(AuthError::InvalidCredentials)?;
            if user.totp.is_some() {
                return Err(AuthError::InvalidChallenge);
            }
        }
        let mut secret = [0u8; 20];
        getrandom::fill(&mut secret).map_err(|err| AuthError::Entropy(err.to_string()))?;
        let totp = TotpSecret(secret);
        let enrollment = TotpEnrollment {
            secret_base32: base32_encode(&secret),
            otpauth_url: totp.otpauth_url(&email)?,
            email: email.clone(),
        };
        lock(&self.pending_totp).insert(email, totp);
        Ok(enrollment)
    }

    fn confirm_totp_enrollment_impl(
        &self,
        email: &str,
        code: &str,
        now: i64,
    ) -> Result<(), AuthError> {
        let email = validate_email(email)?;
        let pending = lock(&self.pending_totp)
            .remove(&email)
            .ok_or(AuthError::InvalidChallenge)?;
        if !pending.verify(code, now) {
            lock(&self.pending_totp).insert(email, pending);
            return Err(AuthError::TotpFailed);
        }
        if let Some(user) = lock(&self.users).get_mut(&email) {
            user.totp = Some(pending);
        }
        Ok(())
    }

    fn refresh_impl(&self, refresh_token: &str, now: i64) -> Result<TokenSet, AuthError> {
        let seq = self.next_seq();
        let access = format!("demo-access-{seq}");
        let (old_access, email, account_id, access_exp) = {
            let mut sessions = lock(&self.sessions);
            let session = sessions
                .get_mut(refresh_token)
                .ok_or(AuthError::RefreshRejected)?;
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
        now_unix: i64,
    ) -> Result<EmailAccountStatus, AuthError> {
        self.sign_up_impl(email, password, now_unix)
    }

    async fn sign_in(
        &self,
        _http: reqwest::Client,
        email: &str,
        password: &SecretString,
        now_unix: i64,
    ) -> Result<SignInOutcome, AuthError> {
        self.sign_in_impl(email, password, now_unix)
    }

    async fn verify_totp(
        &self,
        _http: reqwest::Client,
        challenge: &TotpChallenge,
        code: &str,
        now_unix: i64,
    ) -> Result<AuthenticatedSession, AuthError> {
        self.verify_totp_impl(challenge, code, now_unix)
    }

    async fn begin_totp_enrollment(
        &self,
        _http: reqwest::Client,
        email: &str,
    ) -> Result<TotpEnrollment, AuthError> {
        self.begin_totp_enrollment_impl(email)
    }

    async fn confirm_totp_enrollment(
        &self,
        _http: reqwest::Client,
        email: &str,
        code: &str,
        now_unix: i64,
    ) -> Result<(), AuthError> {
        self.confirm_totp_enrollment_impl(email, code, now_unix)
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
