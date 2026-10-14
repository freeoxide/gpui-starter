use std::rc::Rc;

use anyhow::anyhow;
use gpui_kit::{AnyView, AnyWindowHandle, App, Bounds, Pixels, Size, Window, point};

#[cfg(not(target_family = "wasm"))]
use gpui_kit::{AppContext as _, TitlebarOptions, WindowBounds, WindowKind, WindowOptions};

use super::registry::{
    FloatingHandle, RegistryEntry, ViewFactory, floating_snapshot, remove_entry, set_entry,
    set_entry_level,
};
#[cfg(not(target_family = "wasm"))]
use super::shell::FloatingShell;
use super::shims;
use super::spec::{
    FloatAnchor, FloatLevel, FloatingSpec, FloatingWindowId, anchor_rect, resolve_bounds,
};

/// Spawn a float for `spec`. Native errors: the anchor cannot be resolved or
/// window creation failed. On wasm the only error is that no main window has
/// been recorded yet.
#[cfg(not(target_family = "wasm"))]
pub fn spawn_floating(
    spec: FloatingSpec,
    build: impl Fn(&mut Window, &mut App) -> AnyView + 'static,
    cx: &mut App,
) -> anyhow::Result<FloatingHandle> {
    let factory: ViewFactory = Rc::new(build);
    let bounds = resolve_float_bounds(&spec, cx)?;
    let level = spec.float_level();
    #[cfg(target_os = "linux")]
    let plan = super::layer_shell::wayland::plan(&spec, cx);
    #[cfg(target_os = "linux")]
    let use_layer = plan.layer_options.is_some();
    #[cfg(not(target_os = "linux"))]
    let use_layer = false;
    if spec.layer_shell_spec().is_some() && !use_layer {
        super::layer_shell::note_ignored_once();
    }
    let make_options = |_use_layer: bool| -> WindowOptions {
        let mut options = WindowOptions {
            window_bounds: Some(WindowBounds::Windowed(bounds)),
            titlebar: Some(TitlebarOptions {
                title: Some(spec.window_title()),
                appears_transparent: true,
                traffic_light_position: None,
            }),
            focus: spec.is_focus(),
            kind: match level {
                FloatLevel::Floating => WindowKind::Floating,
                FloatLevel::AlwaysOnTop => WindowKind::PopUp,
            },
            is_resizable: spec.is_resizable() && level != FloatLevel::AlwaysOnTop,
            is_minimizable: false,
            is_movable: spec.is_movable(),
            window_min_size: spec.min_window_size(),
            app_id: Some("gpui-starter".to_string()),
            ..Default::default()
        };
        #[cfg(target_os = "linux")]
        if _use_layer && let Some(layer) = plan.layer_options.clone() {
            // Option set from crates/gpui/examples/layer_shell.rs at
            // zed@93d7456, the pinned snapshot's reference implementation.
            options.kind = WindowKind::LayerShell(layer);
            options.window_background = gpui_kit::WindowBackgroundAppearance::Transparent;
            options.titlebar = None;
            options.display_id = plan.display_id;
        }
        options
    };
    let opened = {
        let id = spec.id().clone();
        let title = spec.window_title();
        let build_view = factory.clone();
        gpui_kit::open_window(make_options(use_layer), cx, move |window, cx| {
            let view = build_view(window, cx);
            cx.new(|cx| FloatingShell::new(id, title, view, cx))
        })
    };
    let (window, surface) = match opened {
        Ok((window, _)) => {
            #[cfg(target_os = "linux")]
            let surface = if use_layer {
                super::layer_shell::state::record_available(cx);
                super::layer_shell::FloatSurface::LayerShell
            } else {
                super::layer_shell::FloatSurface::Window
            };
            #[cfg(not(target_os = "linux"))]
            let surface = super::layer_shell::FloatSurface::Window;
            (window, surface)
        }
        Err(err) => {
            #[cfg(target_os = "linux")]
            {
                if use_layer && super::layer_shell::wayland::is_layer_shell_unsupported(&err) {
                    tracing::warn!(
                        target: super::layer_shell::LOG,
                        error = %err,
                        "compositor rejected the layer surface; retrying as a plain float"
                    );
                    super::layer_shell::state::record_unsupported(&err, cx);
                    let id = spec.id().clone();
                    let title = spec.window_title();
                    let build_view = factory.clone();
                    let (window, _) =
                        gpui_kit::open_window(make_options(false), cx, move |window, cx| {
                            let view = build_view(window, cx);
                            cx.new(|cx| FloatingShell::new(id, title, view, cx))
                        })?;
                    (window, super::layer_shell::FloatSurface::Window)
                } else {
                    return Err(err);
                }
            }
            #[cfg(not(target_os = "linux"))]
            {
                return Err(err);
            }
        }
    };
    let handle = FloatingHandle::Native(window);
    if spec.is_click_through() {
        let applied = window
            .update(cx, |_, window, _| shims::set_click_through(window, true))
            .unwrap_or(false);
        if !applied {
            tracing::debug!(
                target: shims::LOG,
                "click-through not applied at spawn (no shim or unsupported handle)"
            );
        }
    }
    set_entry(
        RegistryEntry {
            spec,
            bounds,
            handle: handle.clone(),
            build: factory,
            view: None,
            surface,
        },
        cx,
    );
    Ok(handle)
}

#[cfg(target_family = "wasm")]
pub fn spawn_floating(
    spec: FloatingSpec,
    build: impl Fn(&mut Window, &mut App) -> AnyView + 'static,
    cx: &mut App,
) -> anyhow::Result<FloatingHandle> {
    if spec.layer_shell_spec().is_some() {
        super::layer_shell::note_ignored_once();
    }
    let root = crate::app::window::root_window(cx)
        .ok_or_else(|| anyhow!("no main window to host overlay float"))?;
    let factory: ViewFactory = Rc::new(build);
    let bounds = overlay_bounds(&root, spec.float_anchor(), spec.size(), cx)?;
    let build_view = factory.clone();
    let view = root
        .update(cx, |_, window, cx| build_view(window, cx))
        .map_err(|_| anyhow!("no main window to host overlay float"))?;
    let handle = FloatingHandle::Overlay(spec.id().clone());
    set_entry(
        RegistryEntry {
            spec,
            bounds,
            handle: handle.clone(),
            build: factory,
            view: Some(view),
            surface: super::layer_shell::FloatSurface::Window,
        },
        cx,
    );
    Ok(handle)
}

/// Returns true if the handle was open and is now closing/closed.
pub fn close_floating(handle: &FloatingHandle, cx: &mut App) -> bool {
    match handle {
        FloatingHandle::Native(window) => window
            .update(cx, |_, window, _| window.remove_window())
            .is_ok(),
        FloatingHandle::Overlay(id) => remove_entry(id, cx).is_some(),
    }
}

/// Runtime mouse pass-through on an open float. Returns true when a
/// platform shim acknowledged the request; false is a normal outcome
/// (overlay handle, wasm, test platform, gone window), never an error.
pub fn set_floating_click_through(handle: &FloatingHandle, enabled: bool, cx: &mut App) -> bool {
    match handle {
        FloatingHandle::Native(window) => window
            .update(cx, |_, window, _| shims::set_click_through(window, enabled))
            .unwrap_or(false),
        FloatingHandle::Overlay(_) => false,
    }
}

/// Runtime always-on-top on an open float. On success the registry entry's
/// level becomes the current level; false (never an error) on overlay
/// handles, Wayland, wasm, the test platform, or a gone window.
pub fn set_floating_always_on_top(handle: &FloatingHandle, enabled: bool, cx: &mut App) -> bool {
    let applied = match handle {
        FloatingHandle::Native(window) => window
            .update(cx, |_, window, _| shims::set_always_on_top(window, enabled))
            .unwrap_or(false),
        FloatingHandle::Overlay(_) => false,
    };
    if applied {
        let level = if enabled {
            FloatLevel::AlwaysOnTop
        } else {
            FloatLevel::Floating
        };
        set_entry_level(handle, level, cx);
    }
    applied
}

pub fn close_all_floating(cx: &mut App) -> usize {
    let handles: Vec<FloatingHandle> = floating_snapshot(cx)
        .iter()
        .map(|entry| entry.handle.clone())
        .collect();
    let count = handles.len();
    for handle in handles {
        close_floating(&handle, cx);
    }
    count
}

/// Recompute the anchor and reopen: native windows close and reopen (gpui has
/// no move API), overlay floats only update their stored bounds. Content
/// state is not carried across a native respawn; caller responsibility.
pub fn respawn_floating(id: &FloatingWindowId, cx: &mut App) -> bool {
    let Some(entry) = remove_entry(id, cx) else {
        return false;
    };
    match &entry.handle {
        FloatingHandle::Overlay(_) => {
            let mut entry = entry;
            if let Some(root) = crate::app::window::root_window(cx)
                && let Some(bounds) =
                    overlay_bounds(&root, entry.spec.float_anchor(), entry.spec.size(), cx).ok()
            {
                entry.bounds = bounds;
                set_entry(entry, cx);
                return true;
            }
            false
        }
        FloatingHandle::Native(old_window) => {
            let _ = old_window.update(cx, |_, window, _| window.remove_window());
            let factory = entry.build.clone();
            let spec = entry.spec;
            spawn_floating(spec, move |window, cx| factory(window, cx), cx).is_ok()
        }
    }
}

#[cfg(not(target_family = "wasm"))]
fn resolve_float_bounds(spec: &FloatingSpec, cx: &mut App) -> anyhow::Result<Bounds<Pixels>> {
    let anchor = spec.float_anchor();
    let (anchor_bounds, clamp_bounds) =
        anchor_rect(anchor, cx).ok_or_else(|| anyhow!("floating anchor unavailable"))?;
    Ok(resolve_bounds(
        anchor_bounds,
        clamp_bounds,
        spec.size(),
        anchor.placement(),
        anchor.offset(),
    ))
}

fn overlay_bounds(
    root: &AnyWindowHandle,
    anchor: &FloatAnchor,
    size: Size<Pixels>,
    cx: &mut App,
) -> anyhow::Result<Bounds<Pixels>> {
    let (anchor_bounds, clamp_bounds) =
        anchor_rect(anchor, cx).ok_or_else(|| anyhow!("floating anchor unavailable"))?;
    let screen = resolve_bounds(
        anchor_bounds,
        clamp_bounds,
        size,
        anchor.placement(),
        anchor.offset(),
    );
    let root_bounds = root
        .update(cx, |_, window, _| window.window_bounds().get_bounds())
        .map_err(|_| anyhow!("no main window to host overlay float"))?;
    Ok(Bounds {
        origin: point(
            screen.origin.x - root_bounds.origin.x,
            screen.origin.y - root_bounds.origin.y,
        ),
        size: screen.size,
    })
}

#[cfg(test)]
#[path = "spawn.test.rs"]
mod spawn_test;
