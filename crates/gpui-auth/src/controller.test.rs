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
