use gpui_form::GpuiForm;
use koruma::Koruma;
use koruma_collection::{collection::NonEmptyValidation, format::EmailValidation};

#[derive(Clone, Debug, Default, GpuiForm, Koruma)]
#[gpui_form(koruma(fluent))]
pub(crate) struct SignInForm {
    #[gpui_form(component(input))]
    #[koruma(EmailValidation::<_>::builder())]
    pub email: String,

    #[gpui_form(component(input))]
    #[koruma(NonEmptyValidation::<_>::builder())]
    pub password: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SignInField {
    Email,
    Password,
}

impl SignInField {
    pub(crate) fn set_on(self, holder: &mut SignInFormFormValueHolder, value: Option<String>) {
        match self {
            SignInField::Email => holder.email = value,
            SignInField::Password => holder.password = value,
        }
    }

    pub(crate) fn label_key(self) -> &'static str {
        match self {
            SignInField::Email => SignInFormFormValueHolder::EMAIL_LABEL_KEY,
            SignInField::Password => SignInFormFormValueHolder::PASSWORD_LABEL_KEY,
        }
    }

    pub(crate) fn description_key(self) -> &'static str {
        match self {
            SignInField::Email => "sign_in_email_description",
            SignInField::Password => "sign_in_password_description",
        }
    }
}

pub(crate) fn error_keys<E>(
    errs: &[E],
    key_of: impl Fn(&E) -> &'static str,
) -> Option<Vec<&'static str>> {
    (!errs.is_empty()).then(|| errs.iter().map(key_of).collect())
}

#[cfg(test)]
#[path = "form.test.rs"]
mod sign_in_form_test;
