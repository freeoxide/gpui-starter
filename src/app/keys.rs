use gpui_kit::component::kbd::Kbd;
use gpui_kit::{Action, App, KeyBinding, KeyBindingContextPredicate, Keystroke, OperatingSystem};

// `secondary-` is gpui's cross-platform modifier: cmd on macOS, ctrl on
// Windows, Linux, and wasm. Binding with cmd- would hit the platform
// modifier, which is the Windows key there and mostly OS-intercepted.
pub const TOGGLE_SEARCH: &str = "secondary-k";
pub const TOGGLE_SIDEBAR: &str = "secondary-b";
// In-app fallback for the quick-float accelerator; works on every platform
// while the app has focus. The OS-global Alt+Shift+D registration behind
// shortcuts::QUICK_FLOAT exists on macOS and Windows only.
pub const TOGGLE_QUICK_FLOAT: &str = "alt-shift-d";

/// NavigateToPage keystroke for sidebar slot i (1-based in the UI, 0-based action).
pub fn page(i: usize) -> String {
    format!("secondary-{}", i + 1)
}

/// KeyBinding for a `secondary-` or runtime-built keystroke. The 0.3.10
/// KeyBinding::new only takes static strings with explicit modifiers, so
/// secondary resolution and dynamic input route through load here.
pub fn binding<A: Action>(
    keystrokes: &str,
    action: A,
    context: Option<&str>,
    cx: &App,
) -> KeyBinding {
    let context = context.map(|c| KeyBindingContextPredicate::parse(c).unwrap().into());
    KeyBinding::load(keystrokes, Box::new(action), context, false, None, cx)
        .expect("invalid keystroke constant")
}

/// User-facing label for a keystroke source: Ctrl+K on Windows, ⌘K on macOS.
/// Panics on malformed constants, same contract as `binding`.
pub fn label(keystroke: &str, os: OperatingSystem) -> String {
    Kbd::format(&Keystroke::parse_with_os(keystroke, os).expect("invalid keystroke constant"))
}

#[cfg(test)]
#[path = "keys.test.rs"]
mod keys_test;
