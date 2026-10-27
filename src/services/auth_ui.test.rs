use std::sync::Arc;

use gpui_auth::{AuthIo, AuthState, InMemoryStore};
use gpui_kit::TestAppContext;

use super::*;

fn mock_setup(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::services::session::initialize(cx);
        crate::services::auth::install_with(
            Arc::new(InMemoryStore::new()),
            AuthIo {
                http: reqwest::Client::new(),
                runtime: None,
            },
            Arc::new(gpui_auth::MockProvider::new()),
            None,
            Arc::new(|| 0),
            cx,
        );
        initialize(cx);
    });
    cx.run_until_parked();
}

#[gpui_kit::test]
async fn default_snapshot_is_signed_out(cx: &mut TestAppContext) {
    mock_setup(cx);
    let snap = cx.read(|cx| snapshot(cx));
    assert_eq!(snap.state, AuthState::SignedOut);
    assert_eq!(snap.provider_id, "mock");
    assert_eq!(snap.account_label, None);
    if cfg!(target_family = "wasm") {
        assert_eq!(snap.store_backend, STORE_BACKEND_MEMORY);
    } else {
        assert_eq!(snap.store_backend, STORE_BACKEND_KEYRING);
    }
}

#[gpui_kit::test]
async fn start_sign_in_captures_the_demo_authorize_url(cx: &mut TestAppContext) {
    mock_setup(cx);
    let controller = cx
        .read(|cx| crate::services::auth::controller(cx))
        .expect("controller installed");
    controller.update(cx, |controller, cx| controller.start_sign_in(cx));
    cx.run_until_parked();
    let snap = cx.read(|cx| snapshot(cx));
    assert_eq!(snap.state, AuthState::SigningIn);
    assert!(
        snap.demo_authorize_url
            .as_deref()
            .is_some_and(|url| url.starts_with("urn:gpui-auth:mock:"))
    );
}

#[gpui_kit::test]
async fn demo_completion_signs_in_and_clears_the_url(cx: &mut TestAppContext) {
    mock_setup(cx);
    let controller = cx
        .read(|cx| crate::services::auth::controller(cx))
        .expect("controller installed");
    controller.update(cx, |controller, cx| controller.start_sign_in(cx));
    cx.run_until_parked();
    cx.update(|cx| assert!(complete_demo_sign_in(cx)));
    cx.run_until_parked();
    let snap = cx.read(|cx| snapshot(cx));
    assert_eq!(
        snap.state,
        AuthState::SignedIn {
            account_label: "Mock account".to_string()
        }
    );
    assert_eq!(snap.demo_authorize_url, None);
}

#[gpui_kit::test]
async fn demo_completion_without_a_pending_flow_returns_false(cx: &mut TestAppContext) {
    mock_setup(cx);
    cx.update(|cx| assert!(!complete_demo_sign_in(cx)));
}

#[gpui_kit::test]
async fn sign_out_flips_the_snapshot_back(cx: &mut TestAppContext) {
    mock_setup(cx);
    let controller = cx
        .read(|cx| crate::services::auth::controller(cx))
        .expect("controller installed");
    controller.update(cx, |controller, cx| controller.start_sign_in(cx));
    cx.run_until_parked();
    cx.update(|cx| assert!(complete_demo_sign_in(cx)));
    cx.run_until_parked();
    controller.update(cx, |controller, cx| controller.sign_out(cx));
    cx.run_until_parked();
    let snap = cx.read(|cx| snapshot(cx));
    assert_eq!(snap.state, AuthState::SignedOut);
    assert_eq!(snap.account_label, None);
}

#[gpui_kit::test]
async fn format_expiry_formats_epoch_and_passes_none_through() {
    assert!(
        format_expiry(Some(0))
            .as_deref()
            .is_some_and(|v| v.contains("1970"))
    );
    assert_eq!(format_expiry(None), None);
}

#[gpui_kit::test]
async fn initialize_without_a_controller_keeps_the_default_snapshot(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::services::session::initialize(cx);
        initialize(cx);
    });
    let snap = cx.read(|cx| snapshot(cx));
    assert_eq!(snap.state, AuthState::SignedOut);
}
