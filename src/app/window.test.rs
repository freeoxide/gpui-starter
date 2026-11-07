use std::cell::Cell;
use std::rc::Rc;

use gpui_kit::{
    AnyWindowHandle, Bounds, Context, FocusHandle, InteractiveElement, IntoElement, Render,
    TestAppContext, Window, WindowBounds, div, px, size,
};

use super::*;
use crate::app_state::{PersistedWindowBounds, PersistedWindowState};

struct Blank;

impl Render for Blank {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

struct QuitProbe {
    focus_handle: FocusHandle,
    hits: usize,
}

impl QuitProbe {
    fn new(cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            hits: 0,
        }
    }
}

impl Render for QuitProbe {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(|this, _: &crate::app::Quit, _, _| this.hits += 1))
    }
}

/// App-level Quit listener counting dispatches, so the last-window-close
/// hand-off is observable without a real platform quit.
fn quit_dispatch_counter(cx: &mut TestAppContext) -> Rc<Cell<usize>> {
    let hits = Rc::new(Cell::new(0));
    let counter = hits.clone();
    cx.update(|cx| {
        cx.on_action(move |_: &crate::app::Quit, _| counter.set(counter.get() + 1));
    });
    hits
}

#[gpui_kit::test]
// The macOS test platform does not implement window removal.
#[cfg(not(target_os = "macos"))]
async fn closing_the_recorded_root_clears_the_global(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(install_close_tracker);
    let quit = quit_dispatch_counter(cx);
    let window = cx.open_window(size(px(800.), px(600.)), |_, _| Blank);
    let root: AnyWindowHandle = window.into();
    cx.update(|cx| note_root_window(root, cx));
    assert_eq!(cx.read(root_window), Some(root));
    cx.update(|cx| {
        root.update(cx, |_, window, _| window.remove_window())
            .unwrap();
    });
    assert_eq!(
        cx.read(root_window),
        None,
        "a stale root handle must not survive its window"
    );
    assert!(
        cx.update(|cx| cx.windows().is_empty()),
        "closing the root as the last window must satisfy the quit guard"
    );
    assert_eq!(
        quit.get(),
        1,
        "the last window closing must hand off to the Quit action, not a bare quit"
    );
}

#[gpui_kit::test]
async fn closing_an_unrelated_window_keeps_the_recorded_root(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(install_close_tracker);
    let quit = quit_dispatch_counter(cx);
    let root_window_handle = cx.open_window(size(px(800.), px(600.)), |_, _| Blank);
    let other = cx.open_window(size(px(400.), px(300.)), |_, _| Blank);
    let root: AnyWindowHandle = root_window_handle.into();
    cx.update(|cx| note_root_window(root, cx));
    cx.update(|cx| {
        other
            .update(cx, |_, window, _| window.remove_window())
            .unwrap();
    });
    assert_eq!(
        cx.read(root_window),
        Some(root),
        "only the recorded window's close may clear the record"
    );
    assert_eq!(
        quit.get(),
        0,
        "a non-root close must not touch the quit policy"
    );
}

#[gpui_kit::test]
async fn closing_the_recorded_root_leaves_remaining_windows_open(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(install_close_tracker);
    let quit = quit_dispatch_counter(cx);
    let root_window_handle = cx.open_window(size(px(800.), px(600.)), |_, _| Blank);
    let float = cx.open_window(size(px(400.), px(300.)), |_, _| Blank);
    let root: AnyWindowHandle = root_window_handle.into();
    cx.update(|cx| note_root_window(root, cx));
    cx.update(|cx| {
        root.update(cx, |_, window, _| window.remove_window())
            .unwrap();
    });
    assert_eq!(
        cx.read(root_window),
        None,
        "closing the root clears the record even while a window remains"
    );
    assert_eq!(
        cx.update(|cx| cx.windows()),
        vec![float.into()],
        "a remaining window must fail the quit guard's emptiness check"
    );
    assert_eq!(
        quit.get(),
        0,
        "float-only sessions must outlive the root without quitting"
    );
}

#[gpui_kit::test]
async fn dispatch_quit_lands_at_the_recorded_root_not_the_active_window(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let root_window = cx.open_window(size(px(800.), px(600.)), |_, cx| QuitProbe::new(cx));
    let float_window = cx.open_window(size(px(400.), px(300.)), |_, cx| QuitProbe::new(cx));
    let root_probe = root_window.root(cx).expect("root probe");
    let float_probe = float_window.root(cx).expect("float probe");
    let root: AnyWindowHandle = root_window.into();
    let float: AnyWindowHandle = float_window.into();
    cx.update(|cx| note_root_window(root, cx));
    cx.update(|cx| {
        for handle in [root, float] {
            handle
                .update(cx, |_, window, cx| window.draw(cx).clear(cx))
                .unwrap();
        }
    });
    let (root_focus, float_focus) = cx.read(|cx| {
        (
            root_probe.read(cx).focus_handle.clone(),
            float_probe.read(cx).focus_handle.clone(),
        )
    });
    cx.update(|cx| {
        for (handle, focus) in [(root, root_focus), (float, float_focus)] {
            handle
                .update(cx, |_, window, cx| focus.focus(window, cx))
                .unwrap();
        }
        float
            .update(cx, |_, window, _| window.activate_window())
            .unwrap();
    });
    assert_eq!(
        cx.update(|cx| cx.active_window()),
        Some(float),
        "the float must be active for the root routing to be observable"
    );
    cx.update(dispatch_quit);
    assert_eq!(
        cx.read(|cx| (root_probe.read(cx).hits, float_probe.read(cx).hits)),
        (1, 0),
        "Quit must land at the recorded root, not at the active float"
    );
}

#[gpui_kit::test]
async fn restored_bounds_clamps_a_saved_size_to_the_window_minimum(cx: &mut TestAppContext) {
    let persisted = PersistedWindowBounds {
        x: 50_000.0,
        y: 50_000.0,
        width: 200.0,
        height: 100.0,
        display_id: None,
        state: PersistedWindowState::Windowed,
    };
    let (window_bounds, _) = cx.update(|cx| restored_bounds(&persisted, cx));
    assert_eq!(
        window_bounds.get_bounds().size,
        size(px(480.0), px(320.0)),
        "a saved size below the window minimum must be clamped back to it"
    );
}

#[gpui_kit::test]
async fn restored_bounds_centers_on_the_primary_when_the_saved_display_is_gone(
    cx: &mut TestAppContext,
) {
    let persisted = PersistedWindowBounds {
        x: 10.0,
        y: 10.0,
        width: 800.0,
        height: 600.0,
        display_id: Some("00000000-0000-0000-0000-000000000000".to_string()),
        state: PersistedWindowState::Windowed,
    };
    let (window_bounds, display_id) = cx.update(|cx| restored_bounds(&persisted, cx));
    assert_eq!(
        display_id, None,
        "an unknown display id must not reach WindowOptions"
    );
    let expected = cx.update(|cx| Bounds::centered(None, size(px(800.0), px(600.0)), cx));
    assert_eq!(window_bounds, WindowBounds::Windowed(expected));
}

#[gpui_kit::test]
async fn restored_bounds_keeps_the_saved_display_and_open_state(cx: &mut TestAppContext) {
    let uuid = cx.update(|cx| cx.displays().first().unwrap().uuid().unwrap().to_string());
    let persisted = PersistedWindowBounds {
        // Plausible per the corruption ceiling, but off every live display.
        x: 40_000.0,
        y: 40_000.0,
        width: 900.0,
        height: 700.0,
        display_id: Some(uuid),
        state: PersistedWindowState::Maximized,
    };
    let (window_bounds, display_id) = cx.update(|cx| restored_bounds(&persisted, cx));
    let expected_id = cx.update(|cx| cx.displays().first().unwrap().id());
    assert_eq!(display_id, Some(expected_id));
    let expected =
        cx.update(|cx| Bounds::centered(Some(expected_id), size(px(900.0), px(700.0)), cx));
    assert_eq!(
        window_bounds,
        WindowBounds::Maximized(expected),
        "an origin off its display must center on that display, keeping the saved state"
    );
}

#[gpui_kit::test]
async fn restored_bounds_reuses_the_saved_origin_only_where_the_wm_allows_it(
    cx: &mut TestAppContext,
) {
    let uuid = cx.update(|cx| cx.displays().first().unwrap().uuid().unwrap().to_string());
    let persisted = PersistedWindowBounds {
        x: 12.0,
        y: 8.0,
        width: 800.0,
        height: 600.0,
        display_id: Some(uuid),
        state: PersistedWindowState::Windowed,
    };
    let (window_bounds, _) = cx.update(|cx| restored_bounds(&persisted, cx));
    #[cfg(target_os = "linux")]
    {
        // Linux WMs reposition self-placed windows, so restore centers.
        let id = cx.update(|cx| cx.displays().first().unwrap().id());
        let expected = cx.update(|cx| Bounds::centered(Some(id), size(px(800.0), px(600.0)), cx));
        assert_eq!(window_bounds, WindowBounds::Windowed(expected));
    }
    #[cfg(not(target_os = "linux"))]
    {
        let bounds = window_bounds.get_bounds();
        assert_eq!(bounds.origin.x, px(12.0));
        assert_eq!(bounds.origin.y, px(8.0));
    }
}

#[gpui_kit::test]
// The macOS test platform does not implement window removal.
#[cfg(not(target_os = "macos"))]
async fn os_close_requests_on_the_root_are_allowed_off_macos(cx: &mut TestAppContext) {
    let allowed = cx.update(|cx| should_close_root_window(cx));
    assert!(
        allowed,
        "off macOS an OS close request on the root must close the window"
    );
}
