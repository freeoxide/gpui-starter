#[cfg(test)]
pub(crate) mod fake_idp;
pub(crate) mod github;
pub(crate) mod oidc;

use gpui_auth::{AccountInfo, AuthError, SecretString, TokenSet};

use super::RedirectMode;
use super::loopback::{LoopbackHandle, LoopbackServer};

pub(crate) fn now_unix() -> i64 {
    chrono::Utc::now().timestamp()
}

pub(crate) fn token_set(
    provider_id: &str,
    access: String,
    refresh: Option<String>,
    id_token: Option<String>,
    expires_in: Option<std::time::Duration>,
    scope: Option<String>,
    account: Option<AccountInfo>,
) -> TokenSet {
    TokenSet {
        provider_id: provider_id.to_string(),
        access_token: SecretString::new(access),
        refresh_token: refresh.map(SecretString::new),
        id_token: id_token.map(SecretString::new),
        token_type: "Bearer".to_string(),
        expires_at: expires_in.map(|duration| now_unix() + duration.as_secs() as i64),
        scope,
        account,
    }
}

pub(crate) fn map_token_error(error: impl std::fmt::Display) -> AuthError {
    AuthError::InvalidTokenResponse(error.to_string())
}

pub(crate) struct PendingRedirect {
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) port: Option<u16>,
    wait: Option<tokio::sync::oneshot::Receiver<String>>,
    pub(crate) abort: Option<tokio::task::AbortHandle>,
}

impl PendingRedirect {
    pub(crate) fn scheme_marker() -> Self {
        Self {
            port: None,
            wait: None,
            abort: None,
        }
    }
}

pub(crate) fn start_redirect(mode: RedirectMode) -> Result<(PendingRedirect, String), AuthError> {
    match mode {
        RedirectMode::Scheme => Ok((
            PendingRedirect::scheme_marker(),
            "gpui-starter://auth/callback".to_string(),
        )),
        RedirectMode::Loopback => {
            let handle = LoopbackServer::spawn(&tokio::runtime::Handle::current())
                .map_err(|err| AuthError::Network(format!("loopback bind failed: {err}")))?;
            let redirect_uri = format!("http://127.0.0.1:{}/", handle.port);
            Ok((PendingRedirect::from(handle), redirect_uri))
        }
    }
}

impl From<LoopbackHandle> for PendingRedirect {
    fn from(handle: LoopbackHandle) -> Self {
        Self {
            port: Some(handle.port),
            wait: Some(handle.wait),
            abort: Some(handle.abort),
        }
    }
}

impl PendingRedirect {
    pub(crate) fn take_wait(
        &mut self,
    ) -> Option<(
        tokio::sync::oneshot::Receiver<String>,
        tokio::task::AbortHandle,
    )> {
        self.wait.take().zip(self.abort.take())
    }
}
