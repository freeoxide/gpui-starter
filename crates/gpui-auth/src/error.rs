/// Failures surfaced by the auth foundation: flow lifecycle, provider
/// responses, storage, network, entropy, and redirect parsing.
#[derive(Clone, Debug, thiserror::Error)]
pub enum AuthError {
    #[error("no pending authorization flow")]
    NoPendingFlow,
    #[error("state mismatch on authorization response")]
    StateMismatch,
    #[error("authorization failed: {error}")]
    ProviderDenied {
        error: String,
        description: Option<String>,
    },
    #[error("token response invalid: {0}")]
    InvalidTokenResponse(String),
    #[error("no refresh token available")]
    NoRefreshToken,
    #[error("token store failure: {0}")]
    Storage(String),
    #[error("network failure: {0}")]
    Network(String),
    #[error("randomness unavailable: {0}")]
    Entropy(String),
    #[error("invalid redirect url: {0}")]
    InvalidRedirect(String),
    #[error("invalid email address")]
    InvalidEmail,
    #[error("password does not meet the minimum policy")]
    WeakPassword,
    #[error("email already registered")]
    EmailTaken,
    #[error("invalid email or password")]
    InvalidCredentials,
    #[error("backend rejected the token")]
    RefreshRejected,
    #[error("provider has no browser authorization flow")]
    NoBrowserFlow,
    #[error("password auth is not available on this controller")]
    PasswordAuthUnavailable,
    #[error("no challenge is pending")]
    NoPendingChallenge,
    #[error("challenge unknown or expired")]
    InvalidChallenge,
    #[error("TOTP code rejected")]
    TotpFailed,
    #[error("TOTP not enrolled for this account")]
    TotpNotEnrolled,
}
