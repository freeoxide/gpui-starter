use gpui::{
    AnyWindowHandle, App, AppContext as _, Bounds, Focusable as _, Global, SharedString,
    WindowBounds, WindowKind, WindowOptions, px, size,
};
use gpui_component::TitleBar;

/// The main app window, recorded at creation so code off the OS-input path
/// (tray, shutdown) can reach it without `active_window()`, which resolves
/// via thread-local `GetActiveWindow()` on Windows.
#[derive(Clone, Copy)]
pub struct RootWindow(pub AnyWindowHandle);

impl Global for RootWindow {}

fn note_root_window(handle: AnyWindowHandle, cx: &mut App) {
    cx.set_global(RootWindow(handle));
}

/// The recorded main window, if one has been opened.
pub fn root_window(cx: &App) -> Option<AnyWindowHandle> {
    cx.try_global::<RootWindow>().map(|root| root.0)
}

/// Dispatch Quit at the recorded root window, falling back to app-level
/// dispatch. `App::dispatch_action` routes at `active_window()` — thread-local
/// `GetActiveWindow()` on Windows — which still reports the closing palette
/// after an Esc and drops the action.
pub fn dispatch_quit(cx: &mut App) {
    let routed = root_window(cx).is_some_and(|root| {
        root.update(cx, |_, window, cx| {
            window.dispatch_action(Box::new(crate::app::Quit), cx)
        })
        .is_ok()
    });
    if !routed {
        cx.dispatch_action(&crate::app::Quit);
    }
}

pub fn create_new_window(title: &str, cx: &mut App) {
    let mut window_size = size(px(1400.0), px(900.0));
    if let Some(display) = cx.primary_display() {
        let display_size = display.bounds().size;
        window_size.width = window_size.width.min(display_size.width * 0.85);
        window_size.height = window_size.height.min(display_size.height * 0.85);
    }
    let persisted_bounds = crate::app_state::config(cx).window_bounds;
    let window_bounds = if let Some(bounds) = persisted_bounds {
        Bounds {
            origin: gpui::point(px(bounds.x), px(bounds.y)),
            size: gpui::size(px(bounds.width), px(bounds.height)),
        }
    } else {
        Bounds::centered(None, window_size, cx)
    };
    let title: SharedString = title.into();

    cx.spawn(async move |cx| {
        let options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(window_bounds)),
            titlebar: Some(TitleBar::title_bar_options()),
            window_min_size: Some(gpui::Size {
                width: px(480.),
                height: px(320.),
            }),
            kind: WindowKind::Normal,
            // The app id becomes Wayland app_id / X11 WM_CLASS, matching the
            // shipped .desktop's StartupWMClass so grouping + icons work.
            app_id: Some("gpui-starter".to_string()),
            #[cfg(target_os = "linux")]
            window_background: gpui::WindowBackgroundAppearance::Transparent,
            #[cfg(target_os = "linux")]
            window_decorations: Some(gpui::WindowDecorations::Client),
            ..Default::default()
        };

        // gpui_kit::open_window needs &mut App, so from this async context it
        // runs inside cx.update per the kit contract; it hosts the Base Root.
        let Some((window, _root_view)) = cx
            .update(|cx| {
                gpui_kit::open_window(options, cx, |window, cx| {
                    let root_view =
                        cx.new(|cx| crate::root::AppRoot::new(title.clone(), window, cx));

                    let focus_handle = root_view.focus_handle(cx);
                    window.defer(cx, move |window, cx| {
                        focus_handle.focus(window, cx);
                    });

                    root_view
                })
            })
            .ok()
        else {
            tracing::error!("failed to open window");
            return Ok::<_, anyhow::Error>(());
        };

        // Stored so tray Show and the quit path reach the window without
        // active_window() (thread-local GetActiveWindow on Windows).
        cx.update(|cx| note_root_window(window, cx));

        window.update(cx, |_, window, _| {
            window.activate_window();
            window.set_window_title(&title);
        })?;
        // App level (not inside the window update) so the new window reports
        // as active in the accessibility snapshot.
        cx.update(crate::accessibility::refresh);

        Ok::<_, anyhow::Error>(())
    })
    .detach();
}
