//! Removable social-login wiring. `install`, `start`, `sign_out`,
//! `controller`, `snapshot`, and `route_callback` are the only public
//! surface. Removal = delete this directory and revert the seam touches:
//! the `route_callback` guard and the auth_social capability move in
//! services/auth.rs (restore plan 21's original entry), the install line in
//! app/init.rs, the cfg'd module + re-export lines in services/mod.rs and
//! lib.rs, the oauth2/openidconnect lines in Cargo.toml's not-wasm table
//! with a regenerated Cargo.lock, and the AccountInfo.avatar_url field in
//! crates/gpui-auth (keyring rows keep parsing either way).
//!
//! Wasm posture: this module does not exist there. A web build would need an
//! https redirect to the app's own origin, `code`/`state` read from
//! `location.search` at boot (the wasm router reads `location.hash` only),
//! and a token-storage decision; none of that is implemented, and the
//! adapter's cfg'd capability entry carries the reason.

pub(crate) mod config;
pub(crate) mod flows;
pub(crate) mod loopback;
pub(crate) mod providers;

use gpui_auth::{AuthController, AuthIo};
use gpui_kit::{App, Context, Entity};

pub use config::{OidcConfig, ProviderConfig, SocialConfig};
pub use providers::github::GitHubProvider;
pub use providers::oidc::OidcProvider;

#[cfg(test)]
pub(crate) static ENV_OVERRIDE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// The three social providers; `Ord` order is config iteration order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProviderKind {
    Google,
    GitHub,
    Oidc,
}

impl ProviderKind {
    pub const ALL: [ProviderKind; 3] = [Self::Google, Self::GitHub, Self::Oidc];

    /// Stable provider identity; also the token-store key.
    pub fn id(&self) -> &'static str {
        match self {
            Self::Google => "google",
            Self::GitHub => "github",
            Self::Oidc => "oidc",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Google => "Google",
            Self::GitHub => "GitHub",
            Self::Oidc => "OIDC",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "google" => Some(Self::Google),
            "github" => Some(Self::GitHub),
            "oidc" => Some(Self::Oidc),
            _ => None,
        }
    }
}

/// How the authorization response reaches the app: an ephemeral
/// `127.0.0.1` listener (default, RFC 8252 first choice) or the
/// `gpui-starter://` deep link (requires a registered OS handler).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RedirectMode {
    #[default]
    Loopback,
    Scheme,
}

/// Per-provider state for diagnostics and plan 24's UI.
#[derive(Clone, Debug, PartialEq)]
pub struct SocialProviderStatus {
    pub configured: bool,
    pub redirect: RedirectMode,
    pub state: Option<gpui_auth::AuthState>,
}

impl Default for SocialProviderStatus {
    fn default() -> Self {
        Self {
            configured: false,
            redirect: RedirectMode::Loopback,
            state: None,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SocialAuthSnapshot {
    /// In `ProviderKind` order.
    pub providers: Vec<(ProviderKind, SocialProviderStatus)>,
    owner: Option<ProviderKind>,
}

impl SocialAuthSnapshot {
    pub fn any_configured(&self) -> bool {
        self.providers.iter().any(|(_, status)| status.configured)
    }

    /// The provider whose sign-in owns the session row, if any.
    pub fn session_owner(&self) -> Option<ProviderKind> {
        self.owner
    }
}

pub(crate) fn config_for_kind(
    config: &SocialConfig,
    kind: ProviderKind,
) -> Option<&ProviderConfig> {
    match kind {
        ProviderKind::Google => config.google.as_ref(),
        ProviderKind::GitHub => config.github.as_ref(),
        ProviderKind::Oidc => config.oidc.as_ref().map(|oidc| &oidc.base),
    }
}

/// Boot wiring: reads auth-providers.json, builds one `AuthController` per
/// configured provider, restores tokens, registers capability entries, and
/// subscribes browser-open + session sync. Idempotent; call after
/// `services::auth::install` (it needs that module's `KeyringStore`).
pub fn install(cx: &mut App) {
    let (config, warnings) = config::load(cx);
    for warning in &warnings {
        tracing::warn!(target: "gpui_starter::auth_social", "{warning}");
    }
    let io = match crate::services::tokio_runtime::runtime_and_client(cx) {
        Some((runtime, http)) => AuthIo {
            http,
            runtime: Some(runtime),
        },
        None => flows::fallback_io(),
    };
    flows::install_with(config, warnings, None, Some(io), cx);
}

/// Begin a browser flow for one provider; a logged no-op when unconfigured.
pub fn start(kind: ProviderKind, cx: &mut App) {
    let _ = with_flows(cx, |flows, cx| flows.start(kind, cx));
}

/// Best-effort revoke + token delete for one provider.
pub fn sign_out(kind: ProviderKind, cx: &mut App) {
    let _ = with_flows(cx, |flows, cx| flows.sign_out(kind, cx));
}

pub fn controller(kind: ProviderKind, cx: &App) -> Option<Entity<AuthController>> {
    cx.try_global::<flows::SocialFlowsHandle>()
        .and_then(|handle| handle.0.read(cx).controller(kind))
}

pub fn snapshot(cx: &App) -> SocialAuthSnapshot {
    cx.try_global::<flows::SocialFlowsHandle>()
        .map(|handle| handle.0.read(cx).snapshot(cx))
        .unwrap_or_default()
}

/// Route `gpui-starter://auth/callback` links. True when the link reached
/// the active-flow controller (the caller skips the mock fallback).
pub fn route_callback(link: &str, cx: &mut App) -> bool {
    let Ok(url) = url::Url::parse(link) else {
        return false;
    };
    if url.scheme() != crate::routes::APP_URL_SCHEME || url.host_str() != Some("auth") {
        return false;
    }
    let is_callback = url
        .path_segments()
        .map(|mut segments| segments.find(|segment| !segment.is_empty()) == Some("callback"))
        .unwrap_or(false);
    if !is_callback {
        return false;
    }
    with_flows(cx, |flows, cx| flows.route_callback(link, cx)).unwrap_or(false)
}

fn with_flows<R>(
    cx: &mut App,
    update: impl FnOnce(&mut flows::SocialFlows, &mut Context<flows::SocialFlows>) -> R,
) -> Option<R> {
    let entity = cx.try_global::<flows::SocialFlowsHandle>()?.0.clone();
    Some(entity.update(cx, |flows, cx| update(flows, cx)))
}
