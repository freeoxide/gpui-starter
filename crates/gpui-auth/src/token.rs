use serde::{Deserialize, Deserializer, Serialize, Serializer};
use zeroize::ZeroizeOnDrop;

/// A string that never prints: `Debug` renders `[redacted]` and the value is
/// wiped on drop. Serde is transparent because the store's backing (an OS
/// keyring row) is itself access-controlled.
#[derive(Eq, ZeroizeOnDrop)]
pub struct SecretString(String);

impl SecretString {
    /// Wrap a secret value; the copy and the original both wipe on drop.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Read the secret; call only at the wire/store boundary.
    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl Clone for SecretString {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl std::fmt::Debug for SecretString {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("SecretString").field(&"[redacted]").finish()
    }
}

/// Compares inner values; not constant-time — a single-use 256-bit value
/// compared once needs no timing defense.
impl PartialEq for SecretString {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Serialize for SecretString {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for SecretString {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self)
    }
}

/// Identity fields for the signed-in account, when the provider supplies them.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AccountInfo {
    pub id: Option<String>,
    pub email: Option<String>,
    pub display_name: Option<String>,
}

/// A provider's tokens plus the metadata needed to use them. `expires_at` is
/// unix seconds, UTC; `None` means no expiry was advertised.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct TokenSet {
    pub provider_id: String,
    pub access_token: SecretString,
    pub refresh_token: Option<SecretString>,
    pub id_token: Option<SecretString>,
    pub token_type: String,
    pub expires_at: Option<i64>,
    pub scope: Option<String>,
    pub account: Option<AccountInfo>,
}

impl TokenSet {
    /// True at or past `expires_at`; false when no expiry is advertised.
    pub fn is_expired(&self, now_unix: i64) -> bool {
        self.expires_at.is_some_and(|t| now_unix >= t)
    }

    /// True when `expires_at` falls within `margin_secs` of `now_unix`.
    pub fn needs_refresh(&self, now_unix: i64, margin_secs: i64) -> bool {
        self.expires_at
            .is_some_and(|t| now_unix.saturating_add(margin_secs) >= t)
    }

    /// Log-safe one-liner: ids, scope, and expiry only, never token values.
    pub fn redacted_summary(&self) -> String {
        let mut out = format!(
            "provider_id={} token_type={}",
            self.provider_id, self.token_type
        );
        if let Some(scope) = &self.scope {
            out.push_str(&format!(" scope={scope}"));
        }
        if let Some(expires_at) = self.expires_at {
            out.push_str(&format!(" expires_at={expires_at}"));
        }
        if let Some(account) = &self.account {
            if let Some(id) = &account.id {
                out.push_str(&format!(" account_id={id}"));
            }
            if let Some(email) = &account.email {
                out.push_str(&format!(" account_email={email}"));
            }
        }
        out
    }
}

/// Per-provider refresh behavior (RFC 9700 framing).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RefreshPolicy {
    Rotated,
    ReuseUntilDead,
    None,
}

impl RefreshPolicy {
    /// Fold a refresh response into the stored set: `Rotated` takes the new
    /// set verbatim; `ReuseUntilDead` keeps the old refresh token when the
    /// response omits one; `None` never reaches merge (refresh is refused).
    pub fn merge(self, old: &TokenSet, new: TokenSet) -> TokenSet {
        match self {
            Self::Rotated => new,
            Self::ReuseUntilDead => {
                let mut merged = new;
                if merged.refresh_token.is_none() {
                    merged.refresh_token = old.refresh_token.clone();
                }
                merged
            }
            Self::None => new,
        }
    }
}

#[cfg(test)]
#[path = "token.test.rs"]
mod token_test;
