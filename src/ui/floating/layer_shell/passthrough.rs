use gpui_kit::App;

use crate::ui::floating::FloatingHandle;

/// Wayland arm of the desktop-widget click-through toggle: applies
/// `set_input_region(Some(&[]) | None)` to the native window. Returns false
/// for overlay or dead handles and any non-Wayland session.
pub fn set_input_passthrough(handle: &FloatingHandle, passthrough: bool, cx: &mut App) -> bool {
    if cx.compositor_name() != "Wayland" {
        return false;
    }
    let FloatingHandle::Native(window) = handle else {
        return false;
    };
    window
        .update(cx, |_, window, _| {
            window.set_input_region(if passthrough { Some(&[]) } else { None })
        })
        .is_ok()
}

#[cfg(test)]
#[path = "passthrough.test.rs"]
mod passthrough_test;
