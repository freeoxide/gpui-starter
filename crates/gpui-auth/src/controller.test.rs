// Feature-gated provider: plain `cargo test -p gpui-auth` skips these; the
// local gate runs them via --all-features.
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::atomic::{AtomicI64, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use gpui::{AppContext as _, Entity, Subscription, TestAppContext};

use super::*;
use crate::email::email_test::{TEST_TTL_SECS, TestBackend, fixed_clock};
use crate::email::{AuthBackend, EmailPasswordProvider, PasswordAuth};
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
    assert!(store.load("mock").await.unwrap().is_some());
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
    assert!(matches!(state, AuthState::Error(AuthError::StateMismatch)));
    assert!(events.borrow().iter().any(|event| matches!(
        event,
        AuthEvent::FlowFailed {
            error: AuthError::StateMismatch
        }
    )));
    assert_eq!(store.load("mock").await.unwrap(), None);

    controller.update(cx, |controller, cx| {
        controller.complete_callback("https://cb?code=x&state=whatever", cx)
    });
    cx.run_until_parked();
    let state = current_state(cx, &controller);
    assert!(matches!(state, AuthState::Error(AuthError::NoPendingFlow)));
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
    assert!(matches!(
        state,
        AuthState::Error(AuthError::ProviderDenied { error, .. }) if error == "access_denied"
    ));
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
    assert!(matches!(state, AuthState::Error(AuthError::NoPendingFlow)));
}

#[gpui::test]
async fn restore_signs_in_from_a_preseeded_store(cx: &mut TestAppContext) {
    let (controller, store) = new_controller(cx);
    let tokens = TokenSet {
        provider_id: "mock".to_string(),
        access_token: SecretString::new("stored-access"),
        refresh_token: Some(SecretString::new("stored-refresh")),
        id_token: None,
        token_type: "Bearer".to_string(),
        expires_at: None,
        scope: None,
        account: Some(AccountInfo {
            id: None,
            email: Some("stored@example.test".to_string()),
            display_name: None,
            avatar_url: None,
        }),
    };
    store.save("mock", &tokens).await.unwrap();
    controller.update(cx, |controller, cx| controller.restore(cx));
    cx.run_until_parked();
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
    let stored = store.load("mock").await.unwrap().unwrap();
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
    cx.run_until_parked();
    let state = current_state(cx, &controller);
    assert!(matches!(state, AuthState::Error(AuthError::NoRefreshToken)));
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
    assert_eq!(store.load("mock").await.unwrap(), None);
    assert!(
        events
            .borrow()
            .iter()
            .any(|event| matches!(event, AuthEvent::SignedOut))
    );
}

const EMAIL_IAT: i64 = 10_000;

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
            SecretString::new("password-8"),
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
            SecretString::new("password-8"),
            cx,
        )
    });
    drive(&fixture, cx, |controller, cx| {
        controller.sign_in_with_password(
            "user@example.test".to_string(),
            SecretString::new("password-8"),
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
    let stored = fixture.store.load("email").await.unwrap().unwrap();
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
            SecretString::new("password-8"),
            cx,
        )
    });
    drive(&fixture, cx, |controller, cx| {
        controller.sign_up_with_password(
            "dup@example.test".to_string(),
            SecretString::new("password-8"),
            cx,
        )
    });
    let state = current_state(cx, &fixture.controller);
    assert!(matches!(state, AuthState::Error(AuthError::EmailTaken)));
    assert!(has_event(&fixture.events, |event| matches!(
        event,
        AuthEvent::FlowFailed { .. }
    )));

    drive(&fixture, cx, |controller, cx| {
        controller.sign_in_with_password(
            "dup@example.test".to_string(),
            SecretString::new("wrong-password"),
            cx,
        )
    });
    let state = current_state(cx, &fixture.controller);
    assert!(matches!(
        state,
        AuthState::Error(AuthError::InvalidCredentials)
    ));
    assert_eq!(fixture.store.load("email").await.unwrap(), None);
}

#[gpui::test]
async fn sign_up_while_signed_in_is_a_noop(cx: &mut TestAppContext) {
    let fixture = email_controller(cx, true);
    drive(&fixture, cx, |controller, cx| {
        controller.sign_up_with_password(
            "user@example.test".to_string(),
            SecretString::new("password-8"),
            cx,
        )
    });
    drive(&fixture, cx, |controller, cx| {
        controller.sign_in_with_password(
            "user@example.test".to_string(),
            SecretString::new("password-8"),
            cx,
        )
    });
    drive(&fixture, cx, |controller, cx| {
        controller.sign_up_with_password(
            "other@example.test".to_string(),
            SecretString::new("password-8"),
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
            controller.sign_up_with_password(email.to_string(), SecretString::new("password-8"), cx)
        });
    }
    for email in ["a@example.test", "b@example.test"] {
        drive(&fixture, cx, |controller, cx| {
            controller.sign_in_with_password(email.to_string(), SecretString::new("password-8"), cx)
        });
    }
    assert_eq!(
        current_state(cx, &fixture.controller),
        AuthState::SignedIn {
            account_label: "b@example.test".to_string()
        }
    );
    let stored = fixture.store.load("email").await.unwrap().unwrap();
    assert_eq!(
        stored.account.unwrap().email,
        Some("b@example.test".to_string())
    );
}

#[gpui::test]
async fn sign_out_clears_the_email_state_slots(cx: &mut TestAppContext) {
    let fixture = email_controller(cx, true);
    drive(&fixture, cx, |controller, cx| {
        controller.sign_up_with_password(
            "t@example.test".to_string(),
            SecretString::new("password-8"),
            cx,
        )
    });
    drive(&fixture, cx, |controller, cx| {
        controller.sign_in_with_password(
            "t@example.test".to_string(),
            SecretString::new("password-8"),
            cx,
        )
    });
    drive(&fixture, cx, |controller, cx| controller.sign_out(cx));
    assert_eq!(current_state(cx, &fixture.controller), AuthState::SignedOut);
    cx.read(|cx| {
        assert_eq!(fixture.controller.read(cx).email_status(), None);
    });
    assert_eq!(fixture.store.load("email").await.unwrap(), None);
}

#[gpui::test]
async fn refresh_if_needed_respects_the_margin_and_the_clock(cx: &mut TestAppContext) {
    let fixture = email_controller(cx, true);
    drive(&fixture, cx, |controller, cx| {
        controller.sign_up_with_password(
            "r@example.test".to_string(),
            SecretString::new("password-8"),
            cx,
        )
    });
    drive(&fixture, cx, |controller, cx| {
        controller.sign_in_with_password(
            "r@example.test".to_string(),
            SecretString::new("password-8"),
            cx,
        )
    });

    fixture
        .clock
        .store(EMAIL_IAT + TEST_TTL_SECS - 121, Ordering::SeqCst);
    drive(&fixture, cx, |controller, cx| {
        controller.refresh_if_needed(120, cx)
    });
    let stored = fixture.store.load("email").await.unwrap().unwrap();
    assert_eq!(stored.access_token.expose(), "test-access-0");
    assert!(fixture.backend.refresh_seen().is_empty());

    fixture
        .clock
        .store(EMAIL_IAT + TEST_TTL_SECS - 120, Ordering::SeqCst);
    drive(&fixture, cx, |controller, cx| {
        controller.refresh_if_needed(120, cx)
    });
    let stored = fixture.store.load("email").await.unwrap().unwrap();
    assert_eq!(stored.access_token.expose(), "test-access-1");
    assert_eq!(stored.refresh_token.unwrap().expose(), "test-refresh-0");
}

#[gpui::test]
async fn refresh_if_needed_without_a_clock_or_a_row_is_a_noop(cx: &mut TestAppContext) {
    let fixture = email_controller(cx, false);
    drive(&fixture, cx, |controller, cx| {
        controller.sign_up_with_password(
            "r@example.test".to_string(),
            SecretString::new("password-8"),
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
            SecretString::new("password-8"),
            cx,
        )
    });
    drive(&fixture, cx, |controller, cx| {
        controller.sign_in_with_password(
            "r@example.test".to_string(),
            SecretString::new("password-8"),
            cx,
        )
    });
    fixture.backend.reject_refresh();
    fixture
        .clock
        .store(EMAIL_IAT + TEST_TTL_SECS, Ordering::SeqCst);
    drive(&fixture, cx, |controller, cx| {
        controller.refresh_if_needed(120, cx)
    });
    let state = current_state(cx, &fixture.controller);
    assert!(matches!(
        state,
        AuthState::Error(AuthError::RefreshRejected)
    ));
}

#[gpui::test]
async fn refresh_response_without_a_refresh_token_keeps_the_stored_one(cx: &mut TestAppContext) {
    let fixture = email_controller(cx, true);
    drive(&fixture, cx, |controller, cx| {
        controller.sign_up_with_password(
            "r@example.test".to_string(),
            SecretString::new("password-8"),
            cx,
        )
    });
    drive(&fixture, cx, |controller, cx| {
        controller.sign_in_with_password(
            "r@example.test".to_string(),
            SecretString::new("password-8"),
            cx,
        )
    });
    fixture.backend.omit_refresh_token_once();
    drive(&fixture, cx, |controller, cx| controller.refresh(cx));
    let stored = fixture.store.load("email").await.unwrap().unwrap();
    assert_eq!(stored.access_token.expose(), "test-access-1");
    assert_eq!(stored.refresh_token.unwrap().expose(), "test-refresh-0");
}

#[gpui::test]
async fn sign_out_contacts_revoke_and_survives_a_revoke_failure(cx: &mut TestAppContext) {
    let fixture = email_controller(cx, true);
    drive(&fixture, cx, |controller, cx| {
        controller.sign_up_with_password(
            "s@example.test".to_string(),
            SecretString::new("password-8"),
            cx,
        )
    });
    drive(&fixture, cx, |controller, cx| {
        controller.sign_in_with_password(
            "s@example.test".to_string(),
            SecretString::new("password-8"),
            cx,
        )
    });
    drive(&fixture, cx, |controller, cx| controller.sign_out(cx));
    assert_eq!(current_state(cx, &fixture.controller), AuthState::SignedOut);
    assert_eq!(
        fixture.backend.refresh_seen(),
        vec!["revoke:test-access-0".to_string()]
    );
    assert_eq!(fixture.store.load("email").await.unwrap(), None);

    drive(&fixture, cx, |controller, cx| {
        controller.sign_in_with_password(
            "s@example.test".to_string(),
            SecretString::new("password-8"),
            cx,
        )
    });
    fixture.backend.fail_revoke_once();
    drive(&fixture, cx, |controller, cx| controller.sign_out(cx));
    assert_eq!(current_state(cx, &fixture.controller), AuthState::SignedOut);
    assert_eq!(
        fixture.backend.refresh_seen(),
        vec![
            "revoke:test-access-0".to_string(),
            "revoke:test-access-1".to_string()
        ]
    );
    assert_eq!(fixture.store.load("email").await.unwrap(), None);
    assert!(has_event(&fixture.events, |event| matches!(
        event,
        AuthEvent::SignedOut
    )));
}

/// Provider whose `start_flow`/`refresh` futures park on a channel gate so
/// tests can act while a provider call is in flight.
struct GatedProvider {
    start_gate: Mutex<Option<tokio::sync::mpsc::Receiver<()>>>,
    refresh_gate: Mutex<Option<tokio::sync::mpsc::Receiver<()>>>,
    counter: AtomicU64,
    refresh_calls: AtomicUsize,
}

impl GatedProvider {
    fn new(
        start_gate: Option<tokio::sync::mpsc::Receiver<()>>,
        refresh_gate: Option<tokio::sync::mpsc::Receiver<()>>,
    ) -> Self {
        Self {
            start_gate: Mutex::new(start_gate),
            refresh_gate: Mutex::new(refresh_gate),
            counter: AtomicU64::new(0),
            refresh_calls: AtomicUsize::new(0),
        }
    }

    async fn wait_for(gate: &Mutex<Option<tokio::sync::mpsc::Receiver<()>>>) {
        // Rebind so the MutexGuard drops before the await; the guard is not Send.
        let receiver = gate.lock().expect("gate lock").take();
        if let Some(mut receiver) = receiver {
            let _ = receiver.recv().await;
        }
    }

    fn mint(&self, refresh_token: &str) -> TokenSet {
        let seq = self.counter.fetch_add(1, Ordering::SeqCst);
        TokenSet {
            provider_id: "gate".to_string(),
            access_token: SecretString::new(format!("gate-access-{seq}")),
            refresh_token: Some(SecretString::new(refresh_token)),
            id_token: None,
            token_type: "Bearer".to_string(),
            expires_at: None,
            scope: None,
            account: Some(AccountInfo {
                id: None,
                email: None,
                display_name: Some("Gate account".to_string()),
                avatar_url: None,
            }),
        }
    }
}

#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
impl AuthProvider for GatedProvider {
    fn id(&self) -> &str {
        "gate"
    }

    fn display_name(&self) -> &str {
        "Gate"
    }

    fn refresh_policy(&self) -> RefreshPolicy {
        RefreshPolicy::ReuseUntilDead
    }

    async fn start_flow(&self, _http: reqwest::Client) -> Result<FlowLaunch, AuthError> {
        Self::wait_for(&self.start_gate).await;
        Ok(FlowLaunch {
            authorize_url: "urn:gpui-auth:gate".to_string(),
            state: SecretString::new("gate-state"),
            pkce: None,
            redirect_uri: "urn:gpui-auth:gate:redirect".to_string(),
        })
    }

    async fn complete_callback(
        &self,
        _http: reqwest::Client,
        redirect: RedirectParams,
        _launch: &FlowLaunch,
    ) -> Result<TokenSet, AuthError> {
        if redirect.code.is_none() {
            return Err(AuthError::ProviderDenied {
                error: "missing_code".to_string(),
                description: None,
            });
        }
        Ok(self.mint("gate-refresh-0"))
    }

    async fn refresh(
        &self,
        _http: reqwest::Client,
        refresh_token: &str,
    ) -> Result<TokenSet, AuthError> {
        self.refresh_calls.fetch_add(1, Ordering::SeqCst);
        Self::wait_for(&self.refresh_gate).await;
        Ok(self.mint(refresh_token))
    }

    async fn revoke(&self, _http: reqwest::Client, _access_token: &str) -> Result<(), AuthError> {
        Ok(())
    }
}

fn gated_controller(
    provider: Arc<GatedProvider>,
    cx: &mut TestAppContext,
) -> (Entity<AuthController>, Arc<InMemoryStore>) {
    let store = Arc::new(InMemoryStore::new());
    let controller = cx.new(|cx| {
        AuthController::new(
            Arc::clone(&provider) as Arc<dyn AuthProvider>,
            Arc::clone(&store) as Arc<dyn TokenStore>,
            test_io(),
            cx,
        )
    });
    (controller, store)
}

async fn sign_in_gated(cx: &mut TestAppContext, controller: &Entity<AuthController>) {
    controller.update(cx, |controller, cx| controller.start_sign_in(cx));
    cx.run_until_parked();
    controller.update(cx, |controller, cx| {
        controller.complete_callback("https://cb?code=x&state=gate-state", cx)
    });
    cx.run_until_parked();
}

#[gpui::test]
async fn cancel_discards_a_late_flow_preparation(cx: &mut TestAppContext) {
    let (release_start, start_gate) = tokio::sync::mpsc::channel::<()>(1);
    let (controller, _store) =
        gated_controller(Arc::new(GatedProvider::new(Some(start_gate), None)), cx);
    let (events, _subscription) = collect_events(cx, &controller);
    controller.update(cx, |controller, cx| controller.start_sign_in(cx));
    cx.run_until_parked();
    controller.update(cx, |controller, cx| controller.cancel(cx));
    assert_eq!(current_state(cx, &controller), AuthState::SignedOut);
    release_start.send(()).await.unwrap();
    cx.run_until_parked();
    assert_eq!(current_state(cx, &controller), AuthState::SignedOut);
    assert!(!cx.read(|cx| controller.read(cx).pending_is_some()));
    assert!(
        events
            .borrow()
            .iter()
            .all(|event| !matches!(event, AuthEvent::AuthorizationRequested { .. }))
    );
}

#[gpui::test]
async fn sign_out_discards_a_late_refresh_completion(cx: &mut TestAppContext) {
    let (release_refresh, refresh_gate) = tokio::sync::mpsc::channel::<()>(1);
    let (controller, store) =
        gated_controller(Arc::new(GatedProvider::new(None, Some(refresh_gate))), cx);
    let (events, _subscription) = collect_events(cx, &controller);
    sign_in_gated(cx, &controller).await;
    assert_eq!(
        current_state(cx, &controller),
        AuthState::SignedIn {
            account_label: "Gate account".to_string()
        }
    );

    controller.update(cx, |controller, cx| controller.refresh(cx));
    cx.run_until_parked();
    controller.update(cx, |controller, cx| controller.sign_out(cx));
    cx.run_until_parked();
    assert_eq!(current_state(cx, &controller), AuthState::SignedOut);
    assert_eq!(store.load("gate").await.unwrap(), None);

    release_refresh.send(()).await.unwrap();
    cx.run_until_parked();
    assert_eq!(current_state(cx, &controller), AuthState::SignedOut);
    assert_eq!(store.load("gate").await.unwrap(), None);
    assert_eq!(
        events
            .borrow()
            .iter()
            .filter(|event| matches!(event, AuthEvent::FlowCompleted))
            .count(),
        1
    );
}

#[gpui::test]
async fn overlapping_refreshes_coalesce_into_one_provider_call(cx: &mut TestAppContext) {
    let (release_refresh, refresh_gate) = tokio::sync::mpsc::channel::<()>(1);
    let provider = Arc::new(GatedProvider::new(None, Some(refresh_gate)));
    let (controller, store) = gated_controller(Arc::clone(&provider), cx);
    sign_in_gated(cx, &controller).await;

    controller.update(cx, |controller, cx| controller.refresh(cx));
    controller.update(cx, |controller, cx| controller.refresh(cx));
    cx.run_until_parked();
    assert_eq!(provider.refresh_calls.load(Ordering::SeqCst), 1);
    release_refresh.send(()).await.unwrap();
    cx.run_until_parked();
    assert_eq!(provider.refresh_calls.load(Ordering::SeqCst), 1);

    let stored = store.load("gate").await.unwrap().unwrap();
    assert_eq!(stored.access_token.expose(), "gate-access-1");
    assert_eq!(stored.refresh_token.unwrap().expose(), "gate-refresh-0");
}

struct ForeignProvider;

#[cfg_attr(not(target_family = "wasm"), async_trait::async_trait)]
#[cfg_attr(target_family = "wasm", async_trait::async_trait(?Send))]
impl AuthProvider for ForeignProvider {
    fn id(&self) -> &str {
        "foreign"
    }

    fn display_name(&self) -> &str {
        "Foreign"
    }

    fn refresh_policy(&self) -> RefreshPolicy {
        RefreshPolicy::ReuseUntilDead
    }

    async fn start_flow(&self, _http: reqwest::Client) -> Result<FlowLaunch, AuthError> {
        Ok(FlowLaunch {
            authorize_url: "urn:gpui-auth:foreign".to_string(),
            state: SecretString::new("foreign-state"),
            pkce: None,
            redirect_uri: "urn:gpui-auth:foreign:redirect".to_string(),
        })
    }

    async fn complete_callback(
        &self,
        _http: reqwest::Client,
        redirect: RedirectParams,
        _launch: &FlowLaunch,
    ) -> Result<TokenSet, AuthError> {
        if redirect.code.is_none() {
            return Err(AuthError::ProviderDenied {
                error: "missing_code".to_string(),
                description: None,
            });
        }
        Ok(ForeignProvider::minted("foreign-access", "foreign-refresh"))
    }

    async fn refresh(
        &self,
        _http: reqwest::Client,
        _refresh_token: &str,
    ) -> Result<TokenSet, AuthError> {
        Ok(ForeignProvider::minted("rotated-access", "rotated-refresh"))
    }

    async fn revoke(&self, _http: reqwest::Client, _access_token: &str) -> Result<(), AuthError> {
        Ok(())
    }
}

impl ForeignProvider {
    /// Every response carries a provider id other than the controller's own.
    fn minted(access: &str, refresh: &str) -> TokenSet {
        TokenSet {
            provider_id: "other".to_string(),
            access_token: SecretString::new(access),
            refresh_token: Some(SecretString::new(refresh)),
            id_token: None,
            token_type: "Bearer".to_string(),
            expires_at: None,
            scope: None,
            account: None,
        }
    }
}

#[gpui::test]
async fn foreign_provider_tokens_error_out_and_never_persist(cx: &mut TestAppContext) {
    let store = Arc::new(InMemoryStore::new());
    let controller = cx.new(|cx| {
        AuthController::new(
            Arc::new(ForeignProvider),
            Arc::clone(&store) as Arc<dyn TokenStore>,
            test_io(),
            cx,
        )
    });
    let (events, _subscription) = collect_events(cx, &controller);
    controller.update(cx, |controller, cx| controller.start_sign_in(cx));
    cx.run_until_parked();
    controller.update(cx, |controller, cx| {
        controller.complete_callback("https://cb?code=x&state=foreign-state", cx)
    });
    cx.run_until_parked();
    let state = current_state(cx, &controller);
    assert!(matches!(
        state,
        AuthState::Error(AuthError::InvalidTokenResponse(message))
            if message.contains("provider id mismatch")
    ));
    assert!(events.borrow().iter().any(|event| matches!(
        event,
        AuthEvent::FlowFailed {
            error: AuthError::InvalidTokenResponse(_)
        }
    )));
    assert_eq!(store.load("foreign").await.unwrap(), None);

    let stored = TokenSet {
        provider_id: "foreign".to_string(),
        access_token: SecretString::new("seed-access"),
        refresh_token: Some(SecretString::new("seed-refresh")),
        id_token: None,
        token_type: "Bearer".to_string(),
        expires_at: None,
        scope: None,
        account: None,
    };
    store.save("foreign", &stored).await.unwrap();
    controller.update(cx, |controller, cx| controller.refresh(cx));
    cx.run_until_parked();
    let state = current_state(cx, &controller);
    assert!(matches!(
        state,
        AuthState::Error(AuthError::InvalidTokenResponse(message))
            if message.contains("provider id mismatch")
    ));
    let row = store.load("foreign").await.unwrap().unwrap();
    assert_eq!(row.access_token.expose(), "seed-access");
}
