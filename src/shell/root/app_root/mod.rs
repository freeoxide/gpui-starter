mod render;
mod state;

pub use state::AppRoot;

use gpui::*;

impl Focusable for AppRoot {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

/// Persist current window bounds immediately, bypassing the debounce; no-op
/// without an open window. Called from the `Quit` handler.
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
            let persisted = persisted_bounds(window.window_bounds().get_bounds());
            crate::app_state::update_config(cx, |config| {
                config.window_bounds = Some(persisted);
            });
        })
        .ok();
}

/// The config shape for a window's bounds; shared by the debounced observer
/// in `state`.
fn persisted_bounds(bounds: Bounds<Pixels>) -> crate::app_state::PersistedWindowBounds {
    crate::app_state::PersistedWindowBounds {
        x: bounds.origin.x.into(),
        y: bounds.origin.y.into(),
        width: bounds.size.width.into(),
        height: bounds.size.height.into(),
    }
}

#[cfg(test)]
#[path = "../../root.test.rs"]
mod root_test;
