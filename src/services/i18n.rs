use std::sync::OnceLock;

use es_fluent::{
    FluentArgs, FluentLocalizer as _, FluentMessage,
    registry::{StaticFluentDomain, StaticFluentEntryId, StaticFluentMessageKey},
};
use es_fluent_manager_embedded::EmbeddedI18n;

es_fluent_manager_embedded::define_i18n_module!();

static I18N: OnceLock<EmbeddedI18n> = OnceLock::new();

#[derive(Debug, thiserror::Error)]
pub enum I18nError {
    #[error("i18n initialization failed: {0}")]
    InitFailed(#[source] Box<dyn std::error::Error + Send + Sync>),
}

pub fn init_i18n(lang: es_fluent::unic_langid::LanguageIdentifier) -> Result<(), I18nError> {
    EmbeddedI18n::try_new_with_language(lang)
        .map_err(|e| I18nError::InitFailed(Box::new(e)))
        .map(|i18n| {
            let _ = I18N.set(i18n);
        })
}

pub fn i18n() -> &'static EmbeddedI18n {
    I18N.get_or_init(|| {
        tracing::warn!("i18n not initialized, using fallback");
        EmbeddedI18n::try_new().expect("embedded i18n fallback must succeed")
    })
}

/// Localizes a message `id` from this crate's fallback Fluent bundle; es-fluent
/// 0.18 scopes the key by owner/domain, both defaulting to `CARGO_PKG_NAME`.
pub fn localize<'a>(id: &'static str, args: Option<&'a FluentArgs<'a>>) -> String {
    let domain = StaticFluentDomain::from_package_name(env!("CARGO_PKG_NAME"));
    let key = StaticFluentMessageKey::new(
        domain,
        domain,
        StaticFluentEntryId::try_new(id).expect("fluent message id must be valid"),
    );
    i18n().localize(key, args).unwrap_or_else(|| id.to_string())
}

pub fn localize_message<T: FluentMessage + ?Sized>(message: &T) -> String {
    i18n().localize_message(message)
}

pub fn detect_system_locale() -> String {
    sys_locale::get_locale().unwrap_or_else(|| "en".to_string())
}
