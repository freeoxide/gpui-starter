//! Removable title-bar account menu. Removal = delete this file and
//! account_menu.test.rs, drop the `pub mod account_menu;` line in
//! shell/mod.rs, and revert the account field/construct/child in
//! shell/title_bar.rs plus the titlebar_account* locale keys.

use gpui_auth::AuthState;
use gpui_kit::component::{
    Disableable as _, IconName, Sizable as _,
    button::{Button, ButtonVariants as _},
    menu::DropdownMenu as _,
};
use gpui_kit::{
    Action, Anchor, AnyElement, Context, FocusHandle, InteractiveElement as _, IntoElement,
    ParentElement as _, Render, Role, SharedString, Window, div,
};

use crate::accessibility::A11yExt as _;
use crate::auth_ui::AuthUiSnapshot;

#[derive(Action, Clone, PartialEq, Eq, serde::Deserialize)]
#[action(namespace = account, no_json)]
pub struct AccountSignIn;

#[derive(Action, Clone, PartialEq, Eq, serde::Deserialize)]
#[action(namespace = account, no_json)]
pub struct AccountSignOut;

pub struct AccountMenu {
    focus_handle: FocusHandle,
    _subscriptions: Vec<gpui_kit::Subscription>,
}

impl AccountMenu {
    pub fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let _subscriptions =
            vec![
                cx.observe_global::<crate::services::auth_ui::AuthUiState>(|_, cx| {
                    cx.notify();
                }),
            ];
        Self {
            focus_handle: cx.focus_handle(),
            _subscriptions,
        }
    }
}

impl Render for AccountMenu {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = crate::auth_ui::snapshot(cx);
        crate::features::tutorial::target(
            "titlebar-account",
            div()
                .id("account-menu")
                .a11y(Role::Group, crate::i18n::localize("titlebar_account"))
                .track_focus(&self.focus_handle)
                .on_action(cx.listener(|_, _: &AccountSignIn, _, cx| {
                    crate::events::emit(
                        crate::events::AppEventKind::Navigate(crate::routes::AppRoute::page(
                            crate::sidebar::Page::SignIn,
                        )),
                        cx,
                    );
                }))
                .on_action(cx.listener(|_, _: &AccountSignOut, _, cx| {
                    if let Some(entity) = crate::services::auth::controller(cx) {
                        entity.update(cx, |controller, cx| controller.sign_out(cx));
                    }
                }))
                .child(render_trigger(&snapshot)),
        )
    }
}

fn render_trigger(snapshot: &AuthUiSnapshot) -> AnyElement {
    match &snapshot.state {
        AuthState::SignedIn { account_label } => {
            signed_in_trigger(account_label, snapshot).into_any_element()
        }
        AuthState::SigningIn | AuthState::SigningUp | AuthState::AwaitingChallenge { .. } => {
            Button::new("account-btn")
                .small()
                .ghost()
                .compact()
                .icon(IconName::LoaderCircle)
                .disabled(true)
                .accessibility_label(crate::i18n::localize("titlebar_account_signing_in"))
                .into_any_element()
        }
        state @ (AuthState::SignedOut | AuthState::Error(_)) => {
            signed_out_trigger(matches!(state, AuthState::Error(_))).into_any_element()
        }
    }
}

fn signed_in_trigger(account_label: &str, snapshot: &AuthUiSnapshot) -> impl IntoElement {
    let header = snapshot
        .account_email
        .clone()
        .unwrap_or_else(|| account_label.to_string());
    Button::new("account-btn")
        .small()
        .ghost()
        .compact()
        .label(initial_of(account_label))
        .accessibility_label(account_label.to_string())
        .dropdown_menu(move |menu, _, _| {
            menu.label(header.clone()).separator().menu(
                crate::i18n::localize("titlebar_account_sign_out"),
                Box::new(AccountSignOut),
            )
        })
        .anchor(Anchor::TopRight)
}

fn signed_out_trigger(error: bool) -> impl IntoElement {
    let icon = if error {
        IconName::TriangleAlert
    } else {
        IconName::CircleUser
    };
    let accessibility_label = if error {
        crate::i18n::localize("titlebar_account_error")
    } else {
        crate::i18n::localize("titlebar_account")
    };
    let menu_header = accessibility_label.clone();
    Button::new("account-btn")
        .small()
        .ghost()
        .compact()
        .icon(icon)
        .accessibility_label(accessibility_label)
        .dropdown_menu(move |menu, _, _| {
            menu.label(menu_header.clone()).menu(
                crate::i18n::localize("titlebar_account_sign_in"),
                Box::new(AccountSignIn),
            )
        })
        .anchor(Anchor::TopRight)
}

fn initial_of(label: &str) -> SharedString {
    let initial = label
        .chars()
        .find(|c| c.is_alphanumeric())
        .and_then(|c| c.to_uppercase().next())
        .map(|c| c.to_string())
        .unwrap_or_else(|| "?".to_string());
    SharedString::from(initial)
}

#[cfg(test)]
#[path = "account_menu.test.rs"]
mod account_menu_test;
