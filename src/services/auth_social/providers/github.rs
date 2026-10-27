use std::borrow::Cow;
use std::sync::Mutex;

use gpui_auth::{
    AccountInfo, AuthError, AuthProvider, FlowLaunch, PkcePair, RedirectParams, RefreshPolicy,
    SecretString, TokenSet, random_state,
};
use oauth2::basic::BasicClient;
use oauth2::{
    AuthUrl, ClientId, ClientSecret, CsrfToken, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl,
    Scope, TokenUrl,
};
use serde::Deserialize;

use super::{PendingRedirect, start_redirect, token_set};
use crate::services::auth_social::{ProviderConfig, ProviderKind, RedirectMode};

const DEFAULT_AUTHORIZE_URL: &str = "https://github.com/login/oauth/authorize";
const DEFAULT_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const DEFAULT_API_URL: &str = "https://api.github.com";

#[derive(Deserialize)]
struct GhUser {
    login: String,
    name: Option<String>,
    email: Option<String>,
    avatar_url: Option<String>,
}

#[derive(Deserialize)]
struct GhEmail {
    email: String,
    primary: bool,
}

#[derive(Deserialize)]
struct GhTokenResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    scope: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

struct GithubEndpoints {
    authorize: String,
    token: String,
    api: String,
}

/// GitHub OAuth app provider. Default tokens are non-expiring with no
/// refresh (`RefreshPolicy::None`), so `refresh` always refuses.
pub struct GitHubProvider {
    client_id: String,
    client_secret: Option<SecretString>,
    scopes: Vec<String>,
    redirect: RedirectMode,
    endpoints: GithubEndpoints,
    pending: Mutex<Option<PendingRedirect>>,
}

impl GitHubProvider {
    pub fn new(config: &ProviderConfig, redirect: RedirectMode) -> Self {
        Self {
            client_id: config.client_id.clone(),
            client_secret: config.client_secret.clone().map(SecretString::new),
            scopes: config
                .scopes
                .clone()
                .unwrap_or_else(|| vec!["read:user".into(), "user:email".into()]),
            redirect,
            endpoints: GithubEndpoints {
                authorize: DEFAULT_AUTHORIZE_URL.to_string(),
                token: DEFAULT_TOKEN_URL.to_string(),
                api: DEFAULT_API_URL.to_string(),
            },
            pending: Mutex::new(None),
        }
    }

    #[cfg(test)]
    pub(crate) fn set_endpoints(&mut self, authorize: &str, token: &str, api: &str) {
        self.endpoints = GithubEndpoints {
            authorize: authorize.to_string(),
            token: token.to_string(),
            api: api.to_string(),
        };
    }

    pub(crate) fn take_pending_redirect(
        &self,
    ) -> Option<(
        tokio::sync::oneshot::Receiver<String>,
        tokio::task::AbortHandle,
    )> {
        lock(&self.pending).as_mut()?.take_wait()
    }

    #[cfg(test)]
    pub(crate) fn pending_port(&self) -> Option<u16> {
        lock(&self.pending).as_ref()?.port
    }

    fn lock_guard(&self) -> std::sync::MutexGuard<'_, Option<PendingRedirect>> {
        lock(&self.pending)
    }

    fn token_request_form(&self, code: &str, redirect_uri: &str) -> String {
        let mut form = url::form_urlencoded::Serializer::new(String::new());
        form.append_pair("grant_type", "authorization_code");
        form.append_pair("code", code);
        form.append_pair("client_id", &self.client_id);
        if let Some(secret) = &self.client_secret {
            form.append_pair("client_secret", secret.expose());
        }
        form.append_pair("redirect_uri", redirect_uri);
        form.finish()
    }

    async fn exchange_json(
        &self,
        http: &reqwest::Client,
        url: &str,
        form: String,
        verifier: &str,
    ) -> Result<GhTokenResponse, AuthError> {
        let body = {
            let mut form = url::form_urlencoded::Serializer::new(form);
            form.append_pair("code_verifier", verifier);
            form.finish()
        };
        let response = http
            .post(url)
            .header(reqwest::header::ACCEPT, "application/json")
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .body(body)
            .send()
            .await
            .map_err(|err| AuthError::Network(err.to_string()))?;
        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|err| AuthError::Network(err.to_string()))?;
        let parsed: GhTokenResponse = serde_json::from_str(&body).map_err(|_| {
            AuthError::InvalidTokenResponse(format!("token endpoint returned {status}"))
        })?;
        if let Some(error) = parsed.error.clone() {
            return Err(AuthError::ProviderDenied {
                error,
                description: parsed.error_description.clone(),
            });
        }
        Ok(parsed)
    }

    async fn fetch_user(&self, http: &reqwest::Client, access_token: &str) -> Option<AccountInfo> {
        let user: GhUser = self
            .github_json(http, &format!("{}/user", self.endpoints.api), access_token)
            .await?;
        let email = match user.email.clone() {
            Some(email) => Some(email),
            None => {
                let emails: Vec<GhEmail> = self
                    .github_json(
                        http,
                        &format!("{}/user/emails", self.endpoints.api),
                        access_token,
                    )
                    .await?;
                emails
                    .iter()
                    .find(|entry| entry.primary)
                    .or_else(|| emails.first())
                    .map(|entry| entry.email.clone())
            }
        };
        Some(AccountInfo {
            id: Some(user.login.clone()),
            email,
            display_name: user.name.clone().or(Some(user.login.clone())),
            avatar_url: user.avatar_url,
        })
    }

    async fn github_json<T: serde::de::DeserializeOwned>(
        &self,
        http: &reqwest::Client,
        url: &str,
        access_token: &str,
    ) -> Option<T> {
        let response = http
            .get(url)
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {access_token}"),
            )
            .header(reqwest::header::ACCEPT, "application/vnd.github+json")
            .send()
            .await
            .ok()?;
        if !response.status().is_success() {
            return None;
        }
        response.json().await.ok()
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[async_trait::async_trait]
impl AuthProvider for GitHubProvider {
    fn id(&self) -> &str {
        ProviderKind::GitHub.id()
    }

    fn display_name(&self) -> &str {
        ProviderKind::GitHub.display_name()
    }

    fn refresh_policy(&self) -> RefreshPolicy {
        RefreshPolicy::None
    }

    async fn start_flow(&self, _http: reqwest::Client) -> Result<FlowLaunch, AuthError> {
        let mut client = BasicClient::new(ClientId::new(self.client_id.clone()))
            .set_auth_uri(
                AuthUrl::new(self.endpoints.authorize.clone())
                    .map_err(|err| AuthError::InvalidRedirect(err.to_string()))?,
            )
            .set_token_uri(
                TokenUrl::new(self.endpoints.token.clone())
                    .map_err(|err| AuthError::InvalidRedirect(err.to_string()))?,
            );
        if let Some(secret) = &self.client_secret {
            client = client.set_client_secret(ClientSecret::new(secret.expose().to_string()));
        }
        let (pending, redirect_uri) = start_redirect(self.redirect)?;
        let redirect_url = RedirectUrl::new(redirect_uri.clone())
            .map_err(|err| AuthError::InvalidRedirect(err.to_string()))?;
        let pkce = PkcePair::generate()?;
        let state = random_state()?;
        let mut request = client
            .authorize_url(|| CsrfToken::new(state.expose().to_string()))
            .set_pkce_challenge(PkceCodeChallenge::from_code_verifier_sha256(
                &PkceCodeVerifier::new(pkce.expose_verifier().to_string()),
            ))
            .set_redirect_uri(Cow::Owned(redirect_url));
        for scope in &self.scopes {
            request = request.add_scope(Scope::new(scope.clone()));
        }
        let (authorize_url, _csrf) = request.url();
        *self.lock_guard() = Some(pending);
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
        let Some(pkce) = launch.pkce.as_ref() else {
            return Err(AuthError::InvalidTokenResponse(
                "flow launch carries no PKCE verifier".into(),
            ));
        };
        let form = self.token_request_form(&code, &launch.redirect_uri);
        let response = self
            .exchange_json(&http, &self.endpoints.token, form, pkce.expose_verifier())
            .await?;
        let Some(access) = response.access_token else {
            return Err(AuthError::InvalidTokenResponse(
                "token response carried no access_token".into(),
            ));
        };
        let account = self.fetch_user(&http, &access).await;
        Ok(token_set(
            self.id(),
            access,
            response.refresh_token,
            None,
            response
                .expires_in
                .map(|secs| std::time::Duration::from_secs(secs.max(0) as u64)),
            response.scope,
            account,
        ))
    }

    async fn refresh(
        &self,
        _http: reqwest::Client,
        _refresh_token: &str,
    ) -> Result<TokenSet, AuthError> {
        Err(AuthError::NoRefreshToken)
    }

    async fn revoke(&self, http: reqwest::Client, access_token: &str) -> Result<(), AuthError> {
        let url = format!(
            "{}/applications/{}/token",
            self.endpoints.api, self.client_id
        );
        let secret = self
            .client_secret
            .as_ref()
            .map(|secret| secret.expose().to_string())
            .unwrap_or_default();
        let response = http
            .delete(&url)
            .basic_auth(&self.client_id, Some(&secret))
            .header(reqwest::header::ACCEPT, "application/vnd.github+json")
            .json(&serde_json::json!({ "access_token": access_token }))
            .send()
            .await
            .map_err(|err| AuthError::Network(err.to_string()))?;
        if !response.status().is_success() {
            return Err(AuthError::Network(format!(
                "token revocation returned {}",
                response.status()
            )));
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "github.test.rs"]
mod github_test;
