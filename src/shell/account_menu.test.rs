use std::sync::Arc;

use gpui_auth::{AuthIo, AuthState, InMemoryStore};
use gpui_kit::{Entity, TestAppContext, VisualTestContext};

use super::{AccountMenu, AccountSignIn, AccountSignOut, initial_of};
use crate::events::{self, AppEventKind};
use crate::page::Page;
use crate::routes::AppRoute;

fn install_mock_auth(cx: &mut TestAppContext) {
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

fn open_menu(cx: &mut TestAppContext) -> (Entity<AccountMenu>, VisualTestContext) {
    cx.update(gpui_kit::component::theme::init);
    let handle = cx.add_window(|_, cx| AccountMenu::new(cx));
    let menu = handle.root(cx).expect("menu root");
    let mut cx = VisualTestContext::from_window(handle.into(), cx);
    cx.update(|window, cx| window.draw(cx).clear(cx));
    let focus = cx.read(|cx| menu.read(cx).focus_handle.clone());
    cx.update(|window, cx| window.focus(&focus, cx));
    (menu, cx)
}

#[gpui_kit::test]
async fn sign_in_action_navigates_to_the_sign_in_page(cx: &mut TestAppContext) {
    install_mock_auth(cx);
    let (_menu, mut cx) = open_menu(cx);
    // Boot wiring queues its own events; drain so only the action's remain.
    cx.update(|_, cx| events::drain(cx));
    cx.update(|window, cx| window.dispatch_action(Box::new(AccountSignIn), cx));
    let mut routes = Vec::new();
    for event in cx.update(|_, cx| events::drain(cx)) {
        if let AppEventKind::Navigate(route) = event.kind {
            routes.push(route);
        }
    }
    assert_eq!(routes, vec![AppRoute::page(Page::SignIn)]);
}

#[gpui_kit::test]
async fn sign_out_action_flips_the_menu_back_to_signed_out(cx: &mut TestAppContext) {
    install_mock_auth(cx);
    cx.update(|cx| {
        let controller = crate::services::auth::controller(cx).expect("controller installed");
        controller.update(cx, |controller, cx| controller.start_sign_in(cx));
    });
    cx.run_until_parked();
    let (_menu, mut cx) = open_menu(cx);
    cx.update(|_, cx| assert!(crate::auth_ui::complete_demo_sign_in(cx)));
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));
    assert_eq!(
        cx.read(|cx| crate::auth_ui::snapshot(cx).state),
        AuthState::SignedIn {
            account_label: "Mock account".to_string()
        }
    );

    cx.update(|window, cx| window.dispatch_action(Box::new(AccountSignOut), cx));
    cx.run_until_parked();
    assert_eq!(
        cx.read(|cx| crate::auth_ui::snapshot(cx).state),
        AuthState::SignedOut
    );
    cx.update(|window, cx| window.draw(cx).clear(cx));
}

#[gpui_kit::test]
async fn initial_of_picks_the_first_alphanumeric_uppercased() {
    assert_eq!(initial_of("Mock account").as_ref(), "M");
    assert_eq!(initial_of("zed dev").as_ref(), "Z");
    assert_eq!(initial_of("").as_ref(), "?");
    assert_eq!(initial_of("* *").as_ref(), "?");
}
