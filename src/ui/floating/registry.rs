use std::collections::BTreeMap;
use std::rc::Rc;

use gpui_kit::{AnyView, AnyWindowHandle, App, Bounds, Global, Pixels, SharedString, Window};

use super::spec::{FloatLevel, FloatingSpec, FloatingWindowId};

pub(crate) type ViewFactory = Rc<dyn Fn(&mut Window, &mut App) -> AnyView>;

pub(crate) struct RegistryEntry {
    pub(crate) spec: FloatingSpec,
    pub(crate) bounds: Bounds<Pixels>,
    pub(crate) handle: FloatingHandle,
    pub(crate) build: ViewFactory,
    pub(crate) view: Option<AnyView>,
}

/// How an open float is reached: an OS window handle on native platforms, or
/// the registry id of an in-window overlay pane on wasm.
#[derive(Clone, Debug)]
pub enum FloatingHandle {
    Native(AnyWindowHandle),
    Overlay(FloatingWindowId),
}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub struct FloatingWindowEntry {
    pub id: FloatingWindowId,
    pub title: SharedString,
    pub level: FloatLevel,
    /// Screen-space bounds (for overlay floats: main-window space).
    pub bounds: Bounds<Pixels>,
    pub handle: FloatingHandle,
}

pub(crate) struct OverlayPane {
    pub(crate) id: FloatingWindowId,
    pub(crate) title: SharedString,
    pub(crate) bounds: Bounds<Pixels>,
    pub(crate) view: AnyView,
}

#[derive(Default)]
pub struct FloatingWindowRegistry {
    entries: BTreeMap<FloatingWindowId, RegistryEntry>,
}

impl Global for FloatingWindowRegistry {}

pub(crate) fn set_entry(entry: RegistryEntry, cx: &mut App) {
    let id = entry.spec.id().clone();
    cx.default_global::<FloatingWindowRegistry>()
        .entries
        .insert(id, entry);
}

pub(crate) fn remove_entry(id: &FloatingWindowId, cx: &mut App) -> Option<RegistryEntry> {
    cx.default_global::<FloatingWindowRegistry>()
        .entries
        .remove(id)
}

pub(crate) fn set_entry_level(handle: &FloatingHandle, level: FloatLevel, cx: &mut App) {
    cx.default_global::<FloatingWindowRegistry>()
        .entries
        .iter_mut()
        .for_each(|(_, entry)| {
            let same = match (&entry.handle, handle) {
                (FloatingHandle::Native(a), FloatingHandle::Native(b)) => {
                    a.window_id() == b.window_id()
                }
                (FloatingHandle::Overlay(a), FloatingHandle::Overlay(b)) => a == b,
                _ => false,
            };
            if same {
                entry.spec.set_level(level);
            }
        });
}

pub(crate) fn overlay_panes(cx: &App) -> Vec<OverlayPane> {
    cx.try_global::<FloatingWindowRegistry>()
        .map(|registry| {
            registry
                .entries
                .values()
                .filter_map(|entry| match &entry.handle {
                    FloatingHandle::Overlay(_) => Some(OverlayPane {
                        id: entry.spec.id().clone(),
                        title: entry.spec.window_title(),
                        bounds: entry.bounds,
                        view: entry.view.clone()?,
                    }),
                    FloatingHandle::Native(_) => None,
                })
                .collect()
        })
        .unwrap_or_default()
}

pub fn floating_snapshot(cx: &App) -> Vec<FloatingWindowEntry> {
    cx.try_global::<FloatingWindowRegistry>()
        .map(|registry| {
            registry
                .entries
                .values()
                .map(|entry| FloatingWindowEntry {
                    id: entry.spec.id().clone(),
                    title: entry.spec.window_title(),
                    level: entry.spec.float_level(),
                    bounds: entry.bounds,
                    handle: entry.handle.clone(),
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn install_close_tracker(cx: &mut App) {
    cx.on_window_closed(|cx, window_id| {
        let stale: Vec<FloatingWindowId> = cx
            .default_global::<FloatingWindowRegistry>()
            .entries
            .iter()
            .filter(|(_, entry)| match &entry.handle {
                FloatingHandle::Native(handle) => handle.window_id() == window_id,
                FloatingHandle::Overlay(_) => false,
            })
            .map(|(id, _)| id.clone())
            .collect();
        if stale.is_empty() {
            return;
        }
        let registry = cx.default_global::<FloatingWindowRegistry>();
        for id in &stale {
            registry.entries.remove(id);
        }
    })
    .detach();
}

#[cfg(test)]
#[path = "registry.test.rs"]
mod registry_test;
