use std::sync::Arc;

use gpui_auth::{AuthIo, AuthState, InMemoryStore};
use gpui_kit::TestAppContext;

use super::{AccountMenu, initial_of};

fn open_menu(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    cx.add_window(|window, cx| AccountMenu::new(window, cx));
}

#[gpui_kit::test]
async fn menu_renders_under_the_default_snapshot(cx: &mut TestAppContext) {
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
    open_menu(cx);
    cx.read(|cx| {
        assert_eq!(crate::auth_ui::snapshot(cx).state, AuthState::SignedOut);
    });
}

#[gpui_kit::test]
async fn menu_renders_after_sign_in(cx: &mut TestAppContext) {
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
    cx.update(|cx| {
        let controller = crate::services::auth::controller(cx).expect("controller installed");
        controller.update(cx, |controller, cx| controller.start_sign_in(cx));
    });
    cx.run_until_parked();
    cx.update(|cx| assert!(crate::auth_ui::complete_demo_sign_in(cx)));
    cx.run_until_parked();
    open_menu(cx);
    cx.read(|cx| {
        assert!(matches!(
            crate::auth_ui::snapshot(cx).state,
            AuthState::SignedIn { .. }
        ));
    });
    let _ = cx.refresh();
}

#[gpui_kit::test]
async fn initial_of_picks_the_first_alphanumeric_uppercased() {
    assert_eq!(initial_of("Mock account").as_ref(), "M");
    assert_eq!(initial_of("zed dev").as_ref(), "Z");
    assert_eq!(initial_of("").as_ref(), "?");
    assert_eq!(initial_of("* *").as_ref(), "?");
}
