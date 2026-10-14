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
}
