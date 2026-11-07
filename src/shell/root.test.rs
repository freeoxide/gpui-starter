// No `use super::*`: gpui's `test` proc-macro glob would make bare `#[test]`
// recurse infinitely. Import the items under test explicitly.
use super::super::frame_time::is_slow_frame;
use super::{flush_window_bounds, persisted_bounds};
use crate::app::window::RootWindow;
use crate::app_state::{
    AppConfig, PersistedWindowBounds, PersistedWindowState, config, debounce_test_lock, force_save,
    install_for_tests,
};
use gpui_kit::{
    AnyWindowHandle, Bounds, Context, IntoElement, Render, TestAppContext, Window, WindowBounds,
    div, point, px, size,
};

#[test]
fn is_slow_frame_below_threshold() {
    assert!(!is_slow_frame(3_999));
}

#[test]
fn is_slow_frame_at_threshold() {
    assert!(!is_slow_frame(4_000));
}

#[test]
fn is_slow_frame_above_threshold() {
    assert!(is_slow_frame(4_001));
}

#[test]
fn persisted_bounds_copies_pixel_geometry() {
    let bounds = Bounds {
        origin: point(px(24.0), px(-8.0)),
        size: size(px(1400.0), px(900.0)),
    };
    assert_eq!(
        persisted_bounds(&WindowBounds::Windowed(bounds), None),
        PersistedWindowBounds {
            x: 24.0,
            y: -8.0,
            width: 1400.0,
            height: 900.0,
            display_id: None,
            state: PersistedWindowState::Windowed,
        }
    );
}

#[test]
fn persisted_bounds_keeps_fractional_dpi_values() {
    let bounds = Bounds {
        origin: point(px(10.5), px(0.0)),
        size: size(px(700.25), px(450.75)),
    };
    let persisted = persisted_bounds(&WindowBounds::Windowed(bounds), None);
    assert_eq!((persisted.x, persisted.width), (10.5, 700.25));
    assert_eq!((persisted.y, persisted.height), (0.0, 450.75));
}

struct Blank;

impl Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

#[gpui_kit::test]
async fn flush_window_bounds_persists_the_recorded_root_not_the_active_palette(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    let _guard = debounce_test_lock();
    cx.update(|cx| install_for_tests(AppConfig::default(), cx));

    let root = cx.open_window(size(px(1024.), px(768.)), |_, _| Blank);
    let palette = cx.open_window(size(px(320.), px(240.)), |_, _| Blank);
    let root_handle: AnyWindowHandle = root.into();
    let palette_handle: AnyWindowHandle = palette.into();
    cx.update(|cx| cx.set_global(RootWindow(root_handle)));
    // Quit can fire while the palette is the active window; it must not win.
    cx.update(|cx| {
        palette
            .update(cx, |_, window, _| window.activate_window())
            .unwrap();
    });
    assert_eq!(
        cx.read(|cx| cx.active_window()),
        Some(palette_handle),
        "the palette must be the active window for the root choice to be observable"
    );

    cx.update(flush_window_bounds);

    let expected_display = cx.read(|cx| cx.primary_display());
    let expected_uuid = expected_display.as_ref().and_then(|d| d.uuid().ok());
    assert_eq!(
        cx.read(|cx| config(cx).window_bounds),
        Some(PersistedWindowBounds {
            x: 0.0,
            y: 0.0,
            width: 1024.0,
            height: 768.0,
            display_id: expected_uuid.map(|uuid| uuid.to_string()),
            state: PersistedWindowState::Windowed,
        }),
        "the recorded root's geometry must be persisted, not the active palette's"
    );
    // Release the process-global debounce flag for later tests.
    cx.update(force_save);
}

#[gpui_kit::test]
async fn flush_window_bounds_persists_the_geometry_after_a_resize(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let _guard = debounce_test_lock();
    cx.update(|cx| install_for_tests(AppConfig::default(), cx));

    let root = cx.open_window(size(px(800.), px(600.)), |_, _| Blank);
    let root_handle: AnyWindowHandle = root.into();
    cx.update(|cx| cx.set_global(RootWindow(root_handle)));

    cx.update(flush_window_bounds);
    let expected_display = cx.read(|cx| cx.primary_display());
    let expected_uuid = expected_display.as_ref().and_then(|d| d.uuid().ok());
    assert_eq!(
        cx.read(|cx| config(cx).window_bounds),
        Some(PersistedWindowBounds {
            x: 0.0,
            y: 0.0,
            width: 800.0,
            height: 600.0,
            display_id: expected_uuid.map(|uuid| uuid.to_string()),
            state: PersistedWindowState::Windowed,
        })
    );

    cx.simulate_window_resize(root_handle, size(px(640.), px(480.)));
    cx.update(flush_window_bounds);
    assert_eq!(
        cx.read(|cx| config(cx).window_bounds),
        Some(PersistedWindowBounds {
            x: 0.0,
            y: 0.0,
            width: 640.0,
            height: 480.0,
            display_id: expected_uuid.map(|uuid| uuid.to_string()),
            state: PersistedWindowState::Windowed,
        }),
        "a geometry change must reach the persisted bounds, not a stale snapshot"
    );
    // Release the process-global debounce flag for later tests.
    cx.update(force_save);
}
