use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use gpui_auth::{AuthEvent, AuthIo, InMemoryStore, TokenStore};
use gpui_kit::{AppContext as _, Subscription, TestAppContext};

use super::*;
use crate::services::session::SessionState;

fn test_install(cx: &mut TestAppContext) -> Arc<InMemoryStore> {
    let store = Arc::new(InMemoryStore::new());
    let shared: Arc<dyn TokenStore> = store.clone();
    cx.update(|cx| {
        crate::services::session::initialize(cx);
        install_with(
            shared,
            AuthIo {
                http: reqwest::Client::new(),
                runtime: None,
            },
            cx,
        );
    });
    store
}

fn authorization_urls(
    cx: &TestAppContext,
    controller: &gpui_kit::Entity<AuthController>,
) -> (Rc<RefCell<Vec<String>>>, Subscription) {
    let urls = Rc::new(RefCell::new(Vec::new()));
    let sink = urls.clone();
    let subscription = cx.update(|cx| {
        cx.subscribe(controller, move |_, event: &AuthEvent, _| {
            if let AuthEvent::AuthorizationRequested { url } = event {
                sink.borrow_mut().push(url.clone());
            }
        })
    });
    (urls, subscription)
}

#[gpui_kit::test]
async fn install_sets_both_capability_entries(cx: &mut TestAppContext) {
    test_install(cx);
    let snapshot = cx.read(|cx| crate::capabilities::snapshot(cx));
    let email = snapshot.get("auth_email").expect("auth_email entry");
    assert!(email.supported && email.enabled);
    let social = snapshot.get("auth_social").expect("auth_social entry");
    #[cfg(not(target_family = "wasm"))]
    assert!(social.supported && social.enabled && !social.degraded);
    #[cfg(target_family = "wasm")]
    assert!(!social.supported && social.degraded);
}

#[gpui_kit::test]
async fn callback_link_routing_truth_table(cx: &mut TestAppContext) {
    test_install(cx);
    cx.update(|cx| {
        assert!(handle_callback_link(
            "gpui-starter://auth/callback?code=1&state=2",
            cx
        ));
        assert!(!handle_callback_link("gpui-starter://settings", cx));
        assert!(!handle_callback_link("gpui-starter://auth/other", cx));
        assert!(!handle_callback_link("not-a-url://x", cx));
    });
}

#[gpui_kit::test]
async fn mock_sign_in_routes_through_to_the_session_state(cx: &mut TestAppContext) {
    let store = test_install(cx);
    let controller = cx
        .read(|cx| controller(cx))
        .expect("auth controller installed");
    let (urls, _subscription) = authorization_urls(cx, &controller);

    controller.update(cx, |controller, cx| controller.start_sign_in(cx));
    cx.run_until_parked();
    let state = urls.borrow()[0]
        .strip_prefix("urn:gpui-auth:mock:")
        .expect("mock authorize url shape")
        .to_string();
    let link = format!("gpui-starter://auth/callback?code=1&state={state}");
    cx.update(|cx| assert!(handle_callback_link(&link, cx)));
    cx.run_until_parked();

    let session = cx.read(|cx| crate::session::snapshot(cx).state);
    assert_eq!(
        session,
        SessionState::SignedIn {
            account_label: "Mock account".to_string()
        }
    );
    cx.update(|cx| assert!(store.load("mock", cx).unwrap().is_some()));

    controller.update(cx, |controller, cx| controller.sign_out(cx));
    cx.run_until_parked();
    let session = cx.read(|cx| crate::session::snapshot(cx).state);
    assert_eq!(session, SessionState::SignedOut);
    cx.update(|cx| assert_eq!(store.load("mock", cx).unwrap(), None));
}

#[gpui_kit::test]
async fn cold_callback_link_without_a_flow_fails_gracefully(cx: &mut TestAppContext) {
    test_install(cx);
    cx.update(|cx| {
        assert!(handle_callback_link(
            "gpui-starter://auth/callback?code=x&state=y",
            cx
        ));
    });
    cx.run_until_parked();
    let session = cx.read(|cx| crate::session::snapshot(cx).state);
    assert!(matches!(session, SessionState::Error(message) if message.contains("no pending")));
}

#[gpui_kit::test]
async fn restored_sign_in_propagates_to_the_session_state(cx: &mut TestAppContext) {
    let store = Arc::new(InMemoryStore::new());
    let tokens = gpui_auth::TokenSet {
        provider_id: "mock".to_string(),
        access_token: gpui_auth::SecretString::new("stored-access"),
        refresh_token: Some(gpui_auth::SecretString::new("stored-refresh")),
        id_token: None,
        token_type: "Bearer".to_string(),
        expires_at: None,
        scope: None,
        account: Some(gpui_auth::AccountInfo {
            id: None,
            email: None,
            display_name: Some("Mock account".to_string()),
        }),
    };
    cx.update(|cx| {
        crate::services::session::initialize(cx);
        store.save("mock", &tokens, cx).unwrap();
        install_with(
            store,
            AuthIo {
                http: reqwest::Client::new(),
                runtime: None,
            },
            cx,
        );
    });
    cx.run_until_parked();
    let session = cx.read(|cx| crate::session::snapshot(cx).state);
    assert_eq!(
        session,
        SessionState::SignedIn {
            account_label: "Mock account".to_string()
        }
    );
}
