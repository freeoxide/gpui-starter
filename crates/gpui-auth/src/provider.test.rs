use super::*;

#[test]
fn from_url_reads_code_and_state_from_an_https_callback() {
    let params = RedirectParams::from_url("https://c.b/x?code=a&state=b").unwrap();
    assert_eq!(params.code.as_deref(), Some("a"));
    assert_eq!(params.state.as_deref(), Some("b"));
    assert_eq!(params.error, None);
    assert_eq!(params.error_description, None);
}

#[test]
fn from_url_reads_the_spec_deep_link_callback() {
    let params = RedirectParams::from_url("gpui-starter://auth/callback?code=k&state=s").unwrap();
    assert_eq!(params.code.as_deref(), Some("k"));
    assert_eq!(params.state.as_deref(), Some("s"));
}

#[test]
fn from_url_reads_error_responses_and_ignores_unknown_keys() {
    let params =
        RedirectParams::from_url("https://c.b/x?error=access_denied&error_description=nope&foo=1")
            .unwrap();
    assert_eq!(params.error.as_deref(), Some("access_denied"));
    assert_eq!(params.error_description.as_deref(), Some("nope"));
    assert_eq!(params.code, None);
}

#[test]
fn from_url_rejects_non_urls() {
    match RedirectParams::from_url("not a url") {
        Err(AuthError::InvalidRedirect(_)) => {}
        Ok(_) => panic!("expected InvalidRedirect, got Ok"),
        Err(other) => panic!("expected InvalidRedirect, got {other}"),
    }
}

#[cfg(not(target_family = "wasm"))]
#[gpui::test]
async fn drive_with_no_runtime_awaits_the_future_directly(_cx: &mut gpui::TestAppContext) {
    let io = AuthIo {
        http: reqwest::Client::new(),
        runtime: None,
    };
    assert_eq!(io.drive(std::future::ready(7)).await, 7);
}
