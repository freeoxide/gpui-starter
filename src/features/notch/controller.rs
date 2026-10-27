//! Notch floater lifecycle: open/close/toggle routing, capability row, and
//! close-state safety nets.

use gpui_kit::{App, Global};

#[cfg(not(target_os = "macos"))]
use crate::ui::floating::FloatingHandle;
#[cfg(target_os = "macos")]
use gpui_kit::AnyWindowHandle;

use super::CAPABILITY_NOTCH_FLOATER;

#[derive(Clone)]
pub(crate) enum NotchHandle {
    #[cfg(target_os = "macos")]
    Native(AnyWindowHandle),
    #[cfg(not(target_os = "macos"))]
    Floating(FloatingHandle),
}

#[derive(Default)]
pub(crate) struct NotchFloaterWindow(Option<NotchHandle>);

impl Global for NotchFloaterWindow {}

/// Whether the notch floater window is currently open.
pub fn is_open(cx: &App) -> bool {
    cx.try_global::<NotchFloaterWindow>()
        .is_some_and(|g| g.0.is_some())
}

#[cfg(target_os = "macos")]
fn open(cx: &mut App) -> anyhow::Result<()> {
    use gpui_kit::AppContext as _;
    use gpui_kit::{
        Bounds, WindowBackgroundAppearance, WindowBounds, WindowKind, WindowOptions, point, px,
        size,
    };

    let info = crate::ui::floating::probe(cx);
    let screen = cx
        .primary_display()
        .map(|display| display.bounds())
        .unwrap_or(Bounds::new(
            point(px(0.), px(0.)),
            size(px(1512.), px(982.)),
        ));
    let safe_top = info.notch.map(|n| n.safe_area.top).or(info.menu_bar);
    let layout =
        super::geometry::notch_layout(screen, safe_top, info.notch.and_then(|n| n.camera_housing));
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(layout.collapsed)),
        titlebar: None,
        kind: WindowKind::PopUp,
        focus: false,
        show: true,
        is_resizable: false,
        is_minimizable: false,
        is_movable: false,
        window_background: WindowBackgroundAppearance::Transparent,
        ..Default::default()
    };
    let (handle, _) = gpui_kit::open_window(options, cx, |_, cx| {
        cx.new(|cx| super::view::NotchFloaterView::new(layout, true, cx))
    })?;
    let pinned = handle
        .update(cx, |_, window, _| {
            window.set_window_title(&crate::i18n::localize("notch_window_title"));
            super::macos::apply_notch_policy(window, &layout)
        })
        .unwrap_or(false);
    if !pinned {
        tracing::debug!(target: super::LOG, "notch policy not applied (no native window handle)");
    }
    cx.set_global(NotchFloaterWindow(Some(NotchHandle::Native(handle))));
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn open(cx: &mut App) -> anyhow::Result<()> {
    use gpui_kit::AppContext as _;
    use gpui_kit::{point, px, size};

    let demo_size = size(px(360.), px(140.));
    let layout = super::geometry::notch_layout(
        gpui_kit::Bounds::new(point(px(0.), px(0.)), demo_size),
        None,
        None,
    );
    let spec = crate::ui::floating::FloatingSpec::new(
        crate::ui::floating::FloatingWindowId("notch-floater-demo".into()),
        demo_size,
    )
    .title(crate::i18n::localize("notch_window_title"))
    .level(crate::ui::floating::FloatLevel::Floating)
    .anchor(crate::ui::floating::FloatAnchor::Screen {
        display: None,
        placement: crate::ui::floating::Placement9::Top,
        offset: point(px(0.), px(8.)),
    })
    .focus(false);
    let handle = crate::ui::floating::spawn_floating(
        spec,
        move |_, cx| {
            cx.new(|cx| super::view::NotchFloaterView::new(layout, false, cx))
                .into()
        },
        cx,
    )?;
    cx.set_global(NotchFloaterWindow(Some(NotchHandle::Floating(handle))));
    Ok(())
}

#[cfg(target_os = "macos")]
pub(crate) fn close(cx: &mut App) {
    let open = cx
        .try_global::<NotchFloaterWindow>()
        .and_then(|g| g.0.clone());
    if let Some(NotchHandle::Native(handle)) = open {
        let _ = handle.update(cx, |_, window, _| window.remove_window());
    }
    cx.set_global(NotchFloaterWindow(None));
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn close(cx: &mut App) {
    let open = cx
        .try_global::<NotchFloaterWindow>()
        .and_then(|g| g.0.clone());
    if let Some(NotchHandle::Floating(handle)) = open {
        let _ = crate::ui::floating::close_floating(&handle, cx);
    }
    cx.set_global(NotchFloaterWindow(None));
}

#[cfg(target_os = "macos")]
fn install_close_tracker(cx: &mut App) {
    cx.on_window_closed(|cx, window_id| {
        let open = cx
            .try_global::<NotchFloaterWindow>()
            .and_then(|g| g.0.clone());
        if matches!(open, Some(NotchHandle::Native(h)) if h.window_id() == window_id) {
            cx.set_global(NotchFloaterWindow(None));
        }
    })
    .detach();
}

#[cfg(not(target_os = "macos"))]
fn install_float_registry_watch(cx: &mut App) {
    cx.observe_global::<crate::ui::floating::FloatingWindowRegistry>(|cx| {
        let ours = cx
            .try_global::<NotchFloaterWindow>()
            .and_then(|g| g.0.clone());
        if matches!(ours, Some(NotchHandle::Floating(_)))
            && !crate::ui::floating::floating_snapshot(cx)
                .iter()
                .any(|entry| entry.id.0 == "notch-floater-demo")
        {
            cx.set_global(NotchFloaterWindow(None));
        }
    })
    .detach();
}

/// Opens the notch floater if closed, closes it if open, and returns the
/// resulting state. Never panics: on failure the capability is written
/// degraded with the error and `false` is returned.
pub fn toggle(cx: &mut App) -> bool {
    if is_open(cx) {
        close(cx);
        false
    } else {
        match open(cx) {
            Ok(()) => true,
            Err(err) => {
                tracing::warn!(target: super::LOG, error = %err, "notch floater open failed");
                crate::capabilities::set(
                    CAPABILITY_NOTCH_FLOATER,
                    crate::capabilities::CapabilityStatus::degraded(err),
                    cx,
                );
                false
            }
        }
    }
}

/// Registers the `notch_floater` capability and the platform close safety
/// net. Call once from `app::init`, after `crate::ui::floating::init(cx)`.
pub fn init(cx: &mut App) {
    #[cfg(target_os = "macos")]
    {
        crate::capabilities::set(
            CAPABILITY_NOTCH_FLOATER,
            crate::capabilities::CapabilityStatus::supported_enabled(),
            cx,
        );
        install_close_tracker(cx);
    }
    #[cfg(all(not(target_os = "macos"), not(target_family = "wasm")))]
    crate::capabilities::set(
        CAPABILITY_NOTCH_FLOATER,
        crate::capabilities::CapabilityStatus::degraded(
            "notch APIs are macOS-only; showing inset-anchored demo window",
        ),
        cx,
    );
    #[cfg(target_family = "wasm")]
    crate::capabilities::set(
        CAPABILITY_NOTCH_FLOATER,
        crate::capabilities::CapabilityStatus::degraded(
            "native notch unavailable on wasm; showing in-window overlay demo",
        ),
        cx,
    );
    #[cfg(not(target_os = "macos"))]
    install_float_registry_watch(cx);
}

#[cfg(test)]
#[path = "controller.test.rs"]
mod controller_test;
