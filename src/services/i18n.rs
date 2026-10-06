/// Localizes an app-owned message key via this crate's rust-i18n backend
/// (initialized in `src/lib.rs`); a missing key renders as the key itself.
pub fn localize(id: &str) -> String {
    rust_i18n::t!(id).to_string()
}

pub fn detect_system_locale() -> String {
    sys_locale::get_locale().unwrap_or_else(|| "en".to_string())
}
