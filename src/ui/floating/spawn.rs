use std::rc::Rc;

use anyhow::anyhow;
use gpui_kit::{AnyView, AnyWindowHandle, App, Bounds, Pixels, Size, Window, point};

#[cfg(not(target_family = "wasm"))]
use gpui_kit::{AppContext as _, WindowBounds, WindowKind, WindowOptions};

use super::registry::{
    FloatingHandle, RegistryEntry, ViewFactory, floating_snapshot, remove_entry, set_entry,
    set_entry_level,
};
#[cfg(not(target_family = "wasm"))]
use super::shell::FloatingShell;
use super::shims;
use super::spec::{
    AnchorResolution, FloatAnchor, FloatLevel, FloatingSpec, FloatingWindowId, resolve_anchor,
    resolve_bounds, window_is_open,
};

/// Spawn a float for `spec`. Native errors: the anchor cannot be resolved or
/// window creation failed. A Window anchor whose window is mid-dispatch
/// defers the open to the end of the cycle and returns an inert registry
/// handle; the real entry lands moments later with exact anchor bounds.
#[cfg(not(target_family = "wasm"))]
pub fn spawn_floating(
    spec: FloatingSpec,
    build: impl Fn(&mut Window, &mut App) -> AnyView + 'static,
    cx: &mut App,
) -> anyhow::Result<FloatingHandle> {
    let interim_id = spec.id().clone();
    let bounds = match resolve_float_bounds_outcome(&spec, cx) {
        BoundsOutcome::Resolved(bounds) => bounds,
        BoundsOutcome::Blocked => {
            defer_past_dispatch(spec, build, cx);
            return Ok(FloatingHandle::Overlay(interim_id));
        }
        BoundsOutcome::Failed(err) => return Err(err),
    };
    open_floating_window(spec, build, bounds, cx)
}

/// Resolve-then-open with no deferral: the retry path after a dispatch
/// lease unwinds, where resolution must succeed or fail on its own merits.
#[cfg(not(target_family = "wasm"))]
fn spawn_floating_settled(
    spec: FloatingSpec,
    build: impl Fn(&mut Window, &mut App) -> AnyView + 'static,
    cx: &mut App,
) -> anyhow::Result<FloatingHandle> {
    let bounds = match resolve_float_bounds_outcome(&spec, cx) {
        BoundsOutcome::Resolved(bounds) => bounds,
        // Retries run un-leased, so a still-blocked anchor is a real failure.
        BoundsOutcome::Blocked => return Err(anyhow!("floating anchor unavailable")),
        BoundsOutcome::Failed(err) => return Err(err),
    };
    open_floating_window(spec, build, bounds, cx)
}

/// Effects flush after the anchor window's dispatch unwinds and releases its
/// update lease; the retry then resolves real anchor bounds.
#[cfg(not(target_family = "wasm"))]
fn defer_past_dispatch(
    spec: FloatingSpec,
    build: impl Fn(&mut Window, &mut App) -> AnyView + 'static,
    cx: &mut App,
) {
    cx.defer(move |cx| {
        if let Err(err) = spawn_floating_settled(spec, build, cx) {
            tracing::warn!(target: shims::LOG, error = %err, "deferred float spawn failed");
        }
    });
}

#[cfg(not(target_family = "wasm"))]
fn open_floating_window(
    spec: FloatingSpec,
    build: impl Fn(&mut Window, &mut App) -> AnyView + 'static,
    bounds: Bounds<Pixels>,
    cx: &mut App,
) -> anyhow::Result<FloatingHandle> {
    let factory: ViewFactory = Rc::new(build);
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
        // Only the linux layer-shell branch mutates the options.
        #[cfg(target_os = "linux")]
        let mut options = float_window_options(&spec, bounds);
        #[cfg(not(target_os = "linux"))]
        let options = float_window_options(&spec, bounds);
        #[cfg(target_os = "linux")]
        if _use_layer && let Some(layer) = plan.layer_options.clone() {
            // Option set from crates/gpui/examples/layer_shell.rs at
            // zed@93d7456, the pinned snapshot's reference implementation.
            options.kind = WindowKind::LayerShell(layer);
            options.window_background = gpui_kit::WindowBackgroundAppearance::Transparent;
            options.display_id = plan.display_id;
        }
        options
    };
    let opened = {
        let id = spec.id().clone();
        let title = spec.window_title();
        let chromeless = spec.is_chromeless();
        let build_view = factory.clone();
        gpui_kit::open_window(make_options(use_layer), cx, move |window, cx| {
            window.set_window_title(&title);
            let view = build_view(window, cx);
            cx.new(|cx| FloatingShell::new(id, title, view, chromeless, cx))
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
                    let chromeless = spec.is_chromeless();
                    let build_view = factory.clone();
                    let (window, _) =
                        gpui_kit::open_window(make_options(false), cx, move |window, cx| {
                            window.set_window_title(&title);
                            let view = build_view(window, cx);
                            cx.new(|cx| FloatingShell::new(id, title, view, chromeless, cx))
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
    let interim_id = spec.id().clone();
    let bounds = match resolve_overlay_bounds(&root, spec.float_anchor(), spec.size(), cx) {
        OverlayOutcome::Bounds(bounds) => bounds,
        OverlayOutcome::Blocked => {
            cx.defer(move |cx| {
                if let Err(err) = spawn_floating_settled(spec, build, cx) {
                    tracing::warn!(
                        target: "gpui_starter::ui::floating",
                        error = %err,
                        "deferred overlay float spawn failed"
                    );
                }
            });
            return Ok(FloatingHandle::Overlay(interim_id));
        }
        OverlayOutcome::Failed(err) => return Err(err),
    };
    let factory: ViewFactory = Rc::new(build);
    register_overlay(spec, factory, bounds, &root, cx)
}

/// Resolve-then-register with no deferral: the retry path after the main
/// window's dispatch lease unwinds.
#[cfg(target_family = "wasm")]
fn spawn_floating_settled(
    spec: FloatingSpec,
    build: impl Fn(&mut Window, &mut App) -> AnyView + 'static,
    cx: &mut App,
) -> anyhow::Result<FloatingHandle> {
    let root = crate::app::window::root_window(cx)
        .ok_or_else(|| anyhow!("no main window to host overlay float"))?;
    let bounds = overlay_bounds(&root, spec.float_anchor(), spec.size(), cx)?;
    let factory: ViewFactory = Rc::new(build);
    register_overlay(spec, factory, bounds, &root, cx)
}

/// Spawn a Screen-anchored float that survives the main window closing:
/// native never reads the root window; wasm hosts it as a root overlay.
pub fn spawn_screen_floating(
    spec: FloatingSpec,
    build: impl Fn(&mut Window, &mut App) -> AnyView + 'static,
    cx: &mut App,
) -> anyhow::Result<FloatingHandle> {
    if !matches!(spec.float_anchor(), FloatAnchor::Screen { .. }) {
        return Err(anyhow!("independent floats require a Screen anchor"));
    }
    spawn_floating(spec, build, cx)
}

#[cfg(target_family = "wasm")]
fn register_overlay(
    spec: FloatingSpec,
    factory: ViewFactory,
    bounds: Bounds<Pixels>,
    root: &AnyWindowHandle,
    cx: &mut App,
) -> anyhow::Result<FloatingHandle> {
    let build_view = factory.clone();
    let view = root
        .update(cx, |_, window, cx| build_view(window, cx))
        .map_err(|_| anyhow!("no main window to host overlay float"))?;
    let id = spec.id().clone();
    let chromeless = spec.is_chromeless();
    let handle = FloatingHandle::Overlay(id.clone());
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
    super::overlay_host::set_pane_chromeless(&id, chromeless, cx);
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

// Floats are widget panels: no titlebar anywhere, so no traffic lights or WM
// close/min/max. Esc (DismissFloating) and the owning manager close them.
#[cfg(not(target_family = "wasm"))]
fn float_window_options(spec: &FloatingSpec, bounds: Bounds<Pixels>) -> WindowOptions {
    let level = spec.float_level();
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: None,
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
        // Server-side decorations would draw a WM titlebar with window buttons.
        #[cfg(target_os = "linux")]
        window_decorations: Some(gpui_kit::WindowDecorations::Client),
        ..Default::default()
    }
}

#[cfg(not(target_family = "wasm"))]
enum BoundsOutcome {
    Resolved(Bounds<Pixels>),
    /// A live anchor window's dispatch holds the update lease; the same
    /// resolution succeeds once the cycle settles.
    Blocked,
    Failed(anyhow::Error),
}

#[cfg(not(target_family = "wasm"))]
fn resolve_float_bounds_outcome(spec: &FloatingSpec, cx: &mut App) -> BoundsOutcome {
    let anchor = spec.float_anchor();
    match resolve_anchor(anchor, cx) {
        AnchorResolution::Resolved((anchor_bounds, clamp_bounds)) => {
            BoundsOutcome::Resolved(resolve_bounds(
                anchor_bounds,
                clamp_bounds,
                spec.size(),
                anchor.placement(),
                anchor.offset(),
            ))
        }
        AnchorResolution::DispatchBlocked => BoundsOutcome::Blocked,
        AnchorResolution::Unavailable => {
            BoundsOutcome::Failed(anyhow!("floating anchor unavailable"))
        }
    }
}

enum OverlayOutcome {
    Bounds(Bounds<Pixels>),
    /// The main window's own dispatch holds its update lease; either the
    /// anchor window or the root blocks the bounds read.
    Blocked,
    Failed(anyhow::Error),
}

fn resolve_overlay_bounds(
    root: &AnyWindowHandle,
    anchor: &FloatAnchor,
    size: Size<Pixels>,
    cx: &mut App,
) -> OverlayOutcome {
    let (anchor_bounds, clamp_bounds) = match resolve_anchor(anchor, cx) {
        AnchorResolution::Resolved(bounds) => bounds,
        AnchorResolution::DispatchBlocked => return OverlayOutcome::Blocked,
        AnchorResolution::Unavailable => {
            return OverlayOutcome::Failed(anyhow!("floating anchor unavailable"));
        }
    };
    let screen = resolve_bounds(
        anchor_bounds,
        clamp_bounds,
        size,
        anchor.placement(),
        anchor.offset(),
    );
    let Some(root_bounds) = root
        .update(cx, |_, window, _| window.window_bounds().get_bounds())
        .ok()
    else {
        // Open + un-updatable is the dispatch lease; closed is a real failure.
        return if window_is_open(root, cx) {
            OverlayOutcome::Blocked
        } else {
            OverlayOutcome::Failed(anyhow!("no main window to host overlay float"))
        };
    };
    OverlayOutcome::Bounds(Bounds {
        origin: point(
            screen.origin.x - root_bounds.origin.x,
            screen.origin.y - root_bounds.origin.y,
        ),
        size: screen.size,
    })
}

fn overlay_bounds(
    root: &AnyWindowHandle,
    anchor: &FloatAnchor,
    size: Size<Pixels>,
    cx: &mut App,
) -> anyhow::Result<Bounds<Pixels>> {
    match resolve_overlay_bounds(root, anchor, size, cx) {
        OverlayOutcome::Bounds(bounds) => Ok(bounds),
        OverlayOutcome::Blocked => Err(anyhow!("floating anchor unavailable")),
        OverlayOutcome::Failed(err) => Err(err),
    }
}

#[cfg(test)]
#[path = "spawn.test.rs"]
mod spawn_test;
