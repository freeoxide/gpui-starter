use base64::Engine as _;
use sha2::Digest as _;

use crate::error::AuthError;
use crate::token::SecretString;

const URL_SAFE_NO_PAD: base64::engine::GeneralPurpose =
    base64::engine::general_purpose::URL_SAFE_NO_PAD;

fn random_urlsafe_32() -> Result<String, AuthError> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|err| AuthError::Entropy(err.to_string()))?;
    Ok(URL_SAFE_NO_PAD.encode(bytes))
}

/// An RFC 7636 PKCE pair: a 43-char base64url verifier (held secret) and its
/// S256 challenge.
#[derive(Clone)]
pub struct PkcePair {
    verifier: SecretString,
    challenge: String,
}

impl PkcePair {
    /// 32 bytes of CSPRNG entropy -> 43-char base64url verifier + S256
    /// challenge.
    pub fn generate() -> Result<Self, AuthError> {
        let verifier = SecretString::new(random_urlsafe_32()?);
        let challenge = authorize_challenge(verifier.expose());
        Ok(Self {
            verifier,
            challenge,
        })
    }

    /// base64url(SHA-256(verifier)), no padding.
    pub fn challenge(&self) -> &str {
        &self.challenge
    }

    /// Read the verifier; call only at the token-exchange boundary.
    pub fn expose_verifier(&self) -> &str {
        self.verifier.expose()
    }
}

/// Deterministic S256 for callers holding their own verifier bytes.
pub fn authorize_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(sha2::Sha256::digest(verifier.as_bytes()))
}

/// 32 bytes of CSPRNG entropy -> 43-char base64url string (state values).
pub fn random_state() -> Result<SecretString, AuthError> {
    random_urlsafe_32().map(SecretString::new)
}

#[cfg(test)]
#[path = "pkce.test.rs"]
mod pkce_test;
