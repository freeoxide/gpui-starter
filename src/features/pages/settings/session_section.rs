use gpui_auth::AuthState;
use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, Size, avatar::Avatar, button::Button, h_flex, label::Label,
    v_flex,
};
use gpui_kit::{
    Context, InteractiveElement as _, IntoElement, ParentElement as _, Role, Styled as _, div,
    prelude::FluentBuilder as _,
};

use crate::accessibility::A11yExt as _;

pub(super) fn render_session_section(cx: &mut Context<super::SettingsPage>) -> impl IntoElement {
    let snapshot = crate::auth_ui::snapshot(cx);
    let theme = cx.theme();

    let state_row: gpui_kit::AnyElement = match &snapshot.state {
        AuthState::SignedIn { account_label } => h_flex()
            .items_center()
            .gap_3()
            .child(
                Avatar::new()
                    .name(account_label.clone())
                    .with_size(Size::Small),
            )
            .child(v_flex().child(Label::new(account_label.clone())).when_some(
                snapshot.account_email.clone(),
                |this, email| {
                    this.child(
                        div()
                            .id("settings-account-email")
                            .a11y(Role::Paragraph, email.clone())
                            .text_sm()
                            .text_color(theme.muted_foreground)
                            .child(email),
                    )
                },
            ))
            .into_any_element(),
        AuthState::SigningIn | AuthState::SigningUp | AuthState::AwaitingChallenge { .. } => div()
            .id("settings-auth-signing-in")
            .a11y(
                Role::Paragraph,
                crate::i18n::localize("settings_auth_signing_in"),
            )
            .text_color(theme.muted_foreground)
            .child(crate::i18n::localize("settings_auth_signing_in"))
            .into_any_element(),
        AuthState::SignedOut | AuthState::Error(_) => h_flex()
            .items_center()
            .gap_3()
            .child(
                div()
                    .id("settings-auth-signed-out")
                    .a11y(
                        Role::Paragraph,
                        crate::i18n::localize("settings_auth_signed_out"),
                    )
                    .text_color(theme.muted_foreground)
                    .child(crate::i18n::localize("settings_auth_signed_out")),
            )
            .child(
                Button::new("settings-auth-sign-in")
                    .outline()
                    .label(crate::i18n::localize("settings_auth_sign_in"))
                    .on_click(|_, _, cx| {
                        crate::events::emit(
                            crate::events::AppEventKind::Navigate(crate::routes::AppRoute::page(
                                crate::sidebar::Page::SignIn,
                            )),
                            cx,
                        );
                    }),
            )
            .into_any_element(),
    };

    let signed_in = matches!(snapshot.state, AuthState::SignedIn { .. });
    let expiry = crate::auth_ui::format_expiry(snapshot.expires_at)
        .unwrap_or_else(|| crate::i18n::localize("auth_no_expiry"));

    crate::features::tutorial::target(
        "settings-account",
        super::dev_sections::settings_card_base(cx)
            .child(super::dev_sections::section_heading(
                "settings-account-title",
                crate::i18n::localize("settings_auth_title"),
            ))
            .child(state_row)
            .child(super::notifications_section::status_row(
                "auth-provider",
                crate::i18n::localize("settings_auth_provider"),
                snapshot.provider_id.clone(),
            ))
            .child(super::notifications_section::status_row(
                "auth-store",
                crate::i18n::localize("settings_auth_store"),
                snapshot.store_backend,
            ))
            .child(super::notifications_section::status_row(
                "auth-expires",
                crate::i18n::localize("settings_auth_expires"),
                expiry,
            ))
            .when(signed_in, |this| {
                this.child(
                    Button::new("settings-auth-sign-out")
                        .outline()
                        .label(crate::i18n::localize("settings_auth_sign_out"))
                        .on_click(|_, _, cx| {
                            if let Some(entity) = crate::services::auth::controller(cx) {
                                entity.update(cx, |controller, cx| controller.sign_out(cx));
                            }
                        }),
                )
            }),
    )
}
