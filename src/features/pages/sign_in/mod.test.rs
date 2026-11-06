use std::sync::Arc;

use gpui_auth::{AuthError, AuthIo, AuthState, InMemoryStore};
use gpui_kit::component::input::InputEvent;
use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{Entity, TestAppContext, WindowHandle};

use super::form::SignInField;
use super::{Mode, SignInPage, error_detail_key};

fn mock_auth(cx: &mut TestAppContext) {
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
        crate::services::auth_ui::initialize(cx);
    });
    cx.run_until_parked();
}

fn open_page(cx: &mut TestAppContext) -> (WindowHandle<SignInPage>, Entity<SignInPage>) {
    cx.update(gpui_kit::component::theme::init);
    let mut entity = None;
    let window = cx.add_window(|window, cx| {
        let page = SignInPage::new(window, cx);
        entity = Some(cx.entity());
        page
    });
    (window, entity.expect("page entity"))
}

/// Types `text` into the real input entity: set_value installs the buffer
/// and the emitted Change drives the page's subscription, never the struct.
fn type_into(
    window: &WindowHandle<SignInPage>,
    page: &Entity<SignInPage>,
    field: SignInField,
    text: &str,
    cx: &mut TestAppContext,
) {
    window
        .update(cx, |_, window, cx| {
            page.update(cx, |page, cx| {
                let input = page.field_input(field).clone();
                input.update(cx, |state, cx| {
                    state.set_value(text.to_string(), window, cx);
                    cx.emit(InputEvent::Change);
                });
            });
        })
        .unwrap();
}

#[gpui_kit::test]
async fn email_mode_renders_both_fields_and_their_validation_errors(cx: &mut TestAppContext) {
    let (window, page) = open_page(cx);
    cx.update(|cx| {
        page.update(cx, |page, _| {
            page.mode = Mode::Email;
        })
    });
    cx.update(|cx| page.update(cx, |page, cx| page.try_submit(cx)));
    window
        .update(cx, |_, window, cx| {
            window.render_frame(cx);
            assert!(
                window.try_find("sign-in-error-Email").is_some(),
                "the email field must render its validation error"
            );
            assert!(
                window.try_find("sign-in-error-Password").is_some(),
                "the password field must render its validation error"
            );
        })
        .unwrap();
}

#[gpui_kit::test]
async fn input_change_events_feed_the_form_and_revalidate_on_render(cx: &mut TestAppContext) {
    let (window, page) = open_page(cx);
    cx.update(|cx| {
        page.update(cx, |page, _| {
            page.mode = Mode::Email;
        })
    });
    // An untouched submit caches the required-field errors and marks the
    // form touched, so later edits revalidate at render time.
    cx.update(|cx| page.update(cx, |page, cx| page.try_submit(cx)));

    type_into(&window, &page, SignInField::Email, "a@b.c", cx);
    type_into(&window, &page, SignInField::Email, "", cx);
    cx.read(|cx| {
        let page = page.read(cx);
        assert_eq!(
            page.current_data.email, None,
            "empty input text must clear the field"
        );
        assert!(page.dirty, "a Change event must mark the form dirty");
    });

    type_into(&window, &page, SignInField::Email, "a@b.c", cx);
    cx.read(|cx| {
        let page = page.read(cx);
        assert_eq!(page.current_data.email.as_deref(), Some("a@b.c"));
    });

    // touched + dirty: the next render recomputes from the live values —
    // the typed email drops its error while the empty password keeps one.
    window
        .update(cx, |_, window, cx| window.render_frame(cx))
        .unwrap();
    cx.read(|cx| {
        let page = page.read(cx);
        assert!(page.cached_errors[SignInField::Email as usize].is_none());
        assert!(
            page.cached_errors[SignInField::Password as usize]
                .as_ref()
                .is_some_and(|keys| keys.contains(&"validation.required"))
        );
        assert!(!page.dirty, "render must consume the dirty flag");
    });
}

#[gpui_kit::test]
async fn press_enter_submits_the_event_fed_credentials(cx: &mut TestAppContext) {
    mock_auth(cx);
    let (window, page) = open_page(cx);
    cx.update(|cx| {
        page.update(cx, |page, _| {
            page.mode = Mode::Email;
        })
    });
    type_into(&window, &page, SignInField::Email, "user@example.test", cx);
    type_into(&window, &page, SignInField::Password, "password-8", cx);
    cx.update(|cx| {
        page.update(cx, |page, cx| {
            page.fields.password_input.update(cx, |_, cx| {
                cx.emit(InputEvent::PressEnter {
                    secondary: false,
                    shift: false,
                });
            });
        })
    });
    cx.run_until_parked();
    cx.read(|cx| {
        assert_eq!(
            crate::auth_ui::snapshot(cx).state,
            AuthState::Error(AuthError::PasswordAuthUnavailable),
            "Enter must submit the event-fed fields through the controller"
        );
    });
}

#[gpui_kit::test]
async fn valid_seeds_leave_errors_clear_and_empty_seeds_flag_both_fields(cx: &mut TestAppContext) {
    let (_window, page) = open_page(cx);
    cx.update(|cx| {
        page.update(cx, |page, _| {
            page.current_data.email = Some("a@b.c".to_string());
            page.current_data.password = Some("x".to_string());
            page.recompute_validation();
            assert!(page.cached_errors.iter().all(|errors| errors.is_none()));

            page.current_data.email = None;
            page.current_data.password = None;
            page.recompute_validation();
            assert!(
                page.cached_errors[SignInField::Email as usize]
                    .as_ref()
                    .is_some_and(|keys| keys.contains(&"validation.required"))
            );
            assert!(
                page.cached_errors[SignInField::Password as usize]
                    .as_ref()
                    .is_some_and(|keys| keys.contains(&"validation.required"))
            );
        });
    });
}

#[gpui_kit::test]
async fn submit_without_password_auth_surfaces_the_controller_error(cx: &mut TestAppContext) {
    mock_auth(cx);
    let (_window, page) = open_page(cx);
    cx.update(|cx| {
        page.update(cx, |page, _| {
            page.mode = Mode::Email;
            page.current_data.email = Some("user@example.test".to_string());
            page.current_data.password = Some("password-8".to_string());
        })
    });
    cx.update(|cx| page.update(cx, |page, cx| page.try_submit(cx)));
    cx.run_until_parked();
    cx.read(|cx| {
        assert_eq!(
            crate::auth_ui::snapshot(cx).state,
            AuthState::Error(AuthError::PasswordAuthUnavailable),
            "the typed credentials must never silently swap to a browser flow"
        );
    });
}

/// Installs the native email provider and creates the account the page
/// submit signs in with.
#[cfg(not(target_family = "wasm"))]
fn email_auth(cx: &mut TestAppContext, email: &str) {
    use gpui_auth::{AuthBackend, DemoBackend, EmailPasswordProvider, PasswordAuth, SecretString};

    let backend = Arc::new(DemoBackend::with_params(
        argon2::Params::new(8, 1, 1, None).expect("cost parameters within argon2 floors"),
    ));
    let clock: Arc<dyn Fn() -> i64 + Send + Sync> = Arc::new(|| 1_000);
    let provider = Arc::new(EmailPasswordProvider::new(
        backend as Arc<dyn AuthBackend>,
        clock.clone(),
    ));
    cx.update(|cx| {
        crate::services::session::initialize(cx);
        crate::services::auth::install_with(
            Arc::new(InMemoryStore::new()),
            AuthIo {
                http: reqwest::Client::new(),
                runtime: None,
            },
            provider.clone() as Arc<dyn gpui_auth::AuthProvider>,
            Some(provider as Arc<dyn PasswordAuth>),
            clock,
            cx,
        );
        crate::services::auth_ui::initialize(cx);
    });
    let controller =
        cx.update(|cx| crate::services::auth::controller(cx).expect("controller installed"));
    cx.update(|cx| {
        controller.update(cx, |controller, cx| {
            controller.sign_up_with_password(email.to_string(), SecretString::new("password-8"), cx)
        })
    });
    cx.run_until_parked();
}

#[cfg(not(target_family = "wasm"))]
#[gpui_kit::test]
async fn submit_uses_password_sign_in_and_clears_credentials_at_session_bounds(
    cx: &mut TestAppContext,
) {
    const EMAIL: &str = "user@example.test";
    email_auth(cx, EMAIL);
    let (_window, page) = open_page(cx);
    cx.update(|cx| {
        page.update(cx, |page, _| {
            page.mode = Mode::Email;
            page.current_data.email = Some(EMAIL.to_string());
            page.current_data.password = Some("password-8".to_string());
        })
    });

    cx.update(|cx| page.update(cx, |page, cx| page.try_submit(cx)));
    cx.read(|cx| {
        assert_eq!(
            crate::auth_ui::snapshot(cx).state,
            AuthState::SigningIn,
            "the email form submit must start password sign-in"
        )
    });

    // Enter mid-flight is a no-op: no relaunch, no validation slam on the
    // half-edited form (the same guard the disabled button carries).
    cx.update(|cx| {
        page.update(cx, |page, cx| {
            page.current_data.email = None;
            page.current_data.password = None;
            page.try_submit(cx);
        })
    });
    cx.read(|cx| {
        let page = page.read(cx);
        assert!(
            page.cached_errors.iter().all(|errors| errors.is_none()),
            "Enter during sign-in must not re-validate the form"
        );
    });

    cx.run_until_parked();
    cx.read(|cx| {
        assert!(matches!(
            crate::auth_ui::snapshot(cx).state,
            AuthState::SignedIn { .. }
        ));
        let page = page.read(cx);
        assert!(page.current_data.email.is_none());
        assert!(page.current_data.password.is_none());
        assert!(page.fields.email_input.read(cx).value().is_empty());
        assert!(
            page.fields.password_input.read(cx).value().is_empty(),
            "the password buffer must not outlive the session"
        );
    });

    cx.update(|cx| {
        let controller = crate::services::auth::controller(cx).expect("controller installed");
        controller.update(cx, |controller, cx| controller.sign_out(cx));
    });
    cx.run_until_parked();
    cx.read(|cx| {
        assert_eq!(crate::auth_ui::snapshot(cx).state, AuthState::SignedOut);
    });
}

#[test]
fn error_detail_key_maps_each_controller_error_and_leaves_unknowns_out() {
    let cases = [
        (
            AuthError::InvalidCredentials,
            "sign_in_err_invalid_credentials",
        ),
        (AuthError::Network("offline".into()), "sign_in_err_network"),
        (AuthError::StateMismatch, "sign_in_err_state_mismatch"),
    ];
    for (error, key) in cases {
        assert_eq!(
            error_detail_key(&error),
            Some(key),
            "error {error:?} must keep its closing key"
        );
    }
    assert_eq!(error_detail_key(&AuthError::NoRefreshToken), None);
}
