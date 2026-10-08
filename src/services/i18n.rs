/// Localizes an app-owned message key via this crate's rust-i18n backend
/// (initialized in `src/lib.rs`); a missing key renders as the key itself.
pub fn localize(id: &str) -> String {
    rust_i18n::t!(id).to_string()
}

/// [`localize`] with `%{name}` placeholder substitution, applying the same
/// replacement `rust_i18n::t!` uses for inline arguments.
pub fn localize_with_args(id: &str, args: &[(&str, &str)]) -> String {
    let patterns: Vec<&str> = args.iter().map(|(name, _)| *name).collect();
    let values: Vec<String> = args.iter().map(|(_, value)| value.to_string()).collect();
    rust_i18n::replace_patterns(&rust_i18n::t!(id), &patterns, &values)
}

pub fn detect_system_locale() -> String {
    sys_locale::get_locale().unwrap_or_else(|| "en".to_string())
}

#[cfg(test)]
#[path = "i18n.test.rs"]
mod i18n_test;
