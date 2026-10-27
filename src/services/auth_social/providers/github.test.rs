use gpui_auth::{AuthProvider, RedirectParams, RefreshPolicy};

use super::*;
use crate::services::auth_social::providers::fake_idp::FakeIdP;

fn scratch_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("scratch runtime")
}

fn github_config() -> ProviderConfig {
    ProviderConfig {
        client_id: "gh-client".to_string(),
        client_secret: Some("gh-secret".to_string()),
        redirect: None,
        scopes: None,
    }
}

fn fake_github(rt: &tokio::runtime::Runtime) -> (FakeIdP, GitHubProvider) {
    let fake = FakeIdP::spawn(rt.handle());
    let mut provider = GitHubProvider::new(&github_config(), RedirectMode::Loopback);
    let origin = fake.origin();
    provider.set_endpoints(
        &format!("{origin}/login/oauth/authorize"),
        &format!("{origin}/login/oauth/access_token"),
        &origin,
    );
    (fake, provider)
}

fn query_pair(url: &str, key: &str) -> Option<String> {
    url::Url::parse(url)
        .ok()?
        .query_pairs()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.to_string())
}

#[test]
fn start_flow_builds_the_github_authorize_url() {
    let rt = scratch_runtime();
    rt.block_on(async {
        let provider = GitHubProvider::new(&github_config(), RedirectMode::Loopback);
        let launch = provider
            .start_flow(reqwest::Client::new())
            .await
            .expect("start flow");
        assert!(
            launch
                .authorize_url
                .contains("github.com/login/oauth/authorize?")
        );
        assert_eq!(
            query_pair(&launch.authorize_url, "client_id").as_deref(),
            Some("gh-client")
        );
        assert_eq!(launch.state.expose().len(), 43);
        assert_eq!(
            query_pair(&launch.authorize_url, "code_challenge_method").as_deref(),
            Some("S256")
        );
        let port = provider.pending_port().expect("loopback bound");
        assert_eq!(
            query_pair(&launch.authorize_url, "redirect_uri").as_deref(),
            Some(format!("http://127.0.0.1:{port}/").as_str())
        );
        assert_eq!(provider.refresh_policy(), RefreshPolicy::None);
    });
}

#[test]
fn complete_callback_exchanges_and_builds_the_profile() {
    let rt = scratch_runtime();
    rt.block_on(async {
        let (fake, provider) = fake_github(&rt);
        let http = reqwest::Client::new();
        let launch = provider.start_flow(http.clone()).await.expect("start flow");
        let port = provider.pending_port().expect("loopback bound");
        let redirect = RedirectParams::from_url(&format!(
            "http://127.0.0.1:{port}/?code=gh-code&state={}",
            launch.state.expose()
        ))
        .expect("redirect parses");
        let tokens = provider
            .complete_callback(http, redirect, &launch)
            .await
            .expect("complete");
        assert_eq!(tokens.provider_id, "github");
        assert_eq!(tokens.access_token.expose(), "gh-access-1");
        assert_eq!(tokens.refresh_token, None);
        let account = tokens.account.expect("profile from /user + /user/emails");
        assert_eq!(account.id.as_deref(), Some("fake-login"));
        assert_eq!(account.email.as_deref(), Some("primary@example.test"));
        assert_eq!(account.display_name.as_deref(), Some("fake-login"));
        assert_eq!(
            account.avatar_url.as_deref(),
            Some("https://fake.example.test/gh.png")
        );

        let exchange = fake.requests_with_path("/login/oauth/access_token");
        assert_eq!(exchange.len(), 1);
        assert_eq!(exchange[0].accept.as_deref(), Some("application/json"));
        assert!(exchange[0].body.contains("code=gh-code"));
        assert!(exchange[0].body.contains("client_id=gh-client"));
        assert!(exchange[0].body.contains("client_secret=gh-secret"));
    });
}

#[test]
fn token_endpoint_requires_the_json_accept_header() {
    let rt = scratch_runtime();
    rt.block_on(async {
        let fake = FakeIdP::spawn(rt.handle());
        let response = reqwest::Client::new()
            .post(format!("{}/login/oauth/access_token", fake.origin()))
            .body("code=x")
            .send()
            .await
            .expect("post without accept");
        assert_eq!(response.status(), reqwest::StatusCode::NOT_ACCEPTABLE);
    });
}

#[test]
fn public_client_exchange_carries_client_id_without_secret() {
    let rt = scratch_runtime();
    rt.block_on(async {
        let fake = FakeIdP::spawn(rt.handle());
        let mut config = github_config();
        config.client_secret = None;
        let mut provider = GitHubProvider::new(&config, RedirectMode::Loopback);
        let origin = fake.origin();
        provider.set_endpoints(
            &format!("{origin}/login/oauth/authorize"),
            &format!("{origin}/login/oauth/access_token"),
            &origin,
        );
        let http = reqwest::Client::new();
        let launch = provider.start_flow(http.clone()).await.expect("start flow");
        let port = provider.pending_port().expect("loopback bound");
        let redirect = RedirectParams::from_url(&format!(
            "http://127.0.0.1:{port}/?code=gh-code&state={}",
            launch.state.expose()
        ))
        .expect("redirect parses");
        provider
            .complete_callback(http, redirect, &launch)
            .await
            .expect("public-client exchange succeeds");
        let exchange = fake.requests_with_path("/login/oauth/access_token");
        assert!(exchange[0].body.contains("client_id=gh-client"));
        assert!(!exchange[0].body.contains("client_secret"));
    });
}

#[test]
fn revoke_deletes_with_basic_auth() {
    let rt = scratch_runtime();
    rt.block_on(async {
        let (fake, provider) = fake_github(&rt);
        provider
            .revoke(reqwest::Client::new(), "gh-access-1")
            .await
            .expect("revoke ok");
        let requests = fake.requests();
        let revocations: Vec<_> = requests
            .iter()
            .filter(|request| request.path.starts_with("/applications/"))
            .collect();
        assert_eq!(revocations.len(), 1);
        assert_eq!(revocations[0].method, "DELETE");
        assert_eq!(revocations[0].path, "/applications/gh-client/token");
        let authorization = revocations[0].authorization.as_deref().expect("basic auth");
        assert!(authorization.starts_with("Basic "));
        assert!(revocations[0].body.contains("gh-access-1"));
    });
}

#[test]
fn error_token_body_maps_to_provider_denied() {
    let rt = scratch_runtime();
    rt.block_on(async {
        let (fake, provider) = fake_github(&rt);
        fake.override_token_body(
            r#"{"error": "incorrect_client_credentials", "error_description": "no"}"#,
        );
        let http = reqwest::Client::new();
        let launch = provider.start_flow(http.clone()).await.expect("start flow");
        let port = provider.pending_port().expect("loopback bound");
        let redirect = RedirectParams::from_url(&format!(
            "http://127.0.0.1:{port}/?code=gh-code&state={}",
            launch.state.expose()
        ))
        .expect("redirect parses");
        let error = provider
            .complete_callback(http, redirect, &launch)
            .await
            .expect_err("error body surfaces");
        assert!(matches!(
            error,
            gpui_auth::AuthError::ProviderDenied { ref error, .. }
                if error == "incorrect_client_credentials"
        ));
    });
}

#[test]
fn refresh_refuses_under_the_none_policy() {
    let rt = scratch_runtime();
    rt.block_on(async {
        let (_fake, provider) = fake_github(&rt);
        let error = provider
            .refresh(reqwest::Client::new(), "any")
            .await
            .expect_err("no refresh for github");
        assert!(matches!(error, gpui_auth::AuthError::NoRefreshToken));
    });
}
