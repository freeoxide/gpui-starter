use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use gpui_auth::{AuthError, AuthEvent, AuthIo, InMemoryStore, TokenStore};
use gpui_kit::{Subscription, TestAppContext};

use super::*;
use crate::capabilities::Capability;
use crate::services::session::SessionState;

fn mock_install(
    store: Arc<dyn TokenStore>,
    cx: &mut TestAppContext,
) -> gpui_kit::Entity<AuthController> {
    cx.update(|cx| {
        crate::services::session::initialize(cx);
        install_with(
            store,
            AuthIo {
                http: reqwest::Client::new(),
                runtime: None,
            },
            Arc::new(gpui_auth::MockProvider::new()),
            None,
            Arc::new(|| 0),
            cx,
        );
    });
    cx.update(|cx| controller(cx))
        .expect("auth controller installed")
}

fn test_install(cx: &mut TestAppContext) -> Arc<InMemoryStore> {
    let store = Arc::new(InMemoryStore::new());
    let shared: Arc<dyn TokenStore> = store.clone();
    mock_install(shared, cx);
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
async fn install_sets_the_email_capability_entry(cx: &mut TestAppContext) {
    test_install(cx);
    let snapshot = cx.read(|cx| crate::capabilities::snapshot(cx));
    let email = snapshot
        .get(&Capability::AUTH_EMAIL)
        .expect("auth_email entry");
    assert!(email.supported && email.enabled && !email.degraded);
    // auth_social::install owns the entry, so the adapter sets none.
    assert!(!snapshot.contains_key(&Capability::AUTH_SOCIAL));
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
async fn callback_link_before_install_falls_through_to_routing(cx: &mut TestAppContext) {
    cx.update(|cx| {
        assert!(!handle_callback_link(
            "gpui-starter://auth/callback?code=1&state=2",
            cx
        ));
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
    assert!(store.load("mock").await.unwrap().is_some());

    controller.update(cx, |controller, cx| controller.sign_out(cx));
    cx.run_until_parked();
    let session = cx.read(|cx| crate::session::snapshot(cx).state);
    assert_eq!(session, SessionState::SignedOut);
    assert_eq!(store.load("mock").await.unwrap(), None);
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
            avatar_url: None,
        }),
    };
    store.save("mock", &tokens).await.unwrap();
    cx.update(|cx| {
        crate::services::session::initialize(cx);
        install_with(
            store,
            AuthIo {
                http: reqwest::Client::new(),
                runtime: None,
            },
            Arc::new(gpui_auth::MockProvider::new()),
            None,
            Arc::new(|| 0),
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

#[cfg(not(target_family = "wasm"))]
mod email_suites {
    use std::sync::atomic::{AtomicI64, Ordering};

    use gpui_auth::{
        AuthBackend, AuthState, DemoBackend, EmailPasswordProvider, PasswordAuth, SecretString,
    };
    use gpui_kit::Entity;

    use super::*;

    const IAT: i64 = 1_000;
    const EMAIL: &str = "user@example.test";
    const PASSWORD: &str = "password-8";

    struct EmailApp {
        controller: Entity<AuthController>,
        store: Arc<InMemoryStore>,
        events: Rc<RefCell<Vec<AuthEvent>>>,
        _subscription: Subscription,
    }

    fn install_email(cx: &mut TestAppContext) -> EmailApp {
        let store = Arc::new(InMemoryStore::new());
        let shared: Arc<dyn TokenStore> = store.clone();
        let clock = Arc::new(AtomicI64::new(IAT));
        let clock_fn: Arc<dyn Fn() -> i64 + Send + Sync> =
            Arc::new(move || clock.load(Ordering::SeqCst));
        let backend = Arc::new(DemoBackend::with_params(
            argon2::Params::new(8, 1, 1, None).unwrap(),
        ));
        let provider = Arc::new(EmailPasswordProvider::new(
            backend as Arc<dyn AuthBackend>,
            clock_fn.clone(),
        ));
        cx.update(|cx| {
            crate::services::session::initialize(cx);
            install_with(
                shared,
                AuthIo {
                    http: reqwest::Client::new(),
                    runtime: None,
                },
                provider.clone() as Arc<dyn AuthProvider>,
                Some(provider as Arc<dyn PasswordAuth>),
                clock_fn,
                cx,
            );
        });
        let controller = cx
            .update(|cx| controller(cx))
            .expect("auth controller installed");
        let events = Rc::new(RefCell::new(Vec::new()));
        let sink = events.clone();
        let subscription = cx.update(|cx| {
            cx.subscribe(&controller, move |_, event: &AuthEvent, _| {
                sink.borrow_mut().push(event.clone())
            })
        });
        EmailApp {
            controller,
            store,
            events,
            _subscription: subscription,
        }
    }

    fn sign_up(app: &EmailApp, cx: &mut TestAppContext, email: &str) {
        app.controller.update(cx, |controller, cx| {
            controller.sign_up_with_password(email.to_string(), SecretString::new(PASSWORD), cx)
        });
        cx.run_until_parked();
    }

    fn sign_in(app: &EmailApp, cx: &mut TestAppContext, email: &str, password: &str) {
        app.controller.update(cx, |controller, cx| {
            controller.sign_in_with_password(email.to_string(), SecretString::new(password), cx)
        });
        cx.run_until_parked();
    }

    fn controller_state(cx: &TestAppContext, app: &EmailApp) -> AuthState {
        cx.read(|cx| app.controller.read(cx).state().clone())
    }

    fn session_state(cx: &TestAppContext) -> SessionState {
        cx.read(|cx| crate::session::snapshot(cx).state)
    }

    async fn stored_row(app: &EmailApp) -> Option<gpui_auth::TokenSet> {
        app.store.load("email").await.unwrap()
    }

    #[gpui_kit::test]
    async fn full_roundtrip_sign_up_sign_in_sign_out(cx: &mut TestAppContext) {
        let app = install_email(cx);
        sign_up(&app, cx, EMAIL);
        assert!(
            app.events.borrow().iter().any(
                |event| matches!(event, AuthEvent::AccountCreated { email } if email == EMAIL)
            )
        );
        assert_eq!(session_state(cx), SessionState::SignedOut);

        sign_in(&app, cx, EMAIL, PASSWORD);
        assert_eq!(
            session_state(cx),
            SessionState::SignedIn {
                account_label: EMAIL.to_string()
            }
        );
        let row = stored_row(&app).await.expect("store row after sign-in");
        assert_eq!(row.provider_id, "email");
        assert!(row.access_token.expose().starts_with("demo-access-"));
        assert!(
            row.refresh_token
                .unwrap()
                .expose()
                .starts_with("demo-refresh-")
        );

        app.controller
            .update(cx, |controller, cx| controller.sign_out(cx));
        cx.run_until_parked();
        assert_eq!(stored_row(&app).await, None);
        assert_eq!(session_state(cx), SessionState::SignedOut);
    }

    #[gpui_kit::test]
    async fn bad_credentials_error_without_a_store_row(cx: &mut TestAppContext) {
        let app = install_email(cx);
        sign_up(&app, cx, EMAIL);
        sign_in(&app, cx, EMAIL, "wrong-password");
        assert!(matches!(
            controller_state(cx, &app),
            AuthState::Error(AuthError::InvalidCredentials)
        ));
        assert!(matches!(session_state(cx), SessionState::Error(_)));
        assert_eq!(stored_row(&app).await, None);
    }
}

// KeyringStore is the only app-owned logic here; these hit the real OS
// keyring (session-keyed on Linux), so each test uses its own provider id.
#[cfg(not(target_family = "wasm"))]
mod keyring_store {
    use super::*;

    fn tokens(provider_id: &str) -> gpui_auth::TokenSet {
        gpui_auth::TokenSet {
            provider_id: provider_id.to_string(),
            access_token: gpui_auth::SecretString::new("roundtrip-access"),
            refresh_token: Some(gpui_auth::SecretString::new("roundtrip-refresh")),
            id_token: None,
            token_type: "Bearer".to_string(),
            expires_at: Some(1_700_000_000),
            scope: Some("profile".to_string()),
            account: Some(gpui_auth::AccountInfo {
                id: None,
                email: Some("user@example.test".to_string()),
                display_name: Some("Keyring Store Test".to_string()),
                avatar_url: None,
            }),
        }
    }

    #[gpui_kit::test]
    async fn save_then_load_roundtrips_the_token_row() {
        KeyringStore
            .save(
                "keyring-store-roundtrip",
                &tokens("keyring-store-roundtrip"),
            )
            .await
            .unwrap();
        assert_eq!(
            KeyringStore.load("keyring-store-roundtrip").await.unwrap(),
            Some(tokens("keyring-store-roundtrip"))
        );
        KeyringStore
            .delete("keyring-store-roundtrip")
            .await
            .unwrap();
    }

    #[gpui_kit::test]
    async fn a_corrupt_row_loads_as_a_miss_not_an_error() {
        crate::services::secure_storage::set_secret_blocking(
            KEYRING_SERVICE,
            "keyring-store-corrupt",
            "{not-json",
        )
        .unwrap();
        assert_eq!(
            KeyringStore.load("keyring-store-corrupt").await.unwrap(),
            None
        );
        KeyringStore.delete("keyring-store-corrupt").await.unwrap();
    }

    #[gpui_kit::test]
    async fn deleting_a_missing_key_is_ok() {
        KeyringStore.delete("keyring-store-missing").await.unwrap();
    }
}
