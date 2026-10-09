#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "macos")]
pub use macos::setup;
#[cfg(target_os = "windows")]
pub use windows::setup;

const LOG: &str = "gpui_starter::tray";
