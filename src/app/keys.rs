use gpui_kit::component::kbd::Kbd;
use gpui_kit::{Action, App, KeyBinding, KeyBindingContextPredicate, Keystroke, OperatingSystem};

// `secondary-` is cmd on macOS, ctrl on Windows, Linux, and wasm; cmd-
// would bind the Windows key there, which the OS mostly intercepts.
pub const TOGGLE_SEARCH: &str = "secondary-k";
pub const TOGGLE_SIDEBAR: &str = "secondary-b";
// In-app fallback for the quick-float accelerator: every platform with
// app focus. The OS-global registration behind shortcuts::QUICK_FLOAT is macOS and Windows only.
pub const TOGGLE_QUICK_FLOAT: &str = "alt-shift-d";

/// NavigateToPage keystroke for sidebar slot i (1-based in the UI, 0-based action).
pub fn page(i: usize) -> String {
    format!("secondary-{}", i + 1)
}

/// KeyBinding for a `secondary-` or runtime-built keystroke: 0.3.10
/// KeyBinding::new takes only static strings with explicit modifiers, so both
/// route through load here. Panics if `keystrokes` or `context` fails to parse.
pub fn binding<A: Action>(
    keystrokes: &str,
    action: A,
    context: Option<&str>,
    cx: &App,
) -> KeyBinding {
    let context = context.map(|c| {
        KeyBindingContextPredicate::parse(c)
            .expect("invalid context constant")
            .into()
    });
    KeyBinding::load(keystrokes, Box::new(action), context, false, None, cx)
        .expect("invalid keystroke constant")
}

/// User-facing label for a keystroke source: Ctrl+K on Windows, ⌘K on macOS.
/// `os` only selects parsing; Kbd::format renders modifiers for the host OS.
/// Panics on malformed constants, same contract as `binding`.
pub fn label(keystroke: &str, os: OperatingSystem) -> String {
    Kbd::format(&Keystroke::parse_with_os(keystroke, os).expect("invalid keystroke constant"))
}

#[cfg(test)]
#[path = "keys.test.rs"]
mod keys_test;
