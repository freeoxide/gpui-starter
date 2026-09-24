pub mod actions;
pub mod assets;
pub mod init;
pub mod keys;
pub mod lifecycle;
pub mod locale;
#[cfg(not(target_family = "wasm"))]
pub mod reload;
pub mod theme;
pub mod window;

// ---------------------------------------------------------------------------
// Re-exports — preserve the public API
// ---------------------------------------------------------------------------

pub use actions::{
    About, ExecuteCommand, Languages, OpenDiagnostics, Quit, Restart, SelectFont, SelectLocale,
    SelectRadius, SwitchTheme, SwitchThemeMode, ToggleSearch, TriggerTestPanic,
};

#[cfg(not(target_family = "wasm"))]
pub use reload::{is_reload_requested, perform_reload, request_reload};

pub use locale::{LOCALE_EN, LOCALE_ZH_CN, LocaleState, current_locale, set_locale};

pub use theme::{set_theme_mode, set_theme_mode_with_record};

pub use init::init;

pub use window::create_new_window;
