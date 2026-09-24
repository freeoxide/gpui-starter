use gpui::Keystroke;
use gpui_component::kbd::Kbd;

// `secondary-` is gpui's cross-platform modifier: cmd on macOS, ctrl on
// Windows, Linux, and wasm. Binding with cmd- would hit the platform
// modifier, which is the Windows key there and mostly OS-intercepted.
pub const TOGGLE_SEARCH: &str = "secondary-k";
pub const TOGGLE_SIDEBAR: &str = "secondary-b";

/// NavigateToPage keystroke for sidebar slot i (1-based in the UI, 0-based action).
pub fn page(i: usize) -> String {
    format!("secondary-{}", i + 1)
}

/// User-facing label for a keystroke source: Ctrl+K on Windows, ⌘K on macOS.
/// Panics on malformed constants, same contract as KeyBinding::new.
pub fn label(keystroke: &str) -> String {
    Kbd::format(&Keystroke::parse(keystroke).expect("invalid keystroke constant"))
}

#[cfg(test)]
#[path = "keys.test.rs"]
mod keys_test;
