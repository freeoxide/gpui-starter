use std::borrow::Cow;
use std::sync::{Mutex, MutexGuard};

use gpui_auth::{
    AccountInfo, AuthError, AuthProvider, FlowLaunch, PkcePair, RedirectParams, RefreshPolicy,
    SecretString, TokenSet, random_state,
};
use openidconnect::core::{
    CoreAuthenticationFlow, CoreClient, CoreProviderMetadata, CoreTokenResponse,
};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointMaybeSet, EndpointNotSet,
    EndpointSet, IssuerUrl, Nonce, OAuth2TokenResponse, PkceCodeChallenge, PkceCodeVerifier,
    RedirectUrl, RefreshToken, Scope, TokenResponse,
};

use super::{PendingRedirect, map_token_error, start_redirect, token_set};
use crate::services::auth_social::{OidcConfig, ProviderConfig, ProviderKind, RedirectMode};

const GOOGLE_ISSUER: &str = "https://accounts.google.com";
const GOOGLE_REVOKE_URL: &str = "https://oauth2.googleapis.com/revoke";

type DiscoveredClient = CoreClient<
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

struct PendingOidcFlow {
    redirect: PendingRedirect,
    nonce: SecretString,
}

/// OIDC provider for Google (`google`) and any discovered issuer
/// (`generic`). `start_flow` binds the loopback listener, so it must run on
/// the tokio runtime the controller's `AuthIo::drive` provides.
pub struct OidcProvider {
    id: &'static str,
    display: &'static str,
    issuer: String,
    client_id: String,
    client_secret: Option<SecretString>,
    scopes: Vec<String>,
    redirect: RedirectMode,
    refresh_policy: RefreshPolicy,
    extra_auth_params: Vec<(String, String)>,
    revoke_url: Option<String>,
    pending: Mutex<Option<PendingOidcFlow>>,
}

impl OidcProvider {
    pub fn google(config: &ProviderConfig, redirect: RedirectMode) -> Self {
        Self {
            id: ProviderKind::Google.id(),
            display: ProviderKind::Google.display_name(),
            issuer: GOOGLE_ISSUER.to_string(),
            client_id: config.client_id.clone(),
            client_secret: config.client_secret.clone().map(SecretString::new),
            scopes: config
                .scopes
                .clone()
                .unwrap_or_else(|| vec!["openid".into(), "email".into(), "profile".into()]),
            redirect,
            refresh_policy: RefreshPolicy::ReuseUntilDead,
            extra_auth_params: vec![("access_type".into(), "offline".into())],
            revoke_url: Some(GOOGLE_REVOKE_URL.to_string()),
            pending: Mutex::new(None),
        }
    }

    pub fn generic(config: &OidcConfig, redirect: RedirectMode) -> Self {
        Self {
            id: ProviderKind::Oidc.id(),
            display: ProviderKind::Oidc.display_name(),
            issuer: config.issuer.clone(),
            client_id: config.base.client_id.clone(),
            client_secret: config.base.client_secret.clone().map(SecretString::new),
            scopes: config
                .base
                .scopes
                .clone()
                .unwrap_or_else(|| vec!["openid".into(), "profile".into(), "email".into()]),
            redirect,
            refresh_policy: RefreshPolicy::Rotated,
            extra_auth_params: Vec::new(),
            revoke_url: config.revoke_url.clone(),
            pending: Mutex::new(None),
        }
    }

    pub(crate) fn take_pending_redirect(
        &self,
    ) -> Option<(
        tokio::sync::oneshot::Receiver<String>,
        tokio::task::AbortHandle,
    )> {
        lock(&self.pending).as_mut()?.redirect.take_wait()
    }

    #[cfg(test)]
    pub(crate) fn pending_port(&self) -> Option<u16> {
        lock(&self.pending).as_ref()?.redirect.port
    }

    async fn discover_client(&self, http: &reqwest::Client) -> Result<DiscoveredClient, AuthError> {
        let issuer = IssuerUrl::new(self.issuer.clone())
            .map_err(|err| AuthError::InvalidTokenResponse(format!("invalid issuer url: {err}")))?;
        let metadata = CoreProviderMetadata::discover_async(issuer, http)
            .await
            .map_err(|err| AuthError::Network(err.to_string()))?;
        Ok(DiscoveredClient::from_provider_metadata(
            metadata,
            ClientId::new(self.client_id.clone()),
            self.client_secret
                .clone()
                .map(|secret| ClientSecret::new(secret.expose().to_string())),
        ))
    }

    async fn fetch_account(
        &self,
        client: &DiscoveredClient,
        http: &reqwest::Client,
        response: &CoreTokenResponse,
    ) -> Option<AccountInfo> {
        let request = client
            .user_info(response.access_token().clone(), None)
            .ok()?;
        let claims: openidconnect::core::CoreUserInfoClaims =
            match request.request_async(http).await {
                Ok(claims) => claims,
                Err(_) => return self.account_from_id_token(client, response).await,
            };
        Some(AccountInfo {
            id: Some(claims.subject().to_string()),
            email: claims.email().map(|email| email.to_string()),
            display_name: claims
                .name()
                .and_then(|name| name.get(None))
                .map(|name| name.to_string())
                .or_else(|| {
                    claims
                        .preferred_username()
                        .map(|username| username.to_string())
                }),
            avatar_url: claims
                .picture()
                .and_then(|picture| picture.get(None))
                .map(|picture| picture.to_string()),
        })
    }

    async fn account_from_id_token(
        &self,
        client: &DiscoveredClient,
        response: &CoreTokenResponse,
    ) -> Option<AccountInfo> {
        let id_token = response.id_token()?;
        let nonce = lock(&self.pending).as_ref()?.nonce.clone();
        let claims = id_token
            .claims(
                &client.id_token_verifier(),
                &Nonce::new(nonce.expose().to_string()),
            )
            .ok()?;
        Some(AccountInfo {
            id: Some(claims.subject().to_string()),
            email: claims.email().map(|email| email.to_string()),
            display_name: claims
                .name()
                .and_then(|name| name.get(None))
                .map(|name| name.to_string())
                .or_else(|| {
                    claims
                        .preferred_username()
                        .map(|username| username.to_string())
                }),
            avatar_url: claims
                .picture()
                .and_then(|picture| picture.get(None))
                .map(|picture| picture.to_string()),
        })
    }

    fn token_response_to_set(
        &self,
        response: &CoreTokenResponse,
        account: Option<AccountInfo>,
    ) -> TokenSet {
        let id_token = response
            .id_token()
            .and_then(|token| serde_json::to_value(token).ok())
            .and_then(|value| value.as_str().map(str::to_string));
        token_set(
            self.id,
            response.access_token().secret().to_string(),
            response
                .refresh_token()
                .map(|token| token.secret().to_string()),
            id_token,
            response.expires_in(),
            response.scopes().map(|scopes| {
                scopes
                    .iter()
                    .map(|scope| scope.to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            }),
            account,
        )
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[async_trait::async_trait]
impl AuthProvider for OidcProvider {
    fn id(&self) -> &str {
        self.id
    }

    fn display_name(&self) -> &str {
        self.display
    }

    fn refresh_policy(&self) -> RefreshPolicy {
        self.refresh_policy
    }

    async fn start_flow(&self, http: reqwest::Client) -> Result<FlowLaunch, AuthError> {
        let client = self.discover_client(&http).await?;
        let (redirect, redirect_uri) = start_redirect(self.redirect)?;
        let redirect_url = RedirectUrl::new(redirect_uri.clone())
            .map_err(|err| AuthError::InvalidRedirect(err.to_string()))?;
        let pkce = PkcePair::generate()?;
        let state = random_state()?;
        let nonce = Nonce::new_random();
        let state_value = state.expose().to_string();
        let nonce_value = nonce.clone();
        let mut request = client.authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            move || CsrfToken::new(state_value.clone()),
            move || nonce_value.clone(),
        );
        for scope in &self.scopes {
            request = request.add_scope(Scope::new(scope.clone()));
        }
        let mut request = request
            .set_pkce_challenge(PkceCodeChallenge::from_code_verifier_sha256(
                &PkceCodeVerifier::new(pkce.expose_verifier().to_string()),
            ))
            .set_redirect_uri(Cow::Owned(redirect_url));
        for (name, value) in &self.extra_auth_params {
            request = request.add_extra_param(name.clone(), value.clone());
        }
        let (authorize_url, _csrf, _nonce) = request.url();
        *lock(&self.pending) = Some(PendingOidcFlow {
            redirect,
            nonce: SecretString::new(nonce.secret().to_string()),
        });
        Ok(FlowLaunch {
            authorize_url: authorize_url.to_string(),
            state,
            pkce: Some(pkce),
            redirect_uri,
        })
    }

    async fn complete_callback(
        &self,
        http: reqwest::Client,
        redirect: RedirectParams,
        launch: &FlowLaunch,
    ) -> Result<TokenSet, AuthError> {
        let Some(code) = redirect.code else {
            return Err(AuthError::InvalidTokenResponse(
                "authorization response carried no code".into(),
            ));
        };
        let client = self.discover_client(&http).await?;
        let Some(pkce) = launch.pkce.as_ref() else {
            return Err(AuthError::InvalidTokenResponse(
                "flow launch carries no PKCE verifier".into(),
            ));
        };
        let redirect_url = RedirectUrl::new(launch.redirect_uri.clone())
            .map_err(|err| AuthError::InvalidRedirect(err.to_string()))?;
        let mut request = client
            .exchange_code(AuthorizationCode::new(code))
            .map_err(|err| AuthError::InvalidTokenResponse(err.to_string()))?
            .set_pkce_verifier(PkceCodeVerifier::new(pkce.expose_verifier().to_string()))
            .set_redirect_uri(Cow::Owned(redirect_url));
        if self.client_secret.is_none() {
            request = request.add_extra_param("client_id", self.client_id.clone());
        }
        let response = request
            .request_async(&http)
            .await
            .map_err(map_token_error)?;
        let account = self.fetch_account(&client, &http, &response).await;
        Ok(self.token_response_to_set(&response, account))
    }

    async fn refresh(
        &self,
        http: reqwest::Client,
        refresh_token: &str,
    ) -> Result<TokenSet, AuthError> {
        let client = self.discover_client(&http).await?;
        let token = RefreshToken::new(refresh_token.to_string());
        let mut request = client
            .exchange_refresh_token(&token)
            .map_err(|err| AuthError::InvalidTokenResponse(err.to_string()))?;
        if self.client_secret.is_none() {
            request = request.add_extra_param("client_id", self.client_id.clone());
        }
        let response = request
            .request_async(&http)
            .await
            .map_err(map_token_error)?;
        let account = self.fetch_account(&client, &http, &response).await;
        Ok(self.token_response_to_set(&response, account))
    }

    async fn revoke(&self, http: reqwest::Client, access_token: &str) -> Result<(), AuthError> {
        let Some(revoke_url) = self.revoke_url.clone() else {
            return Ok(());
        };
        let response = http
            .post(revoke_url)
            .form(&[("token", access_token)])
            .send()
            .await
            .map_err(|err| AuthError::Network(err.to_string()))?;
        if !response.status().is_success() {
            return Err(AuthError::Network(format!(
                "revocation endpoint returned {}",
                response.status()
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "oidc.test.rs"]
mod oidc_test;
