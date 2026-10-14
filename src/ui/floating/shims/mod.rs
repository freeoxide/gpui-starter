//! Platform shims over raw window handles: runtime click-through and
//! always-on-top on live floats. Removal: delete this directory and drop
//! `mod shims;` plus the re-exports from `src/ui/floating/mod.rs`.

use gpui_kit::Window;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

#[cfg(target_os = "linux")]
use raw_window_handle::{HasDisplayHandle, RawDisplayHandle};

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "linux")]
mod x11;

#[cfg(not(target_family = "wasm"))]
pub(crate) const LOG: &str = "gpui_starter::ui::floating";

pub(crate) fn set_click_through(window: &mut Window, enabled: bool) -> bool {
    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return false;
    };
    let raw = handle.as_raw();
    match raw {
        #[cfg(target_os = "macos")]
        RawWindowHandle::AppKit(h) => macos::set_click_through(&h, enabled),
        #[cfg(target_os = "windows")]
        RawWindowHandle::Win32(h) => windows::set_click_through(&h, enabled),
        #[cfg(target_os = "linux")]
        RawWindowHandle::Xcb(h) => xcb_display(window).is_some_and(|(conn, screen)| {
            x11::set_click_through(h.window.get(), conn, screen, enabled)
        }),
        RawWindowHandle::Wayland(_) => {
            window.set_input_region(if enabled { Some(&[]) } else { None });
            true
        }
        _ => false,
    }
}

pub(crate) fn set_always_on_top(window: &mut Window, enabled: bool) -> bool {
    #[cfg(target_family = "wasm")]
    let _ = enabled;
    let Ok(handle) = HasWindowHandle::window_handle(window) else {
        return false;
    };
    let raw = handle.as_raw();
    match raw {
        #[cfg(target_os = "macos")]
        RawWindowHandle::AppKit(h) => macos::set_always_on_top(&h, enabled),
        #[cfg(target_os = "windows")]
        RawWindowHandle::Win32(h) => windows::set_always_on_top(&h, enabled),
        #[cfg(target_os = "linux")]
        RawWindowHandle::Xcb(h) => xcb_display(window).is_some_and(|(conn, screen)| {
            x11::set_always_on_top(h.window.get(), conn, screen, enabled)
        }),
        _ => false,
    }
}

#[cfg(target_os = "linux")]
fn xcb_display(window: &mut Window) -> Option<(std::ptr::NonNull<std::ffi::c_void>, i32)> {
    let Ok(display) = HasDisplayHandle::display_handle(window) else {
        return None;
    };
    match display.as_raw() {
        RawDisplayHandle::Xcb(d) => d.connection.zip(Some(d.screen)),
        _ => None,
    }
}
