use super::{
    SignInField, SignInFormFormValueHolder, SignInFormFormValueHolderEmailKorumaValidator,
    SignInFormFormValueHolderPasswordKorumaValidator,
};

#[gpui_kit::test]
async fn empty_seeds_fail_required_on_both_fields() {
    let holder = SignInFormFormValueHolder::default();
    let errors = holder.validate().err().expect("validation errors");
    let email_keys = super::error_keys(&errors.email().all(), |validator| match validator {
        SignInFormFormValueHolderEmailKorumaValidator::RequiredValidation(_) => {
            "validation.required"
        }
        SignInFormFormValueHolderEmailKorumaValidator::EmailValidation(_) => "validation.email",
    })
    .expect("email errors");
    assert!(email_keys.contains(&"validation.required"));

    let password_keys = super::error_keys(&errors.password().all(), |validator| match validator {
        SignInFormFormValueHolderPasswordKorumaValidator::RequiredValidation(_) => {
            "validation.required"
        }
        SignInFormFormValueHolderPasswordKorumaValidator::NonEmptyValidation(_) => {
            "validation.non_empty"
        }
    })
    .expect("password errors");
    assert!(password_keys.contains(&"validation.required"));
}

#[gpui_kit::test]
async fn malformed_email_maps_to_the_email_key() {
    let mut holder = SignInFormFormValueHolder::default();
    holder.email = Some("not-an-email".to_string());
    holder.password = Some("x".to_string());
    let errors = holder.validate().err().expect("validation errors");
    let email_keys = super::error_keys(&errors.email().all(), |validator| match validator {
        SignInFormFormValueHolderEmailKorumaValidator::RequiredValidation(_) => {
            "validation.required"
        }
        SignInFormFormValueHolderEmailKorumaValidator::EmailValidation(_) => "validation.email",
    })
    .expect("email errors");
    assert_eq!(email_keys, vec!["validation.email"]);
    assert!(errors.password().all().is_empty());
}

#[gpui_kit::test]
async fn label_keys_pin_the_generated_prefix() {
    assert_eq!(
        SignInFormFormValueHolder::EMAIL_LABEL_KEY,
        "sign_in_form.email_label"
    );
    assert_eq!(
        SignInFormFormValueHolder::PASSWORD_LABEL_KEY,
        "sign_in_form.password_label"
    );
    assert_eq!(SignInField::Email.label_key(), "sign_in_form.email_label");
    assert_eq!(
        SignInField::Password.label_key(),
        "sign_in_form.password_label"
    );
}
