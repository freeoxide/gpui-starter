use gpui_kit::{App, Global, SharedString};
use unic_langid::LanguageIdentifier;

pub const LOCALE_EN: &str = "en";
pub const LOCALE_ZH_CN: &str = "zh-CN";

#[derive(Clone, Debug)]
pub struct LocaleState(pub SharedString);

impl Global for LocaleState {}

pub fn current_locale(cx: &App) -> SharedString {
    cx.global::<LocaleState>().0.clone()
}

pub fn set_locale(locale: &str, cx: &mut App) {
    rust_i18n::set_locale(locale);
    let language = locale
        .parse::<LanguageIdentifier>()
        .unwrap_or_else(|_| "en".parse().expect("en is a valid language identifier"));
    let _ = gpui_form::i18n::change_locale(cx, language);
    cx.set_global::<LocaleState>(LocaleState(SharedString::from(locale.to_string())));
    crate::app_state::update_config(cx, |config| {
        config.locale = locale.to_string();
    });
    cx.refresh_windows();
}
