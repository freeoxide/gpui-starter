use raw_window_handle::Win32WindowHandle;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    GWL_EXSTYLE, GetWindowLongPtrW, HWND_NOTOPMOST, HWND_TOPMOST, SWP_NOACTIVATE, SWP_NOMOVE,
    SWP_NOSIZE, SetWindowLongPtrW, SetWindowPos, WINDOW_EX_STYLE, WS_EX_NOACTIVATE,
    WS_EX_TRANSPARENT,
};

#[cfg(test)]
#[path = "windows.test.rs"]
mod windows_test;

const CLICK_THROUGH_BITS: WINDOW_EX_STYLE = WS_EX_TRANSPARENT | WS_EX_NOACTIVATE;

fn with_ex_bits(current: u32, bits: u32, enable: bool) -> u32 {
    if enable {
        current | bits
    } else {
        current & !bits
    }
}

pub(crate) fn set_click_through(handle: &Win32WindowHandle, enabled: bool) -> bool {
    let hwnd = HWND(handle.hwnd.get() as *mut _);
    // SAFETY: hwnd is this window's own handle from gpui's
    // HasWindowHandle impl; the call runs on the thread that owns the
    // window, the documented safe context for SetWindowLongPtrW.
    let current = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) } as u32;
    let next = with_ex_bits(current, CLICK_THROUGH_BITS.0, enabled);
    // SAFETY: same hwnd ownership; GWL_EXSTYLE with a plain style word.
    unsafe { SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next as isize) };
    true
}

pub(crate) fn set_always_on_top(handle: &Win32WindowHandle, enabled: bool) -> bool {
    let hwnd = HWND(handle.hwnd.get() as *mut _);
    let after = if enabled {
        HWND_TOPMOST
    } else {
        HWND_NOTOPMOST
    };
    // SAFETY: same hwnd ownership; NOMOVE|NOSIZE|NOACTIVATE restrict the
    // call to a z-order band change.
    unsafe {
        SetWindowPos(
            hwnd,
            Some(after),
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
    }
    .is_ok()
}
