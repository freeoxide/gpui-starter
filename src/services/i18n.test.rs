use super::*;

#[test]
fn missing_key_with_args_renders_the_key_itself() {
    assert_eq!(
        localize_with_args("i18n_no_such_key", &[("version", "1.0.0")]),
        "i18n_no_such_key"
    );
}

#[test]
fn args_substitute_into_the_translation() {
    rust_i18n::set_locale("en");
    assert_eq!(
        localize_with_args("about_version", &[("version", "1.2.3")]),
        "Version 1.2.3"
    );
}
