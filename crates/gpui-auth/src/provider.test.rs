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
async fn drive_runs_on_a_real_runtime_and_propagates_task_panics(_cx: &mut gpui::TestAppContext) {
    let runtime = std::sync::Arc::new(
        tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("test runtime"),
    );
    let io = AuthIo {
        http: reqwest::Client::new(),
        runtime: Some(std::sync::Arc::clone(&runtime)),
    };
    // drive() parks on the spawned JoinHandle, so its own cold runtime must
    // drive it; the deterministic test executor forbids that park.
    assert_eq!(runtime.block_on(io.drive(std::future::ready(7))), 7);

    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        runtime.block_on(io.drive(async { panic!("auth worker boom") }))
    }));
    std::panic::set_hook(previous_hook);
    assert!(panicked.is_err(), "a panicking auth task must surface");
}
