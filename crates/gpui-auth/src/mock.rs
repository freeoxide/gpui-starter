use std::sync::atomic::{AtomicU64, Ordering};

use crate::error::AuthError;
use crate::pkce::{PkcePair, random_state};
use crate::provider::{AuthProvider, FlowLaunch, RedirectParams};
use crate::token::{AccountInfo, RefreshPolicy, SecretString, TokenSet};

/// Scripted provider (id `"mock"`) for demos and tests: real random state
/// and PKCE, deterministic tokens, no HTTP anywhere. Its `urn:` authorize URL
/// is deliberately not http(s) so nothing can trip the OS browser.
pub struct MockProvider {
    counter: AtomicU64,
}

impl MockProvider {
    pub fn new() -> Self {
        Self {
            counter: AtomicU64::new(0),
        }
    }

    fn mint(&self, access_seq: u64, refresh_token: SecretString) -> TokenSet {
        TokenSet {
            provider_id: "mock".to_string(),
            access_token: SecretString::new(format!("mock-access-{access_seq}")),
            refresh_token: Some(refresh_token),
            id_token: None,
            token_type: "Bearer".to_string(),
            expires_at: None,
            scope: None,
            account: Some(AccountInfo {
                id: None,
                email: None,
                display_name: Some("Mock account".to_string()),
            }),
        }
    }
}

impl Default for MockProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
impl AuthProvider for MockProvider {
    fn id(&self) -> &str {
        "mock"
    }

    fn display_name(&self) -> &str {
        "Mock"
    }

    fn refresh_policy(&self) -> RefreshPolicy {
        RefreshPolicy::ReuseUntilDead
    }

    async fn start_flow(&self, _http: reqwest::Client) -> Result<FlowLaunch, AuthError> {
        let state = random_state()?;
        let authorize_url = format!("urn:gpui-auth:mock:{}", state.expose());
        Ok(FlowLaunch {
            authorize_url,
            pkce: Some(PkcePair::generate()?),
            redirect_uri: "urn:gpui-auth:mock:redirect".to_string(),
            state,
        })
    }

    async fn complete_callback(
        &self,
        _http: reqwest::Client,
        redirect: RedirectParams,
        _launch: &FlowLaunch,
    ) -> Result<TokenSet, AuthError> {
        if redirect.code.is_none() {
            return Err(AuthError::ProviderDenied {
                error: "missing_code".to_string(),
                description: None,
            });
        }
        let seq = self.counter.fetch_add(1, Ordering::SeqCst);
        Ok(self.mint(seq, SecretString::new(format!("mock-refresh-{seq}"))))
    }

    async fn refresh(
        &self,
        _http: reqwest::Client,
        refresh_token: &str,
    ) -> Result<TokenSet, AuthError> {
        let seq = self.counter.fetch_add(1, Ordering::SeqCst);
        Ok(self.mint(seq, SecretString::new(refresh_token)))
    }

    async fn revoke(&self, _http: reqwest::Client, _access_token: &str) -> Result<(), AuthError> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "mock.test.rs"]
mod mock_test;
