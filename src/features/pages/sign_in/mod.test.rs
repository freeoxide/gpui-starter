use std::sync::Arc;

use gpui_auth::{AuthIo, AuthState, InMemoryStore};
use gpui_kit::TestAppContext;

use super::form::SignInField;
use super::{Mode, SignInPage};

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

fn open_page(cx: &mut TestAppContext) -> gpui_kit::Entity<SignInPage> {
    cx.update(gpui_kit::component::theme::init);
    let mut entity = None;
    cx.add_window(|window, cx| {
        let page = SignInPage::new(window, cx);
        entity = Some(cx.entity());
        page
    });
    entity.expect("page entity")
}

#[gpui_kit::test]
async fn page_constructs_and_renders_in_choose_mode(cx: &mut TestAppContext) {
    let page = open_page(cx);
    cx.read(|cx| {
        let page = page.read(cx);
        assert_eq!(page.mode, Mode::Choose);
    });
}

#[gpui_kit::test]
async fn email_mode_renders_both_fields(cx: &mut TestAppContext) {
    mock_auth(cx);
    cx.update(gpui_kit::component::theme::init);
    cx.add_window(|window, cx| {
        let mut page = SignInPage::new(window, cx);
        page.mode = Mode::Email;
        page
    });
}

#[gpui_kit::test]
async fn valid_seeds_leave_errors_clear_and_empty_seeds_flag_both_fields(cx: &mut TestAppContext) {
    let page = open_page(cx);
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
async fn submit_over_the_mock_reaches_signing_in_then_signed_in(cx: &mut TestAppContext) {
    mock_auth(cx);
    let page = open_page(cx);
    cx.update(|cx| {
        page.update(cx, |page, _| {
            page.current_data.email = Some("user@example.test".to_string());
            page.current_data.password = Some("password-8".to_string());
        });
    });
    cx.update(|cx| {
        page.update(cx, |page, cx| page.try_submit(cx));
    });
    cx.run_until_parked();
    cx.read(|cx| {
        assert_eq!(crate::auth_ui::snapshot(cx).state, AuthState::SigningIn,);
    });
    cx.update(|cx| assert!(crate::auth_ui::complete_demo_sign_in(cx)));
    cx.run_until_parked();
    cx.read(|cx| {
        assert_eq!(
            crate::auth_ui::snapshot(cx).state,
            AuthState::SignedIn {
                account_label: "Mock account".to_string()
            }
        );
    });
    let _ = cx.refresh();
    cx.update(|cx| {
        let controller = crate::services::auth::controller(cx).expect("controller installed");
        controller.update(cx, |controller, cx| controller.sign_out(cx));
    });
    cx.run_until_parked();
    cx.read(|cx| {
        assert_eq!(crate::auth_ui::snapshot(cx).state, AuthState::SignedOut);
    });
    let _ = cx.refresh();
}
