//! User-authored provider credentials (`auth-providers.json` in the config
//! directory, or `$GPUI_AUTH_PROVIDERS`). The app reads this file and never
//! writes it. Template:
//!
//! ```json
//! {
//!   "google": { "client_id": "…", "redirect": "loopback" },
//!   "github": { "client_id": "…", "client_secret": "…", "scopes": ["read:user"] },
//!   "oidc": {
//!     "issuer": "https://issuer.example.com",
//!     "revoke_url": "https://issuer.example.com/revoke",
//!     "client_id": "…"
//!   }
//! }
//! ```
//!
//! Register redirect URIs with the provider verbatim, trailing slash
//! included: loopback mode sends `http://127.0.0.1:{port}/`. The default
//! redirect mode is `loopback` on every platform; `"redirect": "scheme"`
//! opts into `gpui-starter://auth/callback` and requires a registered OS
//! handler for the scheme (Windows boot registration, packaged Linux
//! installs, a bundled macOS app). `chmod 600` the file: it holds client
//! secrets.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use super::{ProviderKind, RedirectMode};
use gpui_kit::App;

/// Larger files are treated as unreadable rather than parsed; the same
/// bounding discipline as the app-state file, scaled to a credentials file.
const MAX_CONFIG_BYTES: u64 = 64 * 1024;

const CONFIG_FILE: &str = "auth-providers.json";

#[derive(Clone, Debug, Default, PartialEq, Deserialize)]
pub struct SocialConfig {
    #[serde(default)]
    pub google: Option<ProviderConfig>,
    #[serde(default)]
    pub github: Option<ProviderConfig>,
    #[serde(default)]
    pub oidc: Option<OidcConfig>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ProviderConfig {
    pub client_id: String,
    #[serde(default)]
    pub client_secret: Option<String>,
    #[serde(default)]
    pub redirect: Option<String>,
    #[serde(default)]
    pub scopes: Option<Vec<String>>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct OidcConfig {
    pub issuer: String,
    #[serde(default)]
    pub revoke_url: Option<String>,
    #[serde(flatten)]
    pub base: ProviderConfig,
}

pub(crate) fn load(cx: &App) -> (SocialConfig, Vec<String>) {
    load_from_path(&config_path(cx))
}

fn config_path(cx: &App) -> PathBuf {
    if let Some(path) = std::env::var_os("GPUI_AUTH_PROVIDERS") {
        return PathBuf::from(path);
    }
    crate::app_state::paths(cx).config_dir.join(CONFIG_FILE)
}

pub(crate) fn load_from_path(path: &Path) -> (SocialConfig, Vec<String>) {
    let mut warnings = Vec::new();
    let bytes = match read_bounded(path) {
        Ok(Some(bytes)) => bytes,
        Ok(None) => return (SocialConfig::default(), warnings),
        Err(warning) => {
            warnings.push(warning);
            return (SocialConfig::default(), warnings);
        }
    };
    if let Err(err) = permissions_lint(path) {
        warnings.push(err);
    }
    match serde_json::from_slice::<SocialConfig>(&bytes) {
        Ok(config) => (config, warnings),
        Err(err) => {
            warnings.push(format!(
                "{} is not valid JSON ({err}); social login stays unconfigured",
                path.display()
            ));
            (SocialConfig::default(), warnings)
        }
    }
}

fn read_bounded(path: &Path) -> Result<Option<Vec<u8>>, String> {
    let metadata = match std::fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => {
            return Err(format!(
                "cannot read {} ({err}); social login stays unconfigured",
                path.display()
            ));
        }
    };
    if !metadata.is_file() {
        return Ok(None);
    }
    if metadata.len() > MAX_CONFIG_BYTES {
        return Err(format!(
            "{} is larger than {} bytes; social login stays unconfigured",
            path.display(),
            MAX_CONFIG_BYTES
        ));
    }
    match std::fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(err) => Err(format!(
            "cannot read {} ({err}); social login stays unconfigured",
            path.display()
        )),
    }
}

#[cfg(unix)]
fn permissions_lint(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt as _;
    let mode = std::fs::metadata(path)
        .map(|metadata| metadata.permissions().mode())
        .unwrap_or(0o600);
    if mode & 0o777 != 0o600 {
        return Err(format!(
            "{} is group/world readable; chmod 600 it (it holds client secrets)",
            path.display()
        ));
    }
    Ok(())
}

#[cfg(not(unix))]
fn permissions_lint(_path: &Path) -> Result<(), String> {
    Ok(())
}

pub(crate) fn effective_redirect(kind: ProviderKind, config: &SocialConfig) -> RedirectMode {
    let Some(provider) = super::config_for_kind(config, kind) else {
        return RedirectMode::Loopback;
    };
    match provider.redirect.as_deref() {
        Some("scheme") => RedirectMode::Scheme,
        Some("loopback") | None => RedirectMode::Loopback,
        Some(other) => {
            tracing::warn!(
                target: "gpui_starter::auth_social",
                provider = kind.id(),
                value = other,
                "unknown redirect mode; falling back to loopback"
            );
            RedirectMode::Loopback
        }
    }
}

#[cfg(test)]
#[path = "config.test.rs"]
mod config_test;
