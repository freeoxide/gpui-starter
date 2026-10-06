use gpui_form::GpuiForm;
use gpui_kit::component::{
    ActiveTheme as _, WindowExt as _,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    form::{Field, field, v_form},
    h_flex,
    input::{Input, InputEvent, InputState},
    v_flex,
};
use gpui_kit::{prelude::*, *};
use koruma::Koruma;
use koruma_collection::{
    collection::NonEmptyValidation,
    format::{EmailValidation, PhoneNumberValidation, UrlValidation},
};

use crate::accessibility::A11yExt as _;

#[derive(Clone, Debug, Default, GpuiForm, Koruma)]
#[gpui_form(koruma(fluent))]
pub struct RegistrationForm {
    #[gpui_form(component(input))]
    #[koruma(NonEmptyValidation::<_>::builder())]
    pub name: String,

    #[gpui_form(component(input))]
    #[koruma(EmailValidation::<_>::builder())]
    pub email: String,

    #[gpui_form(component(input))]
    #[koruma(NonEmptyValidation::<_>::builder())]
    pub password: String,

    #[gpui_form(component(input))]
    #[koruma(PhoneNumberValidation::<_>::builder())]
    pub phone: String,

    #[gpui_form(component(input))]
    #[koruma(UrlValidation::<_>::builder())]
    pub website: String,
}

/// The five input fields of [`RegistrationForm`]; the enum keeps label, input,
/// reset, and error handling in sync per field.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FormField {
    Name,
    Email,
    Password,
    Phone,
    Website,
}

impl FormField {
    const ALL: [FormField; 5] = [
        FormField::Name,
        FormField::Email,
        FormField::Password,
        FormField::Phone,
        FormField::Website,
    ];

    /// Writes the latest input value into the form value holder; a cleared
    /// input maps to None so the holder's required-field validation fails.
    fn set_on(self, holder: &mut RegistrationFormFormValueHolder, value: Option<String>) {
        match self {
            FormField::Name => holder.name = value,
            FormField::Email => holder.email = value,
            FormField::Password => holder.password = value,
            FormField::Phone => holder.phone = value,
            FormField::Website => holder.website = value,
        }
    }

    fn label_key(self) -> &'static str {
        match self {
            FormField::Name => RegistrationFormFormValueHolder::NAME_LABEL_KEY,
            FormField::Email => RegistrationFormFormValueHolder::EMAIL_LABEL_KEY,
            FormField::Password => RegistrationFormFormValueHolder::PASSWORD_LABEL_KEY,
            FormField::Phone => RegistrationFormFormValueHolder::PHONE_LABEL_KEY,
            FormField::Website => RegistrationFormFormValueHolder::WEBSITE_LABEL_KEY,
        }
    }

    fn description_key(self) -> &'static str {
        match self {
            FormField::Name => "registration_form.name_description",
            FormField::Email => "registration_form.email_description",
            FormField::Password => "registration_form.password_description",
            FormField::Phone => "registration_form.phone_description",
            FormField::Website => "registration_form.website_description",
        }
    }
}

fn localized_field_error<E>(errs: &[E], key_of: impl Fn(&E) -> &'static str) -> Option<String> {
    (!errs.is_empty()).then(|| {
        errs.iter()
            .map(|v| crate::i18n::localize(key_of(v)))
            .collect::<Vec<_>>()
            .join("\n")
    })
}

pub struct FormPage {
    current_data: RegistrationFormFormValueHolder,
    fields: RegistrationFormFormFields,
    agree_terms: bool,
    submitted: bool,
    touched: bool,
    /// True when `current_data` has changed since the cached validation was
    /// (re)computed. Gates `validate()` so it runs at most once per edit.
    dirty: bool,
    /// Per-field localized error strings, recomputed only when `dirty && touched`.
    cached_errors: [Option<String>; 5],
    _subscriptions: Vec<Subscription>,
}

impl FormPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let current_data = RegistrationFormFormValueHolder::default();

        let name_input = cx.new(|cx| RegistrationFormFormComponents::name_input(window, cx));
        let email_input = cx.new(|cx| RegistrationFormFormComponents::email_input(window, cx));
        let password_input =
            cx.new(|cx| RegistrationFormFormComponents::password_input(window, cx));
        let phone_input = cx.new(|cx| RegistrationFormFormComponents::phone_input(window, cx));
        let website_input = cx.new(|cx| RegistrationFormFormComponents::website_input(window, cx));

        let _subscriptions = vec![
            Self::subscribe_field(FormField::Name, &name_input, cx),
            Self::subscribe_field(FormField::Email, &email_input, cx),
            Self::subscribe_field(FormField::Password, &password_input, cx),
            Self::subscribe_field(FormField::Phone, &phone_input, cx),
            Self::subscribe_field(FormField::Website, &website_input, cx),
        ];

        Self {
            current_data,
            fields: RegistrationFormFormFields {
                name_input,
                email_input,
                password_input,
                phone_input,
                website_input,
            },
            agree_terms: false,
            submitted: false,
            touched: false,
            dirty: false,
            cached_errors: Default::default(),
            _subscriptions,
        }
    }

    fn subscribe_field(
        field: FormField,
        input: &Entity<InputState>,
        cx: &mut Context<Self>,
    ) -> Subscription {
        cx.subscribe(
            input,
            move |this: &mut FormPage, state: Entity<InputState>, event: &InputEvent, cx| {
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

    fn field_input(&self, field: FormField) -> &Entity<InputState> {
        match field {
            FormField::Name => &self.fields.name_input,
            FormField::Email => &self.fields.email_input,
            FormField::Password => &self.fields.password_input,
            FormField::Phone => &self.fields.phone_input,
            FormField::Website => &self.fields.website_input,
        }
    }

    fn on_reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.current_data = RegistrationFormFormValueHolder::default();
        for field in FormField::ALL {
            self.on_reset_field(field, window, cx);
        }
        self.agree_terms = false;
        self.submitted = false;
        self.touched = false;
        self.dirty = false;
        self.cached_errors = Default::default();
        cx.notify();
    }

    fn on_reset_field(&self, field: FormField, window: &mut Window, cx: &mut Context<Self>) {
        self.field_input(field)
            .update(cx, |s, cx| s.set_value("", window, cx));
    }

    /// Returns the cached localized error string for `field` (rebuilt only when
    /// the page is dirty and touched — see `recompute_validation`).
    fn error_for_field(&self, field: FormField) -> Option<String> {
        self.cached_errors[field as usize].clone()
    }

    /// Recomputes `cached_errors` from `current_data.validate()` and clears the
    /// dirty flag; called from `render` (when dirty && touched) and on submit.
    fn recompute_validation(&mut self) {
        if let Some(e) = self.current_data.validate().err() {
            self.cached_errors[FormField::Name as usize] =
                localized_field_error(&e.name().all(), |v| match v {
                    RegistrationFormFormValueHolderNameKorumaValidator::RequiredValidation(_) => {
                        "validation.required"
                    }
                    RegistrationFormFormValueHolderNameKorumaValidator::NonEmptyValidation(_) => {
                        "validation.non_empty"
                    }
                });
            self.cached_errors[FormField::Email as usize] =
                localized_field_error(&e.email().all(), |v| match v {
                    RegistrationFormFormValueHolderEmailKorumaValidator::RequiredValidation(_) => {
                        "validation.required"
                    }
                    RegistrationFormFormValueHolderEmailKorumaValidator::EmailValidation(_) => {
                        "validation.email"
                    }
                });
            self.cached_errors[FormField::Password as usize] =
                localized_field_error(&e.password().all(), |v| match v {
                    RegistrationFormFormValueHolderPasswordKorumaValidator::RequiredValidation(
                        _,
                    ) => "validation.required",
                    RegistrationFormFormValueHolderPasswordKorumaValidator::NonEmptyValidation(
                        _,
                    ) => "validation.non_empty",
                });
            self.cached_errors[FormField::Phone as usize] =
                localized_field_error(&e.phone().all(), |v| match v {
                    RegistrationFormFormValueHolderPhoneKorumaValidator::RequiredValidation(_) => {
                        "validation.required"
                    }
                    RegistrationFormFormValueHolderPhoneKorumaValidator::PhoneNumberValidation(
                        _,
                    ) => "validation.phone_number",
                });
            self.cached_errors[FormField::Website as usize] =
                localized_field_error(&e.website().all(), |v| match v {
                    RegistrationFormFormValueHolderWebsiteKorumaValidator::RequiredValidation(
                        _,
                    ) => "validation.required",
                    RegistrationFormFormValueHolderWebsiteKorumaValidator::UrlValidation(_) => {
                        "validation.url"
                    }
                });
        } else {
            self.cached_errors = Default::default();
        }
        self.dirty = false;
    }

    /// Renders a single form `field()` row, keyed on the enum so the label,
    /// description, required flag, input, and cached error all stay in sync.
    fn render_field(&self, field_kind: FormField, danger: Hsla) -> Field {
        let required = !matches!(field_kind, FormField::Website);
        let error = self.error_for_field(field_kind);
        let description_text = crate::i18n::localize(field_kind.description_key());
        let label_text = crate::i18n::localize(field_kind.label_key());
        let error_id: ElementId =
            ElementId::Name(SharedString::from(format!("form-error-{:?}", field_kind)));
        let input = self.field_input(field_kind);

        field()
            .label(label_text.clone())
            .required(required)
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
            .child(Input::new(input).aria_label(label_text))
    }
}

impl Render for FormPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.touched && self.dirty {
            self.recompute_validation();
        }

        let danger = cx.theme().danger;
        let title = crate::i18n::localize("form_page_title");
        let subtitle = crate::i18n::localize("form_page_subtitle");

        v_flex()
            .min_h_full()
            .p_6()
            .gap_4()
            .child(
                div()
                    .id("form-page-title")
                    .a11y(Role::Heading, title.clone())
                    .aria_level(1)
                    .text_xl()
                    .font_weight(FontWeight::BOLD)
                    .child(title),
            )
            .child(
                div()
                    .id("form-page-subtitle")
                    .a11y(Role::Paragraph, subtitle.clone())
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(subtitle),
            )
            .when(self.submitted, |this| {
                let success = crate::i18n::localize("form_page_success");
                this.child(
                    div()
                        .id("form-success")
                        .a11y(Role::Status, success.clone())
                        .a11y_live(accesskit::Live::Polite)
                        .p_3()
                        .rounded(cx.theme().radius)
                        .bg(cx.theme().success.opacity(0.1))
                        .border_1()
                        .border_color(cx.theme().success)
                        .text_color(cx.theme().success)
                        .child(success),
                )
            })
            .child(
                v_form()
                    .label_width(px(160.))
                    .child(self.render_field(FormField::Name, danger))
                    .child(self.render_field(FormField::Email, danger))
                    .child(self.render_field(FormField::Password, danger))
                    .child(self.render_field(FormField::Phone, danger))
                    .child(self.render_field(FormField::Website, danger))
                    .child(
                        field().label_indent(false).child(
                            Checkbox::new("agree-terms")
                                .label(crate::i18n::localize("form_agree_terms"))
                                .checked(self.agree_terms)
                                .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                    this.agree_terms = *checked;
                                    cx.notify();
                                })),
                        ),
                    )
                    .child(
                        field().label_indent(false).child(
                            h_flex()
                                .gap_3()
                                .pt_2()
                                .child(
                                    Button::new("submit")
                                        .primary()
                                        .label(crate::i18n::localize("form_submit"))
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.touched = true;
                                            this.recompute_validation();
                                            let valid =
                                                this.cached_errors.iter().all(|e| e.is_none());
                                            if valid && this.agree_terms {
                                                this.submitted = true;
                                                window.push_notification(
                                                    crate::i18n::localize(
                                                        "form_notification_submitted",
                                                    ),
                                                    cx,
                                                );
                                            } else if valid && !this.agree_terms {
                                                window.push_notification(
                                                    crate::i18n::localize(
                                                        "form_notification_agree_terms",
                                                    ),
                                                    cx,
                                                );
                                            } else {
                                                window.push_notification(
                                                    crate::i18n::localize(
                                                        "form_notification_fix_errors",
                                                    ),
                                                    cx,
                                                );
                                            }
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    Button::new("reset")
                                        .ghost()
                                        .label(crate::i18n::localize("form_reset"))
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.on_reset(window, cx);
                                        })),
                                ),
                        ),
                    ),
            )
    }
}
