//! Removable sign-in page over the plan-21 auth controller. `SignInPage` is
//! the only public surface; the provider registry and form stay private.
//! Removal = delete this directory and revert the seam touches: the
//! Page::SignIn arms and all() entry in shell/sidebar.rs, the VALID_HOSTS
//! entry in shell/route.rs, the AppRoot wiring in shell/root/app_root/
//! state.rs, the mod/pub use lines in features/pages/mod.rs, the auth_ui
//! service (services/mod.rs, lib.rs, app/init.rs install line), the account
//! menu (shell/mod.rs, title_bar field/child), the settings session section
//! and the restored demo buttons, the diagnostics observer and rows, the
//! command arms, the locale keys, and the page count in
//! tests/e2e_navigation.rs.

use gpui_auth::{AuthState, SecretString};
use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Sizable as _, Size,
    avatar::Avatar,
    button::{Button, ButtonVariants as _},
    form::{Field, field, v_form},
    h_flex,
    input::{Input, InputContentType, InputEvent, InputState},
    v_flex,
};
use gpui_kit::{Action, prelude::*, *};

use crate::accessibility::A11yExt as _;

pub(crate) mod form;
pub(crate) mod providers;

use form::{
    SignInField, SignInFormFormComponents, SignInFormFormFields, SignInFormFormValueHolder,
    SignInFormFormValueHolderEmailKorumaValidator,
    SignInFormFormValueHolderPasswordKorumaValidator, error_keys,
};
use providers::{ProviderKind, is_available, providers};

#[derive(Action, Clone, PartialEq, Eq, serde::Deserialize)]
#[action(namespace = sign_in, no_json)]
pub(crate) struct BackToProviders;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Choose,
    Email,
}

pub struct SignInPage {
    mode: Mode,
    current_data: SignInFormFormValueHolder,
    fields: SignInFormFormFields,
    touched: bool,
    dirty: bool,
    cached_errors: [Option<Vec<&'static str>>; 2],
    focus_handle: FocusHandle,
    _subscriptions: Vec<Subscription>,
}

impl SignInPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let email_input = cx.new(|cx| SignInFormFormComponents::email_input(window, cx));
        let password_input = cx.new(|cx| SignInFormFormComponents::password_input(window, cx));
        password_input.update(cx, |state, cx| state.set_masked(true, window, cx));
        let _subscriptions = vec![
            Self::subscribe_field(SignInField::Email, &email_input, cx),
            Self::subscribe_field(SignInField::Password, &password_input, cx),
            cx.subscribe(&password_input, |this, _, event: &InputEvent, cx| {
                if let InputEvent::PressEnter { .. } = event {
                    this.try_submit(cx);
                }
            }),
            cx.observe_global::<crate::services::auth_ui::AuthUiState>(|_, cx| cx.notify()),
        ];
        cx.bind_keys([KeyBinding::new("escape", BackToProviders, Some("SignIn"))]);
        Self {
            mode: Mode::Choose,
            current_data: SignInFormFormValueHolder::default(),
            fields: SignInFormFormFields {
                email_input,
                password_input,
            },
            touched: false,
            dirty: false,
            cached_errors: Default::default(),
            focus_handle: cx.focus_handle().tab_stop(true),
            _subscriptions,
        }
    }

    fn subscribe_field(
        field: SignInField,
        input: &Entity<InputState>,
        cx: &mut Context<Self>,
    ) -> Subscription {
        cx.subscribe(
            input,
            move |this: &mut SignInPage, state: Entity<InputState>, event: &InputEvent, cx| {
                if let InputEvent::Change = event {
                    let text = state.read(cx).value();
                    let value = if text.is_empty() {
                        None
                    } else {
                        Some(text.to_string())
                    };
                    field.set_on(&mut this.current_data, value);
                    this.dirty = true;
                }
            },
        )
    }

    fn field_input(&self, field: SignInField) -> &Entity<InputState> {
        match field {
            SignInField::Email => &self.fields.email_input,
            SignInField::Password => &self.fields.password_input,
        }
    }

    fn error_for_field(&self, field: SignInField) -> Option<String> {
        self.cached_errors[field as usize].as_ref().map(|keys| {
            keys.iter()
                .map(|key| crate::i18n::localize(key))
                .collect::<Vec<_>>()
                .join("\n")
        })
    }

    fn recompute_validation(&mut self) {
        if let Some(errors) = self.current_data.validate().err() {
            self.cached_errors[SignInField::Email as usize] =
                error_keys(&errors.email().all(), |v| match v {
                    SignInFormFormValueHolderEmailKorumaValidator::RequiredValidation(_) => {
                        "validation.required"
                    }
                    SignInFormFormValueHolderEmailKorumaValidator::EmailValidation(_) => {
                        "validation.email"
                    }
                });
            self.cached_errors[SignInField::Password as usize] =
                error_keys(&errors.password().all(), |v| match v {
                    SignInFormFormValueHolderPasswordKorumaValidator::RequiredValidation(_) => {
                        "validation.required"
                    }
                    SignInFormFormValueHolderPasswordKorumaValidator::NonEmptyValidation(_) => {
                        "validation.non_empty"
                    }
                });
        } else {
            self.cached_errors = Default::default();
        }
        self.dirty = false;
    }

    fn try_submit(&mut self, cx: &mut Context<Self>) {
        self.touched = true;
        self.recompute_validation();
        let valid = self.cached_errors.iter().all(|errors| errors.is_none());
        if !valid {
            cx.notify();
            return;
        }
        let email = self.current_data.email.clone().unwrap_or_default();
        let password = self.current_data.password.clone().unwrap_or_default();
        match crate::services::auth::controller(cx) {
            Some(entity) => {
                let provider_id = crate::auth_ui::snapshot(cx).provider_id;
                entity.update(cx, |controller, cx| {
                    if provider_id == "email" {
                        controller.sign_in_with_password(email, SecretString::new(password), cx);
                    } else {
                        controller.start_sign_in(cx);
                    }
                });
                cx.notify();
            }
            None => {
                tracing::warn!(
                    target: "gpui_starter::sign_in",
                    "no auth controller installed; submit dropped"
                );
                cx.notify();
            }
        }
    }

    fn render_chooser(
        &self,
        snapshot: &crate::auth_ui::AuthUiSnapshot,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let specs = providers();
        let total = specs.len();
        let signing_in = matches!(snapshot.state, AuthState::SigningIn);
        let any_disabled = specs.iter().any(|spec| !is_available(spec, cx));
        let heading = crate::i18n::localize("sign_in_provider_heading");

        v_flex()
            .gap_3()
            .child(
                div()
                    .id("sign-in-provider-heading")
                    .a11y(Role::Heading, heading.clone())
                    .aria_level(2)
                    .child(heading.clone()),
            )
            .child(crate::features::tutorial::target(
                "signin-providers",
                v_flex()
                    .id("sign-in-provider-list")
                    .a11y(Role::List, heading)
                    .gap_2()
                    .aria_size_of_set(total)
                    .children(specs.iter().enumerate().map(|(ix, spec)| {
                        let available = is_available(spec, cx);
                        let label = crate::i18n::localize(spec.label_key);
                        v_flex()
                            .id(ElementId::Name(SharedString::from(format!(
                                "sign-in-provider-row-{}",
                                spec.id
                            ))))
                            .a11y(Role::ListItem, label.clone())
                            .aria_position_in_set(ix)
                            .aria_size_of_set(total)
                            .child(
                                Button::new(ElementId::Name(SharedString::from(format!(
                                    "auth-provider-{}",
                                    spec.id
                                ))))
                                .outline()
                                .label(label)
                                .icon(spec.icon.clone())
                                .disabled(!available)
                                .loading(signing_in && available)
                                .on_click(cx.listener(
                                    move |this, _, _, cx| match spec.kind {
                                        ProviderKind::Email => {
                                            this.mode = Mode::Email;
                                            cx.notify();
                                        }
                                        ProviderKind::Browser => {
                                            providers::start_browser_flow(spec, cx)
                                        }
                                    },
                                )),
                            )
                    })),
            ))
            .when(any_disabled, |this| {
                this.child(self.render_note(
                    "sign-in-provider-unavailable",
                    "sign_in_provider_unavailable",
                    cx,
                ))
            })
            .when(snapshot.provider_id == "mock", |this| {
                this.child(self.render_note("sign-in-demo-note", "sign_in_demo_note", cx))
            })
    }

    fn render_note(
        &self,
        id: &'static str,
        key: &'static str,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let text = crate::i18n::localize(key);
        div()
            .id(id)
            .a11y(Role::Paragraph, text.clone())
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .child(text)
    }

    fn render_field(&self, field_kind: SignInField, danger: Hsla) -> Field {
        let error = self.error_for_field(field_kind);
        let description_text = crate::i18n::localize(field_kind.description_key());
        let label_text = crate::i18n::localize(field_kind.label_key());
        let error_id: ElementId =
            ElementId::Name(SharedString::from(format!("sign-in-error-{field_kind:?}")));
        let input = self.field_input(field_kind);
        let input_element = match field_kind {
            SignInField::Email => Input::new(input)
                .aria_label(label_text.clone())
                .into_any_element(),
            SignInField::Password => Input::new(input)
                .mask_toggle()
                .content_type(InputContentType::Password)
                .aria_label(label_text.clone())
                .into_any_element(),
        };

        field()
            .label(label_text.clone())
            .required(true)
            .description_fn(move |_, _| {
                div()
                    .flex()
                    .flex_col()
                    .gap_1()
                    .child(div().child(description_text.clone()))
                    .when_some(error.clone(), |el, err| {
                        el.child(
                            div()
                                .id(error_id.clone())
                                .a11y(Role::Alert, err.clone())
                                .a11y_live(accesskit::Live::Polite)
                                .text_color(danger)
                                .text_xs()
                                .child(err),
                        )
                    })
            })
            .child(input_element)
    }

    fn render_account_card(
        &self,
        account_label: &str,
        snapshot: &crate::auth_ui::AuthUiSnapshot,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let theme = cx.theme();
        let expiry = crate::auth_ui::format_expiry(snapshot.expires_at)
            .unwrap_or_else(|| crate::i18n::localize("auth_no_expiry"));
        let provider_line = format!(
            "{}: {}",
            crate::i18n::localize("settings_auth_provider"),
            snapshot.provider_id
        );
        let expiry_line = format!(
            "{}: {expiry}",
            crate::i18n::localize("settings_auth_expires")
        );
        let hint = crate::i18n::localize("sign_in_signed_in_hint");

        v_flex()
            .gap_3()
            .p_4()
            .rounded(theme.radius)
            .border_1()
            .border_color(theme.border)
            .max_w_96()
            .child(
                h_flex()
                    .items_center()
                    .gap_3()
                    .child(
                        Avatar::new()
                            .name(account_label.to_string())
                            .with_size(Size::Small),
                    )
                    .child(
                        v_flex()
                            .child(
                                div()
                                    .font_weight(FontWeight::BOLD)
                                    .child(account_label.to_string()),
                            )
                            .when_some(snapshot.account_email.clone(), |this, email| {
                                this.child(
                                    div()
                                        .id("sign-in-account-email")
                                        .a11y(Role::Paragraph, email.clone())
                                        .text_sm()
                                        .text_color(theme.muted_foreground)
                                        .child(email),
                                )
                            }),
                    ),
            )
            .child(
                v_flex()
                    .id("sign-in-account-status")
                    .a11y(Role::Paragraph, format!("{provider_line}. {expiry_line}"))
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(provider_line)
                    .child(expiry_line),
            )
            .child(
                div()
                    .id("sign-in-signed-in-hint")
                    .a11y(Role::Paragraph, hint.clone())
                    .text_sm()
                    .text_color(theme.muted_foreground)
                    .child(hint),
            )
            .child(
                Button::new("auth-page-sign-out")
                    .outline()
                    .label(crate::i18n::localize("settings_auth_sign_out"))
                    .on_click(|_, _, cx| {
                        if let Some(entity) = crate::services::auth::controller(cx) {
                            entity.update(cx, |controller, cx| controller.sign_out(cx));
                        }
                    }),
            )
    }

    fn render_error_banner(&self, message: &str, cx: &Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let heading = crate::i18n::localize("sign_in_error_heading");
        let detail = match error_detail_key(message) {
            Some(key) => crate::i18n::localize(key),
            None => format!(
                "{}: {message}",
                crate::i18n::localize("sign_in_err_unknown")
            ),
        };
        div()
            .id("auth-error")
            .a11y(Role::Alert, format!("{heading}. {detail}"))
            .a11y_live(accesskit::Live::Polite)
            .p_3()
            .rounded(theme.radius)
            .border_1()
            .border_color(theme.danger)
            .text_color(theme.danger)
            .child(div().font_weight(FontWeight::BOLD).child(heading))
            .child(div().text_sm().child(detail))
    }

    fn render_form(
        &self,
        snapshot: &crate::auth_ui::AuthUiSnapshot,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let danger = cx.theme().danger;
        let signing_in = matches!(snapshot.state, AuthState::SigningIn);
        let show_demo =
            signing_in && snapshot.provider_id == "mock" && snapshot.demo_authorize_url.is_some();

        v_flex()
            .gap_3()
            .when_some(
                match &snapshot.state {
                    AuthState::Error(message) => Some(message.clone()),
                    _ => None,
                },
                |this, message| this.child(self.render_error_banner(&message, cx)),
            )
            .child(
                v_form()
                    .label_width(px(160.))
                    .child(self.render_field(SignInField::Email, danger))
                    .child(self.render_field(SignInField::Password, danger))
                    .child(
                        field()
                            .label_indent(false)
                            .child(crate::features::tutorial::target(
                                "signin-submit",
                                h_flex()
                                    .gap_3()
                                    .pt_2()
                                    .child(
                                        Button::new("auth-submit")
                                            .primary()
                                            .label(crate::i18n::localize("sign_in_submit"))
                                            .loading(signing_in)
                                            .disabled(signing_in)
                                            .on_click(
                                                cx.listener(|this, _, _, cx| this.try_submit(cx)),
                                            ),
                                    )
                                    .child(
                                        Button::new("auth-use-another")
                                            .ghost()
                                            .label(crate::i18n::localize("sign_in_back"))
                                            .on_click(cx.listener(|this, _, _, cx| {
                                                this.mode = Mode::Choose;
                                                cx.notify();
                                            })),
                                    ),
                            )),
                    ),
            )
            .when(show_demo, |this| {
                this.child(
                    Button::new("auth-demo-complete")
                        .ghost()
                        .label(crate::i18n::localize("sign_in_demo_complete"))
                        .on_click(|_, _, cx| {
                            crate::services::auth_ui::complete_demo_sign_in(cx);
                        }),
                )
            })
    }
}

fn error_detail_key(message: &str) -> Option<&'static str> {
    if message.contains("invalid email or password") {
        Some("sign_in_err_invalid_credentials")
    } else if message.contains("network failure") {
        Some("sign_in_err_network")
    } else if message.contains("TOTP code rejected")
        || message.contains("challenge unknown or expired")
    {
        Some("sign_in_err_challenge")
    } else if message.contains("state mismatch") {
        Some("sign_in_err_state_mismatch")
    } else {
        None
    }
}

impl Render for SignInPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.touched && self.dirty {
            self.recompute_validation();
        }
        let snapshot = crate::auth_ui::snapshot(cx);
        let title = crate::i18n::localize("sign_in_title");
        let subtitle = crate::i18n::localize("sign_in_subtitle");

        v_flex()
            .id("sign-in-page")
            .track_focus(&self.focus_handle)
            .key_context("SignIn")
            .size_full()
            .min_h_full()
            .p_6()
            .gap_4()
            .on_action(cx.listener(|this, _: &BackToProviders, _, cx| {
                this.mode = Mode::Choose;
                cx.notify();
            }))
            .child(crate::features::tutorial::target(
                "signin-title",
                div()
                    .id("sign-in-title")
                    .a11y(Role::Heading, title.clone())
                    .aria_level(1)
                    .text_xl()
                    .font_weight(FontWeight::BOLD)
                    .child(title),
            ))
            .child(
                div()
                    .id("sign-in-subtitle")
                    .a11y(Role::Paragraph, subtitle.clone())
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(subtitle),
            )
            .child(match &snapshot.state {
                AuthState::SignedIn { account_label } => self
                    .render_account_card(account_label, &snapshot, cx)
                    .into_any_element(),
                _ => match self.mode {
                    Mode::Choose => self.render_chooser(&snapshot, cx).into_any_element(),
                    Mode::Email => self.render_form(&snapshot, cx).into_any_element(),
                },
            })
    }
}

#[cfg(test)]
#[path = "mod.test.rs"]
mod sign_in_test;
