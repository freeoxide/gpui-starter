//! NSWindow reach-in: borderless non-activating panel pinned over the notch
//! strip, resized top-anchored through `setFrame:display:animate:`.

use gpui_kit::{Bounds, Pixels, Window};
use objc2::MainThreadMarker;
use objc2_app_kit::{NSScreen, NSView, NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask};
use objc2_foundation::{NSPoint, NSRect, NSSize};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

use super::geometry::NotchLayout;

// The nonactivating bit is absent from objc2-app-kit's generated bitflags;
// gpui constructs the same value the same way.
const NONACTIVATING_PANEL: NSWindowStyleMask = NSWindowStyleMask::from_bits_retain(1 << 7);

fn ns_window(window: &Window) -> Option<objc2::rc::Retained<NSWindow>> {
    // The trait call is deliberate: Window's inherent window_handle() returns
    // AnyWindowHandle and shadows the trait method.
    let handle = HasWindowHandle::window_handle(window).ok()?;
    let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
        return None;
    };
    // SAFETY: GPUI's AppKit implementation guarantees ns_view is a valid NSView.
    unsafe { appkit.ns_view.cast::<NSView>().as_ref() }.window()
}

fn appkit_rect(target: Bounds<Pixels>, screen_frame: NSRect) -> NSRect {
    NSRect::new(
        NSPoint::new(
            screen_frame.origin.x + target.origin.x.to_f64(),
            screen_frame.origin.y + screen_frame.size.height
                - target.origin.y.to_f64()
                - target.size.height.to_f64(),
        ),
        NSSize::new(target.size.width.to_f64(), target.size.height.to_f64()),
    )
}

pub(crate) fn apply_notch_policy(window: &mut Window, layout: &NotchLayout) -> bool {
    let Some(mtm) = MainThreadMarker::new() else {
        return false;
    };
    let Some(ns_window) = ns_window(window) else {
        return false;
    };
    let Some(screen) = NSScreen::mainScreen(mtm) else {
        return false;
    };
    ns_window.setStyleMask(NSWindowStyleMask::Borderless | NONACTIVATING_PANEL);
    // Without FullScreenAuxiliary the system hides the panel while any window
    // is fullscreen on its display.
    ns_window.setCollectionBehavior(NSWindowCollectionBehavior::CanJoinAllSpaces);
    ns_window.setFrame_display_animate(appkit_rect(layout.collapsed, screen.frame()), true, false);
    true
}

pub(crate) fn resize_to(window: &Window, target: Bounds<Pixels>, animate: bool) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let Some(ns_window) = ns_window(window) else {
        return;
    };
    let Some(screen) = NSScreen::mainScreen(mtm) else {
        return;
    };
    ns_window.setFrame_display_animate(appkit_rect(target, screen.frame()), true, animate);
}
