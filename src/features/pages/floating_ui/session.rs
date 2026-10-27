use std::collections::BTreeMap;

#[cfg(not(target_family = "wasm"))]
use super::geometry::restore_anchor;
use super::persistence;
use super::state::PaletteId;
#[cfg(not(target_family = "wasm"))]
use super::views::palette_view_factory;
#[cfg(not(target_family = "wasm"))]
use crate::ui::floating::{FloatAnchor, FloatLevel, FloatingSpec, Placement9, spawn_floating};
use crate::ui::floating::{
    FloatingWindowId, FloatingWindowRegistry, close_floating, floating_snapshot,
};
#[cfg(target_family = "wasm")]
use anyhow::anyhow;
#[cfg(not(target_family = "wasm"))]
use gpui_kit::{App, Size, size};
use gpui_kit::{Context, Pixels, Point, Subscription, point, px};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteSurface {
    Window,
    Panel,
}

impl PaletteSurface {
    pub fn as_str(self) -> &'static str {
        match self {
            PaletteSurface::Window => "window",
            PaletteSurface::Panel => "panel",
        }
    }

    fn parse(raw: &str) -> Self {
        if raw == "window" {
            PaletteSurface::Window
        } else {
            PaletteSurface::Panel
        }
    }
}

struct SessionSlot {
    open: bool,
    surface: PaletteSurface,
    window_origin: Option<Point<Pixels>>,
}

/// Open/surface truth for the three palettes. AppRoot owns the entity; the
/// page and panel host hold handles.
pub struct PaletteSession {
    slots: BTreeMap<PaletteId, SessionSlot>,
    restored: bool,
    _subscriptions: Vec<Subscription>,
}

impl PaletteSession {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let persisted = crate::app_state::with_config(cx, |config| {
            persistence::sanitize(&config.floating_palettes)
        });
        let mut slots = BTreeMap::new();
        for id in PaletteId::ALL {
            let slot = match persisted.get(id) {
                Some(entry) => SessionSlot {
                    open: entry.open,
                    surface: PaletteSurface::parse(&entry.surface),
                    window_origin: (entry.surface == "window")
                        .then(|| point(px(entry.x), px(entry.y))),
                },
                None => SessionSlot {
                    open: false,
                    surface: PaletteSurface::Panel,
                    window_origin: None,
                },
            };
            slots.insert(*id, slot);
        }
        let registry_watch =
            cx.observe_global::<FloatingWindowRegistry>(|this, cx| this.sync_external_closes(cx));
        Self {
            slots,
            restored: false,
            _subscriptions: vec![registry_watch],
        }
    }

    pub fn is_open(&self, id: PaletteId) -> bool {
        self.slots.get(&id).is_some_and(|slot| slot.open)
    }

    pub fn surface(&self, id: PaletteId) -> Option<PaletteSurface> {
        self.slots.get(&id).map(|slot| slot.surface)
    }

    /// Native window toggle. Returns Err(reason) on wasm or spawn failure;
    /// closing an open window always returns Ok.
    pub fn toggle_window(&mut self, id: PaletteId, cx: &mut Context<Self>) -> anyhow::Result<()> {
        let window_open = self.is_open(id) && self.surface(id) == Some(PaletteSurface::Window);
        if window_open {
            self.close_window(id, cx);
            return Ok(());
        }
        let origin = self.slots.get(&id).and_then(|slot| slot.window_origin);
        self.spawn_window(id, origin, cx)
    }

    pub fn set_panel_open(&mut self, id: PaletteId, open: bool, cx: &mut Context<Self>) {
        let window_open = self.is_open(id) && self.surface(id) == Some(PaletteSurface::Window);
        if open && window_open {
            self.close_window(id, cx);
        }
        if let Some(slot) = self.slots.get_mut(&id) {
            slot.open = open;
            slot.surface = PaletteSurface::Panel;
        }
        persistence::update_slot(id, cx, |entry| {
            entry.open = open;
            entry.surface = PaletteSurface::Panel.as_str().to_string();
        });
        cx.notify();
    }

    /// Close+respawn a native window at the default anchor (position reset;
    /// the only move path on Windows, where PopUp windows cannot be dragged).
    pub fn reset_window_position(&mut self, id: PaletteId, cx: &mut Context<Self>) {
        let window_open = self.is_open(id) && self.surface(id) == Some(PaletteSurface::Window);
        if !window_open {
            return;
        }
        self.close_window(id, cx);
        let _ = self.spawn_window(id, None, cx);
    }

    /// Boot restore of persisted `open && Window` slots; runs once.
    pub(crate) fn restore_windows(&mut self, cx: &mut Context<Self>) {
        if self.restored {
            return;
        }
        self.restored = true;
        #[cfg(not(target_family = "wasm"))]
        {
            let pending: Vec<(PaletteId, Option<Point<Pixels>>)> = PaletteId::ALL
                .iter()
                .copied()
                .filter(|id| self.is_open(*id) && self.surface(*id) == Some(PaletteSurface::Window))
                .map(|id| (id, self.slots.get(&id).and_then(|slot| slot.window_origin)))
                .collect();
            for (id, origin) in pending {
                let _ = self.spawn_window(id, origin, cx);
            }
        }
        #[cfg(target_family = "wasm")]
        let _ = cx;
    }

    fn close_window(&mut self, id: PaletteId, cx: &mut Context<Self>) {
        let window_id = id.window_id();
        if let Some(entry) = floating_snapshot(cx)
            .into_iter()
            .find(|e| e.id == window_id)
        {
            close_floating(&entry.handle, cx);
        }
        if let Some(slot) = self.slots.get_mut(&id) {
            slot.open = false;
        }
        persistence::update_slot(id, cx, |entry| entry.open = false);
        cx.notify();
    }

    fn spawn_window(
        &mut self,
        id: PaletteId,
        persisted_origin: Option<Point<Pixels>>,
        cx: &mut Context<Self>,
    ) -> anyhow::Result<()> {
        #[cfg(target_family = "wasm")]
        {
            let _ = (id, persisted_origin, cx);
            return Err(anyhow!("native palette windows unavailable on wasm"));
        }
        #[cfg(not(target_family = "wasm"))]
        {
            let size = spawn_size(id, cx);
            let anchor = persisted_origin
                .and_then(|origin| {
                    cx.primary_display()
                        .map(|display| restore_anchor(origin, display.visible_bounds()))
                })
                .or_else(|| root_window_anchor(cx))
                .unwrap_or_else(default_screen_anchor);
            let spec = FloatingSpec::new(id.window_id(), size)
                .title(id.localized_title())
                .level(FloatLevel::AlwaysOnTop)
                .anchor(anchor)
                .focus(false)
                .resizable(false)
                .movable(true)
                .min_size(size);
            let factory = palette_view_factory(id);
            spawn_floating(spec, move |window, cx| factory(window, cx), cx)?;
            let resolved = floating_snapshot(cx)
                .into_iter()
                .find(|e| e.id == id.window_id())
                .map(|e| e.bounds.origin);
            if let Some(slot) = self.slots.get_mut(&id) {
                slot.open = true;
                slot.surface = PaletteSurface::Window;
                if let Some(origin) = resolved {
                    slot.window_origin = Some(origin);
                }
            }
            persistence::update_slot(id, cx, |entry| {
                entry.open = true;
                entry.surface = PaletteSurface::Window.as_str().to_string();
                if let Some(origin) = resolved {
                    entry.x = f32::from(origin.x);
                    entry.y = f32::from(origin.y);
                }
            });
            cx.notify();
            Ok(())
        }
    }

    fn sync_external_closes(&mut self, cx: &mut Context<Self>) {
        let open_ids: Vec<FloatingWindowId> = floating_snapshot(cx)
            .iter()
            .map(|entry| entry.id.clone())
            .collect();
        let mut changed = false;
        for id in PaletteId::ALL {
            let Some(slot) = self.slots.get_mut(id) else {
                continue;
            };
            if slot.open
                && slot.surface == PaletteSurface::Window
                && !open_ids.contains(&id.window_id())
            {
                slot.open = false;
                persistence::update_slot(*id, cx, |entry| entry.open = false);
                changed = true;
            }
        }
        if changed {
            cx.notify();
        }
    }
}

#[cfg(not(target_family = "wasm"))]
fn spawn_size(id: PaletteId, cx: &gpui_kit::App) -> Size<Pixels> {
    persistence::slot(id, cx)
        .map(|entry| size(px(entry.width), px(entry.height)))
        .unwrap_or_else(|| id.default_size())
}

#[cfg(not(target_family = "wasm"))]
fn root_window_anchor(cx: &App) -> Option<FloatAnchor> {
    crate::app::window::root_window(cx).map(|window| FloatAnchor::Window {
        window,
        placement: Placement9::TopRight,
        offset: point(px(16.), px(16.)),
    })
}

#[cfg(not(target_family = "wasm"))]
fn default_screen_anchor() -> FloatAnchor {
    FloatAnchor::Screen {
        display: None,
        placement: Placement9::TopRight,
        offset: point(px(16.), px(16.)),
    }
}

#[cfg(test)]
#[path = "session.test.rs"]
mod session_test;
