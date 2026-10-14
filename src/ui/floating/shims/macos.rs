use objc2::rc::Retained;
use objc2_app_kit::{
    NSFloatingWindowLevel, NSView, NSWindow, NSWindowCollectionBehavior, NSWindowLevel,
};
use raw_window_handle::AppKitWindowHandle;

const NS_POPUP_WINDOW_LEVEL: NSWindowLevel = 101;

fn ns_window(handle: &AppKitWindowHandle) -> Option<Retained<NSWindow>> {
    // SAFETY: ns_view is the live GPUIView pointer gpui's own
    // HasWindowHandle impl hands out; shims run on the main thread inside
    // AnyWindowHandle::update, and gpui keeps the view alive for the
    // window's lifetime.
    let view: &NSView = unsafe { handle.ns_view.cast::<NSView>().as_ref() };
    view.window()
}

pub(crate) fn set_click_through(handle: &AppKitWindowHandle, enabled: bool) -> bool {
    let Some(window) = ns_window(handle) else {
        return false;
    };
    window.setIgnoresMouseEvents(enabled);
    true
}

pub(crate) fn set_always_on_top(handle: &AppKitWindowHandle, enabled: bool) -> bool {
    let Some(window) = ns_window(handle) else {
        return false;
    };
    if enabled {
        window.setLevel(NS_POPUP_WINDOW_LEVEL);
        window.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
    } else {
        window.setLevel(NSFloatingWindowLevel);
        window.setCollectionBehavior(NSWindowCollectionBehavior::Default);
    }
    true
}
