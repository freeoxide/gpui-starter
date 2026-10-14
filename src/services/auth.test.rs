use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use gpui_auth::{AuthEvent, AuthIo, InMemoryStore, TokenStore};
use gpui_kit::{Subscription, TestAppContext};

use super::*;
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
async fn install_sets_both_capability_entries(cx: &mut TestAppContext) {
    test_install(cx);
    let snapshot = cx.read(|cx| crate::capabilities::snapshot(cx));
    let email = snapshot.get("auth_email").expect("auth_email entry");
    #[cfg(not(target_family = "wasm"))]
    assert!(email.supported && email.enabled && !email.degraded);
    #[cfg(target_family = "wasm")]
    assert!(!email.supported && email.degraded);
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
        clock: Arc<AtomicI64>,
        events: Rc<RefCell<Vec<AuthEvent>>>,
        _subscription: Subscription,
    }

    fn install_email(cx: &mut TestAppContext) -> EmailApp {
        let store = Arc::new(InMemoryStore::new());
        let shared: Arc<dyn TokenStore> = store.clone();
        let clock = Arc::new(AtomicI64::new(IAT));
        let clock_reader = clock.clone();
        let clock_fn: Arc<dyn Fn() -> i64 + Send + Sync> =
            Arc::new(move || clock_reader.load(Ordering::SeqCst));
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
            clock,
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

    fn stored_row(cx: &mut TestAppContext, app: &EmailApp) -> Option<gpui_auth::TokenSet> {
        cx.update(|cx| app.store.load("email", cx).unwrap())
    }

    #[gpui_kit::test]
    async fn full_roundtrip_sign_up_sign_in_refresh_sign_out(cx: &mut TestAppContext) {
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
        let row = stored_row(cx, &app).expect("store row after sign-in");
        assert_eq!(row.provider_id, "email");
        assert_eq!(row.access_token.expose(), "demo-access-0");

        app.clock.store(IAT + 900 - 121, Ordering::SeqCst);
        app.controller
            .update(cx, |controller, cx| controller.refresh_if_needed(120, cx));
        cx.run_until_parked();
        let row = stored_row(cx, &app).unwrap();
        assert_eq!(row.access_token.expose(), "demo-access-0");

        app.clock.store(IAT + 900 - 120, Ordering::SeqCst);
        app.controller
            .update(cx, |controller, cx| controller.refresh_if_needed(120, cx));
        cx.run_until_parked();
        let row = stored_row(cx, &app).unwrap();
        assert_eq!(row.access_token.expose(), "demo-access-1");
        assert_eq!(row.refresh_token.unwrap().expose(), "demo-refresh-0");

        app.controller
            .update(cx, |controller, cx| controller.sign_out(cx));
        cx.run_until_parked();
        assert_eq!(stored_row(cx, &app), None);
        assert_eq!(session_state(cx), SessionState::SignedOut);
    }

    #[gpui_kit::test]
    async fn bad_credentials_error_without_a_store_row(cx: &mut TestAppContext) {
        let app = install_email(cx);
        sign_up(&app, cx, EMAIL);
        sign_in(&app, cx, EMAIL, "wrong-password");
        assert!(matches!(
            controller_state(cx, &app),
            AuthState::Error(message) if message.contains("invalid email or password")
        ));
        assert!(matches!(session_state(cx), SessionState::Error(_)));
        assert_eq!(stored_row(cx, &app), None);
    }

    #[gpui_kit::test]
    async fn duplicate_sign_up_surfaces_the_error(cx: &mut TestAppContext) {
        let app = install_email(cx);
        sign_up(&app, cx, EMAIL);
        sign_up(&app, cx, EMAIL);
        assert!(matches!(
            controller_state(cx, &app),
            AuthState::Error(message) if message.contains("already registered")
        ));
    }

    #[gpui_kit::test]
    async fn totp_enrollment_challenge_and_verify(cx: &mut TestAppContext) {
        let app = install_email(cx);
        sign_up(&app, cx, EMAIL);
        sign_in(&app, cx, EMAIL, PASSWORD);

        app.controller
            .update(cx, |controller, cx| controller.enroll_totp(cx));
        cx.run_until_parked();
        let enrollment = cx
            .read(|cx| app.controller.read(cx).totp_enrollment().cloned())
            .expect("enrollment round");
        assert_eq!(enrollment.email, EMAIL);
        assert_eq!(enrollment.secret_base32.len(), 32);
        assert!(
            enrollment
                .otpauth_url
                .starts_with("otpauth://totp/gpui-starter:")
        );
        let secret = decode_base32(&enrollment.secret_base32);
        let now = app.clock.load(Ordering::SeqCst);

        app.controller.update(cx, |controller, cx| {
            controller.confirm_totp_enrollment(wrong_code_for(&secret, now), cx)
        });
        cx.run_until_parked();
        let error = cx.read(|cx| {
            app.controller
                .read(cx)
                .totp_enrollment_error()
                .map(str::to_string)
        });
        assert_eq!(error.as_deref(), Some("TOTP code rejected"));

        app.controller.update(cx, |controller, cx| {
            controller.confirm_totp_enrollment(totp_code(&secret, now), cx)
        });
        cx.run_until_parked();
        assert!(cx.read(|cx| app.controller.read(cx).totp_enrollment().is_none()));

        app.controller
            .update(cx, |controller, cx| controller.sign_out(cx));
        cx.run_until_parked();
        sign_in(&app, cx, EMAIL, PASSWORD);
        assert_eq!(
            controller_state(cx, &app),
            AuthState::AwaitingChallenge {
                email: EMAIL.to_string(),
                error: None,
            }
        );

        app.controller.update(cx, |controller, cx| {
            controller.submit_totp(wrong_code_for(&secret, now), cx)
        });
        cx.run_until_parked();
        assert_eq!(
            controller_state(cx, &app),
            AuthState::AwaitingChallenge {
                email: EMAIL.to_string(),
                error: Some("TOTP code rejected".to_string()),
            }
        );

        app.controller.update(cx, |controller, cx| {
            controller.submit_totp(totp_code(&secret, now), cx)
        });
        cx.run_until_parked();
        assert_eq!(
            controller_state(cx, &app),
            AuthState::SignedIn {
                account_label: EMAIL.to_string()
            }
        );
        assert!(stored_row(cx, &app).is_some());
    }

    fn totp_code(secret: &[u8], unix: i64) -> String {
        use hmac::{Hmac, Mac};
        use sha1::Sha1;
        let counter = unix.div_euclid(30) as u64;
        let mut mac = Hmac::<Sha1>::new_from_slice(secret).unwrap();
        mac.update(&counter.to_be_bytes());
        let digest = mac.finalize().into_bytes();
        let offset = usize::from(digest[19] & 0x0f);
        let bin = ((u32::from(digest[offset]) & 0x7f) << 24)
            | (u32::from(digest[offset + 1]) << 16)
            | (u32::from(digest[offset + 2]) << 8)
            | u32::from(digest[offset + 3]);
        format!("{:06}", bin % 1_000_000)
    }

    fn wrong_code_for(secret: &[u8], unix: i64) -> String {
        let valid: Vec<String> = (-1..=1)
            .map(|delta| totp_code(secret, unix + delta * 30))
            .collect();
        (0..1_000_000u32)
            .map(|candidate| format!("{candidate:06}"))
            .find(|candidate| !valid.contains(candidate))
            .unwrap()
    }

    fn decode_base32(encoded: &str) -> Vec<u8> {
        const ALPHABET: &[u8; 32] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ234567";
        let mut out = Vec::new();
        let mut buffer = 0u32;
        let mut bits = 0u32;
        for char in encoded.chars() {
            let value = ALPHABET
                .iter()
                .position(|candidate| *candidate as char == char)
                .expect("base32 alphabet");
            buffer = (buffer << 5) | value as u32;
            bits += 5;
            if bits >= 8 {
                bits -= 8;
                out.push(((buffer >> bits) & 0xff) as u8);
            }
        }
        out
    }
}
