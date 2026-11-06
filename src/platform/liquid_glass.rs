//! Installs a macOS 26+ Liquid Glass `NSGlassEffectView` inside a GPUI
//! window's native view hierarchy.

use gpui_kit::Window;
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, NSObjectProtocol};
use objc2_app_kit::NSView;
use objc2_foundation::NSArray;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

const LOG: &str = "gpui_starter::liquid_glass";

const NS_VIEW_WIDTH_SIZABLE: u64 = 1 << 1;
const NS_VIEW_HEIGHT_SIZABLE: u64 = 1 << 4;

pub struct LiquidGlass;

impl LiquidGlass {
    /// Insert an `NSGlassEffectView` behind all other subviews of `window`.
    /// No-op when unavailable (macOS < 26 or a non-AppKit backend).
    pub fn install(window: &Window) {
        let Some(glass_cls) = AnyClass::get(c"NSGlassEffectView") else {
            return;
        };
        let Some(ns_view) = Self::get_ns_view(window) else {
            return;
        };
        let Some(ns_window) = ns_view.window() else {
            return;
        };

        unsafe {
            let _: () = objc2::msg_send![&*ns_window, setHasShadow: false];

            let Some(content_view) = ns_window.contentView() else {
                return;
            };
            let frame: objc2_foundation::NSRect = objc2::msg_send![&*content_view, bounds];

            let glass: *mut objc2::runtime::AnyObject = objc2::msg_send![glass_cls, alloc];
            let glass: *mut objc2::runtime::AnyObject =
                objc2::msg_send![glass, initWithFrame: frame];
            let _: () = objc2::msg_send![glass, setCornerRadius: 16.0f64];
            let _: () = objc2::msg_send![glass, setStyle: 1i64];
            // SAFETY: set_variant: is a private NSGlassEffectView setter.
            let _: () = objc2::msg_send![glass, set_variant: 3i64];

            let color_cls = AnyClass::get(c"NSColor").expect("NSColor class should exist");
            let tint: *mut objc2::runtime::AnyObject = objc2::msg_send![
                color_cls,
                colorWithRed: 0.0f64,
                green: 0.0f64,
                blue: 0.0f64,
                alpha: 0.06f64
            ];
            let _: () = objc2::msg_send![glass, setTintColor: tint];

            // Layer-backed content view — required for the glass effect to render.
            let view_cls = AnyClass::get(c"NSView").expect("NSView class should exist");
            let content: *mut objc2::runtime::AnyObject = objc2::msg_send![view_cls, alloc];
            let content: *mut objc2::runtime::AnyObject =
                objc2::msg_send![content, initWithFrame: frame];
            let _: () = objc2::msg_send![content, setWantsLayer: true];
            let _: () = objc2::msg_send![
                content,
                setAutoresizingMask: NS_VIEW_WIDTH_SIZABLE | NS_VIEW_HEIGHT_SIZABLE
            ];
            let _: () = objc2::msg_send![glass, setContentView: content];

            let _: () = objc2::msg_send![
                glass,
                setAutoresizingMask: NS_VIEW_WIDTH_SIZABLE | NS_VIEW_HEIGHT_SIZABLE
            ];

            // Remove any existing NSVisualEffectView that GPUI created for the
            // Blurred background — it clashes with the glass effect.
            if let Some(blur_cls) = AnyClass::get(c"NSVisualEffectView") {
                let subviews: Retained<NSArray<NSView>> =
                    objc2::msg_send![&*content_view, subviews];
                let count = subviews.count();
                for i in (0..count).rev() {
                    let sub = subviews.objectAtIndex(i);
                    if sub.isKindOfClass(blur_cls) {
                        let _: () = objc2::msg_send![&*sub, removeFromSuperview];
                    }
                }
            }

            // Insert below all other subviews so it acts as a background.
            // NSWindowBelow = -1
            let _: () = objc2::msg_send![
                &*content_view,
                addSubview: glass,
                positioned: -1i64,
                relativeTo: std::ptr::null_mut::<objc2::runtime::AnyObject>()
            ];

            tracing::debug!(target: LOG, "Installed NSGlassEffectView");
        }
    }

    fn get_ns_view(window: &Window) -> Option<&NSView> {
        // Use the HasWindowHandle trait method explicitly — Window has an inherent
        // window_handle() that returns AnyWindowHandle, which shadows the trait method.
        let handle = HasWindowHandle::window_handle(window).ok()?;
        let RawWindowHandle::AppKit(appkit) = handle.as_raw() else {
            tracing::debug!(target: LOG, "Not an AppKit window");
            return None;
        };
        // SAFETY: GPUI's AppKit implementation guarantees ns_view is a valid NSView.
        Some(unsafe { appkit.ns_view.cast::<NSView>().as_ref() })
    }
}
