// Feature-gated module: plain `cargo test -p gpui-auth` skips these; the
// local gate runs them via --all-features.
use super::*;

#[gpui::test]
async fn start_flow_returns_random_state_pkce_and_urn_url(_cx: &mut gpui::TestAppContext) {
    let provider = MockProvider::new();
    let launch = provider.start_flow(reqwest::Client::new()).await.unwrap();
    assert!(launch.authorize_url.starts_with("urn:gpui-auth:mock:"));
    assert_eq!(launch.state.expose().len(), 43);
    let challenge = launch.pkce.as_ref().expect("browser flow carries pkce");
    assert_eq!(challenge.challenge().len(), 43);
}

#[gpui::test]
async fn complete_callback_mints_round_trippable_tokens(_cx: &mut gpui::TestAppContext) {
    let provider = MockProvider::new();
    let launch = provider.start_flow(reqwest::Client::new()).await.unwrap();
    let redirect = RedirectParams {
        code: Some("any-code".to_string()),
        state: Some(launch.state.expose().to_string()),
        error: None,
        error_description: None,
    };
    let tokens = provider
        .complete_callback(reqwest::Client::new(), redirect, &launch)
        .await
        .unwrap();
    assert_eq!(tokens.provider_id, "mock");
    assert_eq!(tokens.access_token.expose(), "mock-access-0");
    assert_eq!(
        tokens.refresh_token.as_ref().unwrap().expose(),
        "mock-refresh-0"
    );
    let json = serde_json::to_string(&tokens).unwrap();
    let restored: TokenSet = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, tokens);
}

#[gpui::test]
async fn refresh_keeps_the_token_and_increments_access(_cx: &mut gpui::TestAppContext) {
    let provider = MockProvider::new();
    let refreshed = provider
        .refresh(reqwest::Client::new(), "mock-refresh-0")
        .await
        .unwrap();
    assert_eq!(refreshed.access_token.expose(), "mock-access-0");
    assert_eq!(
        refreshed.refresh_token.as_ref().unwrap().expose(),
        "mock-refresh-0"
    );
    let again = provider
        .refresh(reqwest::Client::new(), "mock-refresh-0")
        .await
        .unwrap();
    assert_eq!(again.access_token.expose(), "mock-access-1");
    assert_eq!(
        again.refresh_token.as_ref().unwrap().expose(),
        "mock-refresh-0"
    );
}

#[gpui::test]
async fn revoke_succeeds_and_policy_is_reuse_until_dead(_cx: &mut gpui::TestAppContext) {
    let provider = MockProvider::new();
    provider
        .revoke(reqwest::Client::new(), "mock-access-0")
        .await
        .unwrap();
    assert_eq!(provider.refresh_policy(), RefreshPolicy::ReuseUntilDead);
}
