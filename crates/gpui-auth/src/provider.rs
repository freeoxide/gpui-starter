use crate::error::AuthError;
use crate::pkce::PkcePair;
use crate::token::{SecretString, TokenSet};

pub use crate::token::RefreshPolicy;

/// Executor + HTTP client shared by controller spawns. Clone it freely.
#[derive(Clone)]
pub struct AuthIo {
    pub http: reqwest::Client,
    #[cfg(not(target_family = "wasm"))]
    pub runtime: Option<std::sync::Arc<tokio::runtime::Runtime>>,
}

impl AuthIo {
    /// Native: run `fut` on the tokio runtime when present (`Send + 'static`
    /// required by `rt.spawn`); `None` falls through to a direct await (tests
    /// and mock-only setups). A joined-task panic propagates.
    #[cfg(not(target_family = "wasm"))]
    pub async fn drive<F: Future + Send + 'static>(&self, fut: F) -> F::Output
    where
        F::Output: Send,
    {
        match &self.runtime {
            Some(runtime) => runtime
                .spawn(fut)
                .await
                .expect("auth runtime task panicked"),
            None => fut.await,
        }
    }

    /// Wasm: direct await, no Send bound — reqwest fetch futures bottom out
    /// in `JsFuture` (Rc-based, `!Send`) and GPUI's executor takes them.
    #[cfg(target_family = "wasm")]
    pub async fn drive<F: Future>(&self, fut: F) -> F::Output {
        fut.await
    }
}

/// What a provider returns from `start_flow`; the controller matches `state`
/// on callback and re-sends `redirect_uri` at exchange (RFC 8252 §8.1).
#[derive(Clone)]
pub struct FlowLaunch {
    pub authorize_url: String,
    pub state: SecretString,
    pub pkce: Option<PkcePair>,
    pub redirect_uri: String,
}

/// The OAuth/OIDC query parameters carried on a redirect URL.
pub struct RedirectParams {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

impl RedirectParams {
    /// Parse the query of any redirect URL (scheme-agnostic; the app decides
    /// which URLs are auth callbacks). Unknown keys are ignored.
    pub fn from_url(raw: &str) -> Result<Self, AuthError> {
        let url =
            url::Url::parse(raw).map_err(|err| AuthError::InvalidRedirect(err.to_string()))?;
        let mut params = Self {
            code: None,
            state: None,
            error: None,
            error_description: None,
        };
        for (key, value) in url.query_pairs() {
            match key.as_ref() {
                "code" => params.code = Some(value.into_owned()),
                "state" => params.state = Some(value.into_owned()),
                "error" => params.error = Some(value.into_owned()),
                "error_description" => params.error_description = Some(value.into_owned()),
                _ => {}
            }
        }
        Ok(params)
    }
}

// Native: Send-boxed futures (rt.spawn requires them). Wasm: (?Send) mode,
// because JsFuture-backed provider futures are !Send. The trait object stays
// Send + Sync on both targets.
#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
pub trait AuthProvider: Send + Sync {
    /// Stable provider identity; also the store key.
    fn id(&self) -> &str;
    /// Fallback account label when the provider reports no account fields.
    fn display_name(&self) -> &str;
    fn refresh_policy(&self) -> RefreshPolicy;
    /// Build the authorization request and return the launch. MUST set a
    /// fresh random state and, for browser flows, an S256 `PkcePair` from
    /// this crate's helpers.
    async fn start_flow(&self, http: reqwest::Client) -> Result<FlowLaunch, AuthError>;
    /// Validate the redirect against the launch (verifier, redirect_uri) and
    /// exchange the code for tokens. The controller has already exact-matched
    /// `redirect.state` against `launch.state`.
    async fn complete_callback(
        &self,
        http: reqwest::Client,
        redirect: RedirectParams,
        launch: &FlowLaunch,
    ) -> Result<TokenSet, AuthError>;
    async fn refresh(
        &self,
        http: reqwest::Client,
        refresh_token: &str,
    ) -> Result<TokenSet, AuthError>;
    async fn revoke(&self, http: reqwest::Client, access_token: &str) -> Result<(), AuthError>;
}

#[cfg(test)]
#[path = "provider.test.rs"]
mod provider_test;
