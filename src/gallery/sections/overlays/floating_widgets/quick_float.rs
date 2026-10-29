use gpui_kit::App;

use crate::shortcuts::{self, RegistryError};
use crate::ui::floating::close_floating;

/// Shortcut route for the dictation HUD: the first trigger opens the screen
/// float, the next closes it (toggle semantics, matching the action name).
pub fn toggle_quick_float(cx: &mut App) {
    if let Some(handle) = super::entry_handle(super::dictation::DICTATION_ID, cx) {
        close_floating(&handle, cx);
        return;
    }
    if let Err(err) = super::spawn_dictation(cx) {
        tracing::warn!(
            target: "gpui_starter::gallery",
            error = %err,
            "quick float failed to open"
        );
    }
}

/// Wire Alt+Shift+D to [`toggle_quick_float`] (OS-global on macOS/Windows,
/// app chord elsewhere). A second registration fails with DuplicateId.
pub fn install_quick_float(cx: &mut App) -> Result<(), RegistryError> {
    shortcuts::set_handler(shortcuts::QUICK_FLOAT, toggle_quick_float, cx)
}

#[cfg(test)]
#[path = "quick_float.test.rs"]
mod quick_float_test;
