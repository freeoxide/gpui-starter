//! macOS 26+ Liquid Glass utilities: create, find, and configure an
//! `NSGlassEffectView` inside a GPUI window's native view hierarchy.

use gpui_kit::Window;
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, NSObjectProtocol};
use objc2_app_kit::NSView;
use objc2_foundation::NSArray;
use raw_window_handle::{HasWindowHandle, RawWindowHandle};

const LOG: &str = "liquid_glass";

const NS_VIEW_WIDTH_SIZABLE: u64 = 1 << 1;
const NS_VIEW_HEIGHT_SIZABLE: u64 = 1 << 4;

/// Configuration for `NSGlassEffectView`.
#[derive(Debug, Clone)]
pub struct LiquidGlassConfig {
    pub corner_radius: f64,
    /// Glass style: `0` = default (heavier), `1` = clear/thin.
    pub style: i64,
    /// `0` = default, higher values change the glass appearance.
    pub variant: i64,
    pub tint: Option<LiquidGlassTint>,
    pub disable_shadow: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum LiquidGlassTint {
    Grey,
    Blue,
    /// Custom RGBA color (each component 0.0–1.0).
    Custom {
        r: f64,
        g: f64,
        b: f64,
        a: f64,
    },
}

impl Default for LiquidGlassConfig {
    fn default() -> Self {
        Self {
            corner_radius: 16.0,
            style: 1,
            variant: 3,
            tint: Some(LiquidGlassTint::Custom {
                r: 0.0,
                g: 0.0,
                b: 0.0,
                a: 0.06,
            }),
            disable_shadow: true,
        }
    }
}

pub struct LiquidGlassView {
    view: *mut objc2::runtime::AnyObject,
}

// SAFETY: The pointer comes from the view hierarchy of a live GPUI window.
// We only read from it and send messages; the window owns the view.
unsafe impl Send for LiquidGlassView {}
unsafe impl Sync for LiquidGlassView {}

impl LiquidGlassView {
    pub fn set_corner_radius(&self, radius: f64) {
        unsafe {
            let _: () = objc2::msg_send![self.view, setCornerRadius: radius];
        }
    }

    /// Set the glass style (`0` = default, `1` = clear).
    pub fn set_style(&self, style: i64) {
        unsafe {
            let _: () = objc2::msg_send![self.view, setStyle: style];
        }
    }

    /// Set the glass variant (controls intensity/pattern).
    pub fn set_variant(&self, variant: i64) {
        // SAFETY: set_variant: is a private NSGlassEffectView setter.
        unsafe {
            let _: () = objc2::msg_send![self.view, set_variant: variant];
        }
    }

    pub fn set_tint(&self, tint: LiquidGlassTint) {
        unsafe {
            let color: *mut objc2::runtime::AnyObject = make_nscolor(tint);
            let _: () = objc2::msg_send![self.view, setTintColor: color];
        }
    }

    pub fn clear_tint(&self) {
        // Use *mut AnyObject (encodes as '@') not *mut u8 (encodes as '*').
        unsafe {
            let _: () = objc2::msg_send![
                self.view,
                setTintColor: std::ptr::null_mut::<objc2::runtime::AnyObject>()
            ];
        }
    }
}

fn make_nscolor(tint: LiquidGlassTint) -> *mut objc2::runtime::AnyObject {
    unsafe {
        let cls = AnyClass::get(c"NSColor").expect("NSColor class should exist");
        match tint {
            LiquidGlassTint::Grey => objc2::msg_send![cls, systemGrayColor],
            LiquidGlassTint::Blue => objc2::msg_send![cls, systemBlueColor],
            LiquidGlassTint::Custom { r, g, b, a } => {
                objc2::msg_send![cls, colorWithRed: r, green: g, blue: b, alpha: a]
            }
        }
    }
}

pub struct LiquidGlass;

impl LiquidGlass {
    /// Create, configure, and insert an `NSGlassEffectView` below all other
    /// subviews; `None` when unavailable (macOS < 26).
    pub fn install(window: &Window, config: &LiquidGlassConfig) -> Option<LiquidGlassView> {
        let glass_cls = AnyClass::get(c"NSGlassEffectView")?;

        let ns_view = Self::get_ns_view(window)?;
        let ns_window = ns_view.window()?;

        unsafe {
            if config.disable_shadow {
                let _: () = objc2::msg_send![&*ns_window, setHasShadow: false];
            }

            let content_view = ns_window.contentView()?;
            let frame: objc2_foundation::NSRect = objc2::msg_send![&*content_view, bounds];

            let glass: *mut objc2::runtime::AnyObject = objc2::msg_send![glass_cls, alloc];
            let glass: *mut objc2::runtime::AnyObject =
                objc2::msg_send![glass, initWithFrame: frame];
            let _: () = objc2::msg_send![glass, setCornerRadius: config.corner_radius];
            let _: () = objc2::msg_send![glass, setStyle: config.style];
            let _: () = objc2::msg_send![glass, set_variant: config.variant];

            match &config.tint {
                Some(tint) => {
                    let color = make_nscolor(*tint);
                    let _: () = objc2::msg_send![glass, setTintColor: color];
                }
                None => {
                    let _: () = objc2::msg_send![
                        glass,
                        setTintColor: std::ptr::null_mut::<objc2::runtime::AnyObject>()
                    ];
                }
            }

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
            Some(LiquidGlassView { view: glass })
        }
    }

    /// Find an existing `NSGlassEffectView`; `None` when absent or on macOS < 26.
    pub fn find(window: &Window) -> Option<LiquidGlassView> {
        let glass_cls = AnyClass::get(c"NSGlassEffectView")?;
        let ns_view = Self::get_ns_view(window)?;

        let result = Self::find_in_subviews(ns_view, glass_cls);
        if result.is_some() {
            tracing::debug!(target: LOG, "Found NSGlassEffectView");
        }
        result
    }

    pub fn apply(window: &Window, config: &LiquidGlassConfig) -> bool {
        let Some(glass) = Self::find(window) else {
            return false;
        };
        glass.set_corner_radius(config.corner_radius);
        glass.set_style(config.style);
        glass.set_variant(config.variant);
        match &config.tint {
            Some(tint) => glass.set_tint(*tint),
            None => glass.clear_tint(),
        }
        true
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

    fn find_in_subviews(view: &NSView, target: &AnyClass) -> Option<LiquidGlassView> {
        let subviews = view.subviews();
        let count = subviews.count();

        for i in 0..count {
            let subview = subviews.objectAtIndex(i);

            if subview.isKindOfClass(target) {
                return Some(LiquidGlassView {
                    view: subview.as_ref() as *const NSView as *mut objc2::runtime::AnyObject,
                });
            }

            if let Some(found) = Self::find_in_subviews(&subview, target) {
                return Some(found);
            }
        }

        None
    }
}
