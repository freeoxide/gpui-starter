use gpui_auth::{AuthProvider, RedirectParams, RefreshPolicy};

use super::*;
use crate::services::auth_social::providers::fake_idp::FakeIdP;

fn scratch_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("scratch runtime")
}

fn fake_oidc_config(origin: &str) -> OidcConfig {
    OidcConfig {
        issuer: origin.to_string(),
        revoke_url: None,
        base: ProviderConfig {
            client_id: "oidc-client".to_string(),
            client_secret: None,
            redirect: None,
            scopes: None,
        },
    }
}

fn query_pair(url: &str, key: &str) -> Option<String> {
    url::Url::parse(url)
        .ok()?
        .query_pairs()
        .find(|(name, _)| name == key)
        .map(|(_, value)| value.to_string())
}

#[test]
fn generic_flow_against_the_fake_idp() {
    let rt = scratch_runtime();
    rt.block_on(async {
        let fake = FakeIdP::spawn(rt.handle());
        let provider =
            OidcProvider::generic(&fake_oidc_config(&fake.origin()), RedirectMode::Loopback);
        let http = reqwest::Client::new();
        let launch = provider.start_flow(http.clone()).await.expect("start flow");

        assert!(
            launch
                .authorize_url
                .starts_with(&format!("{}/authorize", fake.origin()))
        );
        assert_eq!(
            query_pair(&launch.authorize_url, "client_id").as_deref(),
            Some("oidc-client")
        );
        let challenge = query_pair(&launch.authorize_url, "code_challenge").expect("challenge");
        assert_eq!(challenge.len(), 43);
        assert_eq!(
            query_pair(&launch.authorize_url, "code_challenge_method").as_deref(),
            Some("S256")
        );
        assert_eq!(launch.state.expose().len(), 43);
        let port = provider.pending_port().expect("loopback listener bound");
        assert_eq!(
            query_pair(&launch.authorize_url, "redirect_uri").as_deref(),
            Some(format!("http://127.0.0.1:{port}/").as_str())
        );

        let redirect = RedirectParams::from_url(&format!(
            "http://127.0.0.1:{port}/?code=fake-code&state={}",
            launch.state.expose()
        ))
        .expect("redirect parses");
        let tokens = provider
            .complete_callback(http.clone(), redirect, &launch)
            .await
            .expect("complete callback");
        assert_eq!(tokens.provider_id, "oidc");
        assert_eq!(tokens.access_token.expose(), "fake-access-1");
        assert_eq!(
            tokens.refresh_token.as_ref().unwrap().expose(),
            "fake-refresh-1"
        );
        assert_eq!(tokens.id_token, None);
        let now = super::super::now_unix();
        let expires_at = tokens.expires_at.expect("expires_at from expires_in");
        assert!(
            (expires_at - now - 3600).abs() <= 2,
            "{expires_at} vs {now}"
        );
        let account = tokens.account.clone().expect("account from userinfo");
        assert_eq!(account.id.as_deref(), Some("fake-sub-1"));
        assert_eq!(account.email.as_deref(), Some("fake-user@example.test"));
        assert_eq!(account.display_name.as_deref(), Some("Fake User"));
        assert_eq!(
            account.avatar_url.as_deref(),
            Some("https://fake.example.test/avatar.png")
        );
        let round_tripped: gpui_auth::TokenSet =
            serde_json::from_str(&serde_json::to_string(&tokens).unwrap()).unwrap();
        assert_eq!(round_tripped, tokens);

        let token_requests = fake.requests_with_path("/token");
        assert_eq!(token_requests.len(), 1);
        assert!(token_requests[0].body.contains("client_id=oidc-client"));
        assert!(token_requests[0].body.contains("code=fake-code"));
        assert!(
            token_requests[0]
                .body
                .contains(&format!("redirect_uri=http%3A%2F%2F127.0.0.1%3A{port}%2F"))
        );
    });
}

#[test]
fn refresh_mints_a_new_access_token() {
    let rt = scratch_runtime();
    rt.block_on(async {
        let fake = FakeIdP::spawn(rt.handle());
        let provider =
            OidcProvider::generic(&fake_oidc_config(&fake.origin()), RedirectMode::Loopback);
        let http = reqwest::Client::new();
        let tokens = provider
            .refresh(http, "fake-refresh-1")
            .await
            .expect("refresh");
        assert_eq!(tokens.access_token.expose(), "fake-access-1");
        assert!(tokens.account.is_some(), "userinfo keeps the account label");
        assert_eq!(provider.refresh_policy(), RefreshPolicy::Rotated);
    });
}

#[test]
fn google_constructor_policy_and_revoke_endpoint() {
    let config = ProviderConfig {
        client_id: "google-client".to_string(),
        client_secret: None,
        redirect: None,
        scopes: None,
    };
    let provider = OidcProvider::google(&config, RedirectMode::Loopback);
    assert_eq!(provider.id(), "google");
    assert_eq!(provider.display_name(), "Google");
    assert_eq!(provider.refresh_policy(), RefreshPolicy::ReuseUntilDead);
}

#[test]
fn revoke_posts_the_token_form_body() {
    let rt = scratch_runtime();
    rt.block_on(async {
        let fake = FakeIdP::spawn(rt.handle());
        let mut config = fake_oidc_config(&fake.origin());
        config.revoke_url = Some(format!("{}/revoke", fake.origin()));
        let provider = OidcProvider::generic(&config, RedirectMode::Loopback);
        let http = reqwest::Client::new();
        provider
            .revoke(http, "token-to-revoke")
            .await
            .expect("revoke ok");
        let revocations = fake.requests_with_path("/revoke");
        assert_eq!(revocations.len(), 1);
        assert_eq!(revocations[0].method, "POST");
        assert!(revocations[0].body.contains("token=token-to-revoke"));
        assert_eq!(revocations[0].authorization, None);
    });
}

#[test]
fn revoke_without_an_endpoint_makes_no_http_call() {
    let rt = scratch_runtime();
    rt.block_on(async {
        let fake = FakeIdP::spawn(rt.handle());
        let provider =
            OidcProvider::generic(&fake_oidc_config(&fake.origin()), RedirectMode::Loopback);
        let http = reqwest::Client::new();
        provider.revoke(http, "token-to-revoke").await.expect("ok");
        assert!(fake.requests().is_empty());
    });
}

#[test]
fn scheme_mode_authorize_url_uses_the_deep_link() {
    let rt = scratch_runtime();
    rt.block_on(async {
        let fake = FakeIdP::spawn(rt.handle());
        let provider =
            OidcProvider::generic(&fake_oidc_config(&fake.origin()), RedirectMode::Scheme);
        let http = reqwest::Client::new();
        let launch = provider.start_flow(http).await.expect("start flow");
        assert_eq!(
            query_pair(&launch.authorize_url, "redirect_uri").as_deref(),
            Some("gpui-starter://auth/callback")
        );
        assert_eq!(provider.pending_port(), None);
    });
}

#[test]
fn missing_userinfo_endpoint_falls_back_to_no_account() {
    let rt = scratch_runtime();
    rt.block_on(async {
        let fake = FakeIdP::spawn(rt.handle());
        fake.disable_userinfo();
        let provider =
            OidcProvider::generic(&fake_oidc_config(&fake.origin()), RedirectMode::Loopback);
        let http = reqwest::Client::new();
        let launch = provider.start_flow(http.clone()).await.expect("start flow");
        let port = provider.pending_port().expect("loopback listener bound");
        let redirect = RedirectParams::from_url(&format!(
            "http://127.0.0.1:{port}/?code=fake-code&state={}",
            launch.state.expose()
        ))
        .expect("redirect parses");
        let tokens = provider
            .complete_callback(http, redirect, &launch)
            .await
            .expect("sign-in still succeeds");
        assert_eq!(tokens.access_token.expose(), "fake-access-1");
        assert!(tokens.account.is_none());
    });
}

#[test]
fn provider_error_response_maps_to_provider_denied() {
    let rt = scratch_runtime();
    rt.block_on(async {
        let fake = FakeIdP::spawn(rt.handle());
        fake.override_token_body(r#"{"error": "invalid_grant", "error_description": "bad code"}"#);
        let provider =
            OidcProvider::generic(&fake_oidc_config(&fake.origin()), RedirectMode::Loopback);
        let http = reqwest::Client::new();
        let launch = provider.start_flow(http.clone()).await.expect("start flow");
        let port = provider.pending_port().expect("loopback listener bound");
        let redirect = RedirectParams::from_url(&format!(
            "http://127.0.0.1:{port}/?code=bad&state={}",
            launch.state.expose()
        ))
        .expect("redirect parses");
        let error = provider
            .complete_callback(http, redirect, &launch)
            .await
            .expect_err("error body surfaces");
        let message = error.to_string();
        assert!(
            matches!(error, gpui_auth::AuthError::InvalidTokenResponse(_)),
            "got: {message}"
        );
        assert!(message.contains("invalid_grant"), "got: {message}");
    });
}
