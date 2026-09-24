pub mod clock;
#[cfg(target_os = "windows")]
pub mod deep_link_registration;
pub mod desktop_shell;
pub mod environment;
pub mod filesystem;
pub mod input;
pub mod ipc;
#[cfg(target_os = "macos")]
pub mod liquid_glass;
pub mod network;
pub mod process;
#[cfg(target_os = "windows")]
pub mod toast_identity;
#[cfg(target_os = "windows")]
pub mod win_toast;
// Browser API bridges (clipboard, hash deep links, favicon/title,
// connectivity events) — wasm32-unknown-unknown only.
#[cfg(target_family = "wasm")]
pub mod web;
