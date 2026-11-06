mod render;
mod state;

pub use state::AppRoot;

use gpui_kit::*;

impl Focusable for AppRoot {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

/// Persist current window bounds immediately, bypassing the debounce; no-op
/// without an open window. Called from the app-quit observer in `app::init`.
pub fn flush_window_bounds(cx: &mut App) {
    // Quit can fire while the palette is the active window; persist the
    // recorded root window's bounds, not whichever window is active
    // (thread-local GetActiveWindow on Windows).
    let Some(window_handle) = crate::app::window::root_window(cx).or_else(|| cx.active_window())
    else {
        return;
    };
    window_handle
        .update(cx, |_, window, cx| {
            let persisted = persisted_bounds(&window.window_bounds(), display_uuid(window, cx));
            crate::app_state::update_config(cx, |config| {
                config.window_bounds = Some(persisted);
            });
        })
        .ok();
}

/// The config shape for a window's bounds; shared by the debounced observer
/// in `state`.
fn persisted_bounds(
    bounds: &WindowBounds,
    display_id: Option<String>,
) -> crate::app_state::PersistedWindowBounds {
    let geometry = bounds.get_bounds();
    crate::app_state::PersistedWindowBounds {
        x: geometry.origin.x.into(),
        y: geometry.origin.y.into(),
        width: geometry.size.width.into(),
        height: geometry.size.height.into(),
        display_id,
        state: match bounds {
            WindowBounds::Windowed(_) => crate::app_state::PersistedWindowState::Windowed,
            WindowBounds::Maximized(_) => crate::app_state::PersistedWindowState::Maximized,
            WindowBounds::Fullscreen(_) => crate::app_state::PersistedWindowState::Fullscreen,
        },
    }
}

/// Stable per-monitor identity for the saved bounds; `None` where the
/// platform cannot name the display.
fn display_uuid(window: &Window, cx: &App) -> Option<String> {
    window
        .display(cx)
        .and_then(|display| display.uuid().ok().map(|uuid| uuid.to_string()))
}

#[cfg(test)]
#[path = "../../root.test.rs"]
mod root_test;
