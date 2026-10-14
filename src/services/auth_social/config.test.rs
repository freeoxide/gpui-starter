use std::io::Write as _;
use std::sync::atomic::{AtomicU32, Ordering};

use tempfile::TempDir;

use super::*;

static TEMP_COUNTER: AtomicU32 = AtomicU32::new(0);

fn unique_temp_path() -> (TempDir, std::path::PathBuf) {
    let dir = TempDir::new().expect("tempdir");
    let name = format!(
        "auth-providers-{}.json",
        TEMP_COUNTER.fetch_add(1, Ordering::SeqCst)
    );
    let path = dir.path().join(name);
    (dir, path)
}

fn write_config(path: &std::path::Path, contents: &str) {
    let mut file = std::fs::File::create(path).expect("create temp config");
    file.write_all(contents.as_bytes())
        .expect("write temp config");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let mut permissions = file.metadata().expect("metadata").permissions();
        permissions.set_mode(0o600);
        std::fs::set_permissions(path, permissions).expect("chmod 600");
    }
}

const FULL_CONFIG: &str = r#"{
  "google": {
    "client_id": "google-id",
    "client_secret": "google-secret",
    "redirect": "scheme",
    "scopes": ["openid", "email"]
  },
  "github": { "client_id": "github-id" },
  "oidc": {
    "issuer": "https://issuer.example.com",
    "revoke_url": "https://issuer.example.com/revoke",
    "client_id": "oidc-id",
    "scopes": ["openid", "profile"]
  }
}"#;

#[test]
fn parses_a_full_three_provider_file() {
    let (dir, path) = unique_temp_path();
    write_config(&path, FULL_CONFIG);
    let (config, warnings) = load_from_path(&path);
    assert_eq!(warnings, Vec::<String>::new());
    let google = config.google.expect("google section");
    assert_eq!(google.client_id, "google-id");
    assert_eq!(google.client_secret.as_deref(), Some("google-secret"));
    assert_eq!(google.redirect.as_deref(), Some("scheme"));
    assert_eq!(
        google.scopes,
        Some(vec!["openid".to_string(), "email".to_string()])
    );
    let github = config.github.expect("github section");
    assert_eq!(github.client_id, "github-id");
    assert_eq!(github.client_secret, None);
    assert_eq!(github.redirect, None);
    assert_eq!(github.scopes, None);
    let oidc = config.oidc.expect("oidc section");
    assert_eq!(oidc.issuer, "https://issuer.example.com");
    assert_eq!(
        oidc.revoke_url.as_deref(),
        Some("https://issuer.example.com/revoke")
    );
    assert_eq!(oidc.base.client_id, "oidc-id");
    drop(dir);
}

#[test]
fn partial_files_default_the_missing_keys() {
    let (dir, path) = unique_temp_path();
    write_config(&path, r#"{ "github": { "client_id": "github-id" } }"#);
    let (config, warnings) = load_from_path(&path);
    assert_eq!(warnings, Vec::<String>::new());
    assert_eq!(config.google, None);
    assert_eq!(config.oidc, None);
    assert!(config.github.is_some());
    drop(dir);
}

#[test]
fn missing_file_is_an_unconfigured_no_op() {
    let (dir, _path) = unique_temp_path();
    let (config, warnings) = load_from_path(dir.path().join("absent.json").as_path());
    assert_eq!(config, SocialConfig::default());
    assert_eq!(warnings, Vec::<String>::new());
    drop(dir);
}

#[test]
fn empty_file_parses_to_default_with_a_warning() {
    let (dir, path) = unique_temp_path();
    write_config(&path, "");
    let (config, warnings) = load_from_path(&path);
    assert_eq!(config, SocialConfig::default());
    assert!(!warnings.is_empty());
    drop(dir);
}

#[test]
fn malformed_json_yields_default_and_a_warning() {
    let (dir, path) = unique_temp_path();
    write_config(&path, "{ not json");
    let (config, warnings) = load_from_path(&path);
    assert_eq!(config, SocialConfig::default());
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("not valid JSON"));
    drop(dir);
}

#[cfg(unix)]
#[test]
fn group_readable_file_triggers_the_perms_lint() {
    use std::os::unix::fs::PermissionsExt as _;
    let (dir, path) = unique_temp_path();
    write_config(&path, r#"{ "github": { "client_id": "github-id" } }"#);
    let mut permissions = std::fs::metadata(&path).expect("metadata").permissions();
    permissions.set_mode(0o644);
    std::fs::set_permissions(&path, permissions).expect("chmod 644");
    let (config, warnings) = load_from_path(&path);
    assert!(config.github.is_some());
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("chmod 600"));
    drop(dir);
}

#[cfg(unix)]
#[test]
fn owner_only_file_passes_the_perms_lint() {
    use std::os::unix::fs::PermissionsExt as _;
    let (dir, path) = unique_temp_path();
    write_config(&path, r#"{ "github": { "client_id": "github-id" } }"#);
    let mut permissions = std::fs::metadata(&path).expect("metadata").permissions();
    permissions.set_mode(0o600);
    std::fs::set_permissions(&path, permissions).expect("chmod 600");
    let (_, warnings) = load_from_path(&path);
    assert_eq!(warnings, Vec::<String>::new());
    drop(dir);
}

#[test]
fn oversized_file_is_rejected_without_parsing() {
    let (dir, path) = unique_temp_path();
    let mut file = std::fs::File::create(&path).expect("create temp config");
    let block = "[".repeat(65 * 1024);
    file.write_all(block.as_bytes()).expect("write oversized");
    drop(file);
    let (config, warnings) = load_from_path(&path);
    assert_eq!(config, SocialConfig::default());
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("larger than"));
    drop(dir);
}

#[gpui_kit::test]
async fn env_var_override_points_load_at_the_given_file(cx: &mut gpui_kit::TestAppContext) {
    let _guard = super::super::ENV_OVERRIDE_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let (dir, path) = unique_temp_path();
    write_config(&path, r#"{ "github": { "client_id": "env-id" } }"#);
    let previous = std::env::var_os("GPUI_AUTH_PROVIDERS");
    // Serialized by ENV_OVERRIDE_LOCK with the flows suite's install test.
    unsafe { std::env::set_var("GPUI_AUTH_PROVIDERS", &path) };
    let config = cx.read(|cx| load(cx));
    unsafe {
        match previous {
            Some(value) => std::env::set_var("GPUI_AUTH_PROVIDERS", value),
            None => std::env::remove_var("GPUI_AUTH_PROVIDERS"),
        }
    }
    assert_eq!(config.0.github.expect("github section").client_id, "env-id");
    drop(dir);
}

#[test]
fn effective_redirect_truth_table() {
    let mut config = SocialConfig::default();
    assert_eq!(
        effective_redirect(ProviderKind::Google, &config),
        RedirectMode::Loopback
    );

    config.google = Some(ProviderConfig {
        client_id: "id".to_string(),
        client_secret: None,
        redirect: Some("scheme".to_string()),
        scopes: None,
    });
    config.github = Some(ProviderConfig {
        client_id: "id".to_string(),
        client_secret: None,
        redirect: Some("loopback".to_string()),
        scopes: None,
    });
    config.oidc = Some(OidcConfig {
        issuer: "https://issuer.example.com".to_string(),
        revoke_url: None,
        base: ProviderConfig {
            client_id: "id".to_string(),
            client_secret: None,
            redirect: Some("carrier-pigeon".to_string()),
            scopes: None,
        },
    });
    assert_eq!(
        effective_redirect(ProviderKind::Google, &config),
        RedirectMode::Scheme
    );
    assert_eq!(
        effective_redirect(ProviderKind::GitHub, &config),
        RedirectMode::Loopback
    );
    assert_eq!(
        effective_redirect(ProviderKind::Oidc, &config),
        RedirectMode::Loopback
    );
}

#[test]
fn provider_kind_parse_round_trips_every_id() {
    for kind in ProviderKind::ALL {
        assert_eq!(ProviderKind::parse(kind.id()), Some(kind));
    }
    assert_eq!(ProviderKind::parse("Google"), None);
    assert_eq!(ProviderKind::parse(""), None);
}
