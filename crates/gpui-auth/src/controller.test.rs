// Feature-gated provider: plain `cargo test -p gpui-auth` skips these; the
// local gate runs them via --all-features.
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{AppContext as _, Entity, Subscription, TestAppContext};

use super::*;
use crate::mock::MockProvider;
use crate::store::InMemoryStore;

fn current_state(cx: &TestAppContext, controller: &Entity<AuthController>) -> AuthState {
    cx.read(|cx| controller.read(cx).state().clone())
}

fn test_io() -> AuthIo {
    AuthIo {
        http: reqwest::Client::new(),
        runtime: None,
    }
}

fn new_controller(cx: &mut TestAppContext) -> (Entity<AuthController>, Arc<InMemoryStore>) {
    let store = Arc::new(InMemoryStore::new());
    let controller = cx.new(|cx| {
        AuthController::new(
            Arc::new(MockProvider::new()),
            Arc::clone(&store) as Arc<dyn TokenStore>,
            test_io(),
            cx,
        )
    });
    (controller, store)
}

fn collect_events(
    cx: &TestAppContext,
    controller: &Entity<AuthController>,
) -> (Rc<RefCell<Vec<AuthEvent>>>, Subscription) {
    let events = Rc::new(RefCell::new(Vec::new()));
    let sink = events.clone();
    let subscription = cx.update(|cx| {
        cx.subscribe(controller, move |_, event: &AuthEvent, _| {
            sink.borrow_mut().push(event.clone())
        })
    });
    (events, subscription)
}

fn authorization_url(events: &Rc<RefCell<Vec<AuthEvent>>>) -> String {
    events
        .borrow()
        .iter()
        .find_map(|event| match event {
            AuthEvent::AuthorizationRequested { url } => Some(url.clone()),
            _ => None,
        })
        .expect("AuthorizationRequested event captured")
}

fn state_param(events: &Rc<RefCell<Vec<AuthEvent>>>) -> String {
    authorization_url(events)
        .strip_prefix("urn:gpui-auth:mock:")
        .expect("mock authorize url shape")
        .to_string()
}

#[gpui::test]
async fn start_sign_in_emits_authorization_requested(cx: &mut TestAppContext) {
    let (controller, _store) = new_controller(cx);
    let (events, _subscription) = collect_events(cx, &controller);
    controller.update(cx, |controller, cx| controller.start_sign_in(cx));
    cx.run_until_parked();
    assert_eq!(current_state(cx, &controller), AuthState::SigningIn);
    assert!(cx.read(|cx| controller.read(cx).pending_is_some()));
    assert!(authorization_url(&events).starts_with("urn:gpui-auth:mock:"));
    assert_eq!(state_param(&events).len(), 43);
}

#[gpui::test]
async fn complete_callback_happy_path_signs_in_and_persists(cx: &mut TestAppContext) {
    let (controller, store) = new_controller(cx);
    let (events, _subscription) = collect_events(cx, &controller);
    controller.update(cx, |controller, cx| controller.start_sign_in(cx));
    cx.run_until_parked();
    let redirect = format!("https://cb?code=x&state={}", state_param(&events));
    controller.update(cx, |controller, cx| {
        controller.complete_callback(&redirect, cx)
    });
    cx.run_until_parked();
    assert_eq!(
        current_state(cx, &controller),
        AuthState::SignedIn {
            account_label: "Mock account".to_string()
        }
    );
    assert!(
        events
            .borrow()
            .iter()
            .any(|event| matches!(event, AuthEvent::FlowCompleted))
    );
    cx.update(|cx| assert!(store.load("mock", cx).unwrap().is_some()));
}

#[gpui::test]
async fn wrong_state_redirect_fails_and_consumes_the_flow(cx: &mut TestAppContext) {
    let (controller, store) = new_controller(cx);
    let (events, _subscription) = collect_events(cx, &controller);
    controller.update(cx, |controller, cx| controller.start_sign_in(cx));
    cx.run_until_parked();
    let redirect = "https://cb?code=x&state=tampered".to_string();
    controller.update(cx, |controller, cx| {
        controller.complete_callback(&redirect, cx)
    });
    cx.run_until_parked();
    let state = current_state(cx, &controller);
    assert!(matches!(state, AuthState::Error(message) if message.contains("state mismatch")));
    assert!(events.borrow().iter().any(|event| matches!(
        event,
        AuthEvent::FlowFailed {
            error: AuthError::StateMismatch
        }
    )));
    cx.update(|cx| assert_eq!(store.load("mock", cx).unwrap(), None));

    controller.update(cx, |controller, cx| {
        controller.complete_callback("https://cb?code=x&state=whatever", cx)
    });
    cx.run_until_parked();
    let state = current_state(cx, &controller);
    assert!(matches!(state, AuthState::Error(message) if message.contains("no pending")));
}

#[gpui::test]
async fn error_param_redirect_maps_to_provider_denied(cx: &mut TestAppContext) {
    let (controller, _store) = new_controller(cx);
    let (events, _subscription) = collect_events(cx, &controller);
    controller.update(cx, |controller, cx| controller.start_sign_in(cx));
    cx.run_until_parked();
    let state = state_param(&events);
    let redirect = format!("https://cb?error=access_denied&error_description=nope&state={state}");
    controller.update(cx, |controller, cx| {
        controller.complete_callback(&redirect, cx)
    });
    cx.run_until_parked();
    let state = current_state(cx, &controller);
    assert!(matches!(state, AuthState::Error(message) if message.contains("access_denied")));
}

#[gpui::test]
async fn cancel_drops_the_flow_without_provider_contact(cx: &mut TestAppContext) {
    let (controller, _store) = new_controller(cx);
    let (events, _subscription) = collect_events(cx, &controller);
    controller.update(cx, |controller, cx| controller.start_sign_in(cx));
    cx.run_until_parked();
    controller.update(cx, |controller, cx| controller.cancel(cx));
    assert_eq!(current_state(cx, &controller), AuthState::SignedOut);
    assert!(!cx.read(|cx| controller.read(cx).pending_is_some()));
    let state = state_param(&events);
    controller.update(cx, |controller, cx| {
        controller.complete_callback(&format!("https://cb?code=x&state={state}"), cx)
    });
    cx.run_until_parked();
    let state = current_state(cx, &controller);
    assert!(matches!(state, AuthState::Error(message) if message.contains("no pending")));
}

#[gpui::test]
async fn restore_signs_in_from_a_preseeded_store(cx: &mut TestAppContext) {
    let (controller, store) = new_controller(cx);
    let tokens = TokenSet {
        provider_id: "mock".to_string(),
        access_token: crate::SecretString::new("stored-access"),
        refresh_token: Some(crate::SecretString::new("stored-refresh")),
        id_token: None,
        token_type: "Bearer".to_string(),
        expires_at: None,
        scope: None,
        account: Some(AccountInfo {
            id: None,
            email: Some("stored@example.test".to_string()),
            display_name: None,
        }),
    };
    cx.update(|cx| store.save("mock", &tokens, cx).unwrap());
    controller.update(cx, |controller, cx| controller.restore(cx));
    assert_eq!(
        current_state(cx, &controller),
        AuthState::SignedIn {
            account_label: "stored@example.test".to_string()
        }
    );
}

#[gpui::test]
async fn refresh_after_sign_in_merges_per_reuse_until_dead(cx: &mut TestAppContext) {
    let (controller, store) = new_controller(cx);
    let (events, _subscription) = collect_events(cx, &controller);
    controller.update(cx, |controller, cx| controller.start_sign_in(cx));
    cx.run_until_parked();
    let redirect = format!("https://cb?code=x&state={}", state_param(&events));
    controller.update(cx, |controller, cx| {
        controller.complete_callback(&redirect, cx)
    });
    cx.run_until_parked();
    controller.update(cx, |controller, cx| controller.refresh(cx));
    cx.run_until_parked();
    let stored = cx.update(|cx| store.load("mock", cx).unwrap().unwrap());
    assert_eq!(stored.access_token.expose(), "mock-access-1");
    assert_eq!(stored.refresh_token.unwrap().expose(), "mock-refresh-0");
    assert_eq!(
        current_state(cx, &controller),
        AuthState::SignedIn {
            account_label: "Mock account".to_string()
        }
    );
}

#[gpui::test]
async fn refresh_while_signed_out_reports_no_refresh_token(cx: &mut TestAppContext) {
    let (controller, _store) = new_controller(cx);
    controller.update(cx, |controller, cx| controller.refresh(cx));
    let state = current_state(cx, &controller);
    assert!(matches!(state, AuthState::Error(message) if message.contains("no refresh token")));
}

#[gpui::test]
async fn sign_out_revokes_deletes_and_emits(cx: &mut TestAppContext) {
    let (controller, store) = new_controller(cx);
    let (events, _subscription) = collect_events(cx, &controller);
    controller.update(cx, |controller, cx| controller.start_sign_in(cx));
    cx.run_until_parked();
    let redirect = format!("https://cb?code=x&state={}", state_param(&events));
    controller.update(cx, |controller, cx| {
        controller.complete_callback(&redirect, cx)
    });
    cx.run_until_parked();
    controller.update(cx, |controller, cx| controller.sign_out(cx));
    cx.run_until_parked();
    assert_eq!(current_state(cx, &controller), AuthState::SignedOut);
    cx.update(|cx| assert_eq!(store.load("mock", cx).unwrap(), None));
    assert!(
        events
            .borrow()
            .iter()
            .any(|event| matches!(event, AuthEvent::SignedOut))
    );
}

const EMAIL_IAT: i64 = 10_000;

use std::sync::atomic::AtomicI64;

use crate::email::email_test::{TEST_TTL_SECS, TestBackend, fixed_clock};
use crate::email::{AuthBackend, EmailPasswordProvider, PasswordAuth};

struct EmailFixture {
    controller: Entity<AuthController>,
    store: Arc<InMemoryStore>,
    backend: Arc<TestBackend>,
    clock: Arc<AtomicI64>,
    events: Rc<RefCell<Vec<AuthEvent>>>,
    _subscription: Subscription,
}

fn email_controller(cx: &mut TestAppContext, with_clock: bool) -> EmailFixture {
    let store = Arc::new(InMemoryStore::new());
    let backend = Arc::new(TestBackend::default());
    let (clock, clock_fn) = fixed_clock(EMAIL_IAT);
    let provider = Arc::new(EmailPasswordProvider::new(
        Arc::clone(&backend) as Arc<dyn AuthBackend>,
        clock_fn.clone(),
    ));
    let controller = cx.new(|cx| {
        let mut controller = AuthController::new(
            Arc::clone(&provider) as Arc<dyn AuthProvider>,
            Arc::clone(&store) as Arc<dyn TokenStore>,
            test_io(),
            cx,
        );
        controller.enable_password(Arc::clone(&provider) as Arc<dyn PasswordAuth>);
        if with_clock {
            controller.set_clock(clock_fn);
        }
        controller
    });
    let (events, subscription) = collect_events(cx, &controller);
    EmailFixture {
        controller,
        store,
        backend,
        clock,
        events,
        _subscription: subscription,
    }
}

fn drive(
    fixture: &EmailFixture,
    cx: &mut TestAppContext,
    f: impl FnOnce(&mut AuthController, &mut gpui::Context<AuthController>),
) {
    fixture.controller.update(cx, f);
    cx.run_until_parked();
}

fn has_event(events: &Rc<RefCell<Vec<AuthEvent>>>, predicate: impl Fn(&AuthEvent) -> bool) -> bool {
    events.borrow().iter().any(predicate)
}

#[gpui::test]
async fn password_readers_start_none_and_enable_password_survives_replacement(
    cx: &mut TestAppContext,
) {
    let fixture = email_controller(cx, true);
    cx.read(|cx| {
        let controller = fixture.controller.read(cx);
        assert_eq!(controller.email_status(), None);
        assert!(controller.totp_enrollment().is_none());
        assert_eq!(controller.totp_enrollment_error(), None);
    });
    drive(&fixture, cx, |controller, cx| {
        let provider = Arc::new(EmailPasswordProvider::new(
            Arc::clone(&fixture.backend) as Arc<dyn AuthBackend>,
            Arc::new(|| 0),
        ));
        controller.enable_password(Arc::clone(&provider) as Arc<dyn PasswordAuth>);
        controller.enable_password(provider as Arc<dyn PasswordAuth>);
        controller.sign_up_with_password(
            "user@example.test".to_string(),
            crate::token::SecretString::new("password-8"),
            cx,
        );
    });
    assert_eq!(current_state(cx, &fixture.controller), AuthState::SignedOut);
    assert!(has_event(&fixture.events, |event| matches!(
        event,
        AuthEvent::AccountCreated { email } if email == "user@example.test"
    )));
}

#[gpui::test]
async fn password_sign_up_then_sign_in_persists_and_completes(cx: &mut TestAppContext) {
    let fixture = email_controller(cx, true);
    drive(&fixture, cx, |controller, cx| {
        controller.sign_up_with_password(
            "user@example.test".to_string(),
            crate::token::SecretString::new("password-8"),
            cx,
        )
    });
    drive(&fixture, cx, |controller, cx| {
        controller.sign_in_with_password(
            "user@example.test".to_string(),
            crate::token::SecretString::new("password-8"),
            cx,
        )
    });
    assert_eq!(
        current_state(cx, &fixture.controller),
        AuthState::SignedIn {
            account_label: "user@example.test".to_string()
        }
    );
    assert!(has_event(&fixture.events, |event| matches!(
        event,
        AuthEvent::FlowCompleted
    )));
    let stored = cx.update(|cx| fixture.store.load("email", cx).unwrap().unwrap());
    assert_eq!(stored.access_token.expose(), "test-access-0");
    cx.read(|cx| {
        assert_eq!(
            fixture
                .controller
                .read(cx)
                .email_status()
                .map(|status| status.email.clone()),
            Some("user@example.test".to_string())
        );
    });
}

#[gpui::test]
async fn duplicate_sign_up_and_bad_credentials_error_out(cx: &mut TestAppContext) {
    let fixture = email_controller(cx, true);
    drive(&fixture, cx, |controller, cx| {
        controller.sign_up_with_password(
            "dup@example.test".to_string(),
            crate::token::SecretString::new("password-8"),
            cx,
        )
    });
    drive(&fixture, cx, |controller, cx| {
        controller.sign_up_with_password(
            "dup@example.test".to_string(),
            crate::token::SecretString::new("password-8"),
            cx,
        )
    });
    let state = current_state(cx, &fixture.controller);
    assert!(matches!(state, AuthState::Error(message) if message.contains("already registered")));
    assert!(has_event(&fixture.events, |event| matches!(
        event,
        AuthEvent::FlowFailed { .. }
    )));

    drive(&fixture, cx, |controller, cx| {
        controller.sign_in_with_password(
            "dup@example.test".to_string(),
            crate::token::SecretString::new("wrong-password"),
            cx,
        )
    });
    let state = current_state(cx, &fixture.controller);
    assert!(
        matches!(state, AuthState::Error(message) if message.contains("invalid email or password"))
    );
    cx.update(|cx| assert_eq!(fixture.store.load("email", cx).unwrap(), None));
}

#[gpui::test]
async fn sign_up_while_signed_in_is_a_noop(cx: &mut TestAppContext) {
    let fixture = email_controller(cx, true);
    drive(&fixture, cx, |controller, cx| {
        controller.sign_up_with_password(
            "user@example.test".to_string(),
            crate::token::SecretString::new("password-8"),
            cx,
        )
    });
    drive(&fixture, cx, |controller, cx| {
        controller.sign_in_with_password(
            "user@example.test".to_string(),
            crate::token::SecretString::new("password-8"),
            cx,
        )
    });
    drive(&fixture, cx, |controller, cx| {
        controller.sign_up_with_password(
            "other@example.test".to_string(),
            crate::token::SecretString::new("password-8"),
            cx,
        )
    });
    assert_eq!(
        current_state(cx, &fixture.controller),
        AuthState::SignedIn {
            account_label: "user@example.test".to_string()
        }
    );
    assert_eq!(
        fixture
            .events
            .borrow()
            .iter()
            .filter(|event| matches!(event, AuthEvent::AccountCreated { .. }))
            .count(),
        1
    );
}

#[gpui::test]
async fn a_second_sign_in_overwrites_the_single_store_row(cx: &mut TestAppContext) {
    let fixture = email_controller(cx, true);
    for email in ["a@example.test", "b@example.test"] {
        drive(&fixture, cx, |controller, cx| {
            controller.sign_up_with_password(
                email.to_string(),
                crate::token::SecretString::new("password-8"),
                cx,
            )
        });
    }
    for email in ["a@example.test", "b@example.test"] {
        drive(&fixture, cx, |controller, cx| {
            controller.sign_in_with_password(
                email.to_string(),
                crate::token::SecretString::new("password-8"),
                cx,
            )
        });
    }
    assert_eq!(
        current_state(cx, &fixture.controller),
        AuthState::SignedIn {
            account_label: "b@example.test".to_string()
        }
    );
    let stored = cx.update(|cx| fixture.store.load("email", cx).unwrap().unwrap());
    assert_eq!(
        stored.account.unwrap().email,
        Some("b@example.test".to_string())
    );
}

#[gpui::test]
async fn totp_challenge_keeps_on_wrong_code_and_completes_on_the_right_one(
    cx: &mut TestAppContext,
) {
    let fixture = email_controller(cx, true);
    fixture.backend.require_totp();
    drive(&fixture, cx, |controller, cx| {
        controller.sign_up_with_password(
            "t@example.test".to_string(),
            crate::token::SecretString::new("password-8"),
            cx,
        )
    });
    drive(&fixture, cx, |controller, cx| {
        controller.sign_in_with_password(
            "t@example.test".to_string(),
            crate::token::SecretString::new("password-8"),
            cx,
        )
    });
    assert_eq!(
        current_state(cx, &fixture.controller),
        AuthState::AwaitingChallenge {
            email: "t@example.test".to_string(),
            error: None,
        }
    );

    fixture.backend.fail_totp_once();
    drive(&fixture, cx, |controller, cx| {
        controller.submit_totp("123456".to_string(), cx)
    });
    assert_eq!(
        current_state(cx, &fixture.controller),
        AuthState::AwaitingChallenge {
            email: "t@example.test".to_string(),
            error: Some("TOTP code rejected".to_string()),
        }
    );

    drive(&fixture, cx, |controller, cx| {
        controller.submit_totp("654321".to_string(), cx)
    });
    assert_eq!(
        current_state(cx, &fixture.controller),
        AuthState::SignedIn {
            account_label: "t@example.test".to_string()
        }
    );
    cx.update(|cx| assert!(fixture.store.load("email", cx).unwrap().is_some()));
}

#[gpui::test]
async fn expired_challenge_and_cancel_return_to_signed_out(cx: &mut TestAppContext) {
    let fixture = email_controller(cx, true);
    fixture.backend.require_totp();
    drive(&fixture, cx, |controller, cx| {
        controller.sign_up_with_password(
            "t@example.test".to_string(),
            crate::token::SecretString::new("password-8"),
            cx,
        )
    });
    drive(&fixture, cx, |controller, cx| {
        controller.sign_in_with_password(
            "t@example.test".to_string(),
            crate::token::SecretString::new("password-8"),
            cx,
        )
    });

    drive(&fixture, cx, |controller, cx| {
        controller.cancel_challenge(cx)
    });
    assert_eq!(current_state(cx, &fixture.controller), AuthState::SignedOut);

    drive(&fixture, cx, |controller, cx| {
        controller.sign_in_with_password(
            "t@example.test".to_string(),
            crate::token::SecretString::new("password-8"),
            cx,
        )
    });
    fixture.backend.fail_challenge_once();
    drive(&fixture, cx, |controller, cx| {
        controller.submit_totp("123456".to_string(), cx)
    });
    assert_eq!(current_state(cx, &fixture.controller), AuthState::SignedOut);
}

#[gpui::test]
async fn submit_totp_without_a_pending_challenge_errors(cx: &mut TestAppContext) {
    let fixture = email_controller(cx, true);
    drive(&fixture, cx, |controller, cx| {
        controller.submit_totp("123456".to_string(), cx)
    });
    let state = current_state(cx, &fixture.controller);
    assert!(matches!(state, AuthState::Error(message) if message.contains("no challenge")));
}

#[gpui::test]
async fn sign_out_clears_the_email_state_slots(cx: &mut TestAppContext) {
    let fixture = email_controller(cx, true);
    fixture.backend.require_totp();
    drive(&fixture, cx, |controller, cx| {
        controller.sign_up_with_password(
            "t@example.test".to_string(),
            crate::token::SecretString::new("password-8"),
            cx,
        )
    });
    drive(&fixture, cx, |controller, cx| {
        controller.sign_in_with_password(
            "t@example.test".to_string(),
            crate::token::SecretString::new("password-8"),
            cx,
        )
    });
    drive(&fixture, cx, |controller, cx| controller.sign_out(cx));
    assert_eq!(current_state(cx, &fixture.controller), AuthState::SignedOut);
    cx.read(|cx| {
        let controller = fixture.controller.read(cx);
        assert_eq!(controller.email_status(), None);
        assert!(controller.totp_enrollment().is_none());
        assert_eq!(controller.totp_enrollment_error(), None);
    });
    cx.update(|cx| assert_eq!(fixture.store.load("email", cx).unwrap(), None));
}

#[gpui::test]
async fn refresh_if_needed_respects_the_margin_and_the_clock(cx: &mut TestAppContext) {
    let fixture = email_controller(cx, true);
    drive(&fixture, cx, |controller, cx| {
        controller.sign_up_with_password(
            "r@example.test".to_string(),
            crate::token::SecretString::new("password-8"),
            cx,
        )
    });
    drive(&fixture, cx, |controller, cx| {
        controller.sign_in_with_password(
            "r@example.test".to_string(),
            crate::token::SecretString::new("password-8"),
            cx,
        )
    });

    fixture.clock.store(
        EMAIL_IAT + TEST_TTL_SECS - 121,
        std::sync::atomic::Ordering::SeqCst,
    );
    drive(&fixture, cx, |controller, cx| {
        controller.refresh_if_needed(120, cx)
    });
    let stored = cx.update(|cx| fixture.store.load("email", cx).unwrap().unwrap());
    assert_eq!(stored.access_token.expose(), "test-access-0");
    assert!(fixture.backend.refresh_seen().is_empty());

    fixture.clock.store(
        EMAIL_IAT + TEST_TTL_SECS - 120,
        std::sync::atomic::Ordering::SeqCst,
    );
    drive(&fixture, cx, |controller, cx| {
        controller.refresh_if_needed(120, cx)
    });
    let stored = cx.update(|cx| fixture.store.load("email", cx).unwrap().unwrap());
    assert_eq!(stored.access_token.expose(), "test-access-1");
    assert_eq!(stored.refresh_token.unwrap().expose(), "test-refresh-0");
}

#[gpui::test]
async fn refresh_if_needed_without_a_clock_or_a_row_is_a_noop(cx: &mut TestAppContext) {
    let fixture = email_controller(cx, false);
    drive(&fixture, cx, |controller, cx| {
        controller.sign_up_with_password(
            "r@example.test".to_string(),
            crate::token::SecretString::new("password-8"),
            cx,
        )
    });
    drive(&fixture, cx, |controller, cx| {
        controller.refresh_if_needed(120, cx)
    });
    assert_eq!(current_state(cx, &fixture.controller), AuthState::SignedOut);
    assert!(fixture.backend.refresh_seen().is_empty());
}

#[gpui::test]
async fn refresh_if_needed_with_a_dead_refresh_token_errors(cx: &mut TestAppContext) {
    let fixture = email_controller(cx, true);
    drive(&fixture, cx, |controller, cx| {
        controller.sign_up_with_password(
            "r@example.test".to_string(),
            crate::token::SecretString::new("password-8"),
            cx,
        )
    });
    drive(&fixture, cx, |controller, cx| {
        controller.sign_in_with_password(
            "r@example.test".to_string(),
            crate::token::SecretString::new("password-8"),
            cx,
        )
    });
    fixture.backend.reject_refresh();
    fixture.clock.store(
        EMAIL_IAT + TEST_TTL_SECS,
        std::sync::atomic::Ordering::SeqCst,
    );
    drive(&fixture, cx, |controller, cx| {
        controller.refresh_if_needed(120, cx)
    });
    let state = current_state(cx, &fixture.controller);
    assert!(matches!(state, AuthState::Error(message) if message.contains("backend rejected")));
}

#[gpui::test]
async fn enrollment_requires_a_signed_in_account(cx: &mut TestAppContext) {
    let fixture = email_controller(cx, true);
    drive(&fixture, cx, |controller, cx| controller.enroll_totp(cx));
    cx.read(|cx| assert!(fixture.controller.read(cx).totp_enrollment().is_none()));
    assert_eq!(current_state(cx, &fixture.controller), AuthState::SignedOut);
}

#[gpui::test]
async fn enrollment_round_keeps_on_failure_and_clears_on_success(cx: &mut TestAppContext) {
    let fixture = email_controller(cx, true);
    drive(&fixture, cx, |controller, cx| {
        controller.sign_up_with_password(
            "e@example.test".to_string(),
            crate::token::SecretString::new("password-8"),
            cx,
        )
    });
    drive(&fixture, cx, |controller, cx| {
        controller.sign_in_with_password(
            "e@example.test".to_string(),
            crate::token::SecretString::new("password-8"),
            cx,
        )
    });
    drive(&fixture, cx, |controller, cx| controller.enroll_totp(cx));
    cx.read(|cx| {
        let enrollment = fixture.controller.read(cx).totp_enrollment().unwrap();
        assert_eq!(enrollment.email, "e@example.test");
        assert_eq!(enrollment.secret_base32.len(), 32);
        assert!(enrollment.otpauth_url.starts_with("otpauth://totp/"));
    });

    fixture.backend.fail_confirm_once();
    drive(&fixture, cx, |controller, cx| {
        controller.confirm_totp_enrollment("000000".to_string(), cx)
    });
    cx.read(|cx| {
        let controller = fixture.controller.read(cx);
        assert!(controller.totp_enrollment().is_some());
        assert_eq!(
            controller.totp_enrollment_error(),
            Some("TOTP code rejected")
        );
    });

    drive(&fixture, cx, |controller, cx| {
        controller.confirm_totp_enrollment("123456".to_string(), cx)
    });
    cx.read(|cx| {
        let controller = fixture.controller.read(cx);
        assert!(controller.totp_enrollment().is_none());
        assert_eq!(controller.totp_enrollment_error(), None);
    });
}

#[gpui::test]
async fn restored_session_enrolls_through_the_stored_account_email(cx: &mut TestAppContext) {
    let fixture = email_controller(cx, true);
    drive(&fixture, cx, |controller, cx| {
        controller.sign_up_with_password(
            "restored@example.test".to_string(),
            crate::token::SecretString::new("password-8"),
            cx,
        )
    });
    let tokens = TokenSet {
        provider_id: "email".to_string(),
        access_token: crate::SecretString::new("stored-access"),
        refresh_token: Some(crate::SecretString::new("stored-refresh")),
        id_token: None,
        token_type: "Bearer".to_string(),
        expires_at: None,
        scope: None,
        account: Some(AccountInfo {
            id: None,
            email: Some("restored@example.test".to_string()),
            display_name: None,
        }),
    };
    cx.update(|cx| fixture.store.save("email", &tokens, cx).unwrap());
    drive(&fixture, cx, |controller, cx| controller.restore(cx));
    drive(&fixture, cx, |controller, cx| controller.enroll_totp(cx));
    cx.read(|cx| {
        assert_eq!(
            fixture
                .controller
                .read(cx)
                .totp_enrollment()
                .map(|enrollment| enrollment.email.clone()),
            Some("restored@example.test".to_string())
        );
    });
}
