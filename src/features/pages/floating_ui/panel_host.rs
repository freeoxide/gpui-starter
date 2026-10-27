use std::collections::BTreeMap;

use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};
use gpui_kit::{prelude::*, *};

use crate::accessibility::A11yExt as _;

use super::geometry::{clamp_origin, snap_origin};
use super::persistence;
use super::session::{PaletteSession, PaletteSurface};
use super::state::PaletteId;
use super::views::palette_view_factory;

struct PanelGeom {
    origin: Point<Pixels>,
    size: Size<Pixels>,
}

struct ActiveDrag {
    id: PaletteId,
    last_cursor: Point<Pixels>,
}

struct PanelDrag {
    id: PaletteId,
}

struct DragGhost;

impl Render for DragGhost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// In-window draggable panel layer; AppRoot renders it as the last child.
pub struct PalettePanelHost {
    session: Entity<PaletteSession>,
    geoms: BTreeMap<PaletteId, PanelGeom>,
    content: BTreeMap<PaletteId, AnyView>,
    drag: Option<ActiveDrag>,
    rendered: Vec<PaletteId>,
    _subscriptions: Vec<Subscription>,
}

impl PalettePanelHost {
    pub fn new(
        session: Entity<PaletteSession>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut geoms = BTreeMap::new();
        let mut content = BTreeMap::new();
        for id in PaletteId::ALL {
            let geom = match persistence::slot(*id, cx) {
                Some(entry) => PanelGeom {
                    origin: point(px(entry.x), px(entry.y)),
                    size: size(px(entry.width), px(entry.height)),
                },
                None => PanelGeom {
                    origin: point(px(16.), px(16.)),
                    size: id.default_size(),
                },
            };
            geoms.insert(*id, geom);
            let factory = palette_view_factory(*id);
            content.insert(*id, factory(window, cx));
        }
        let session_watch = cx.observe(&session, |_, _, cx| {
            cx.notify();
        });
        Self {
            session,
            geoms,
            content,
            drag: None,
            rendered: Vec::new(),
            _subscriptions: vec![session_watch],
        }
    }

    /// Panels rendered by the last frame; the pane count the tests assert on.
    pub fn open_panels(&self) -> &[PaletteId] {
        &self.rendered
    }
}

impl Render for PalettePanelHost {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let open: Vec<PaletteId> = {
            let session = self.session.read(cx);
            PaletteId::ALL
                .iter()
                .copied()
                .filter(|id| {
                    session.is_open(*id) && session.surface(*id) == Some(PaletteSurface::Panel)
                })
                .collect()
        };
        self.rendered = open.clone();
        let panes: Vec<AnyElement> = open
            .into_iter()
            .map(|id| self.render_pane(id, cx))
            .collect();
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .children(panes)
    }
}

impl PalettePanelHost {
    fn render_pane(&mut self, id: PaletteId, cx: &mut Context<Self>) -> AnyElement {
        let Some(geom) = self.geoms.get(&id) else {
            return div().into_any_element();
        };
        let origin = geom.origin;
        let size = geom.size;
        let slug = id.slug();
        let title = id.localized_title();
        let session = self.session.clone();
        let content = self.content.get(&id).cloned();
        let header = h_flex()
            .id(SharedString::from(format!("palette-panel-header-{slug}")))
            .w_full()
            .h_8()
            .px_2()
            .gap_2()
            .items_center()
            .flex_shrink_0()
            .border_b_1()
            .border_color(cx.theme().border)
            .cursor_grab()
            .on_drag(PanelDrag { id }, |_, _, _, cx| cx.new(|_| DragGhost))
            .on_drag_move(
                cx.listener(|this, ev: &DragMoveEvent<PanelDrag>, window, cx| {
                    let pos = ev.event.position;
                    let Some(active) = this.drag.as_mut() else {
                        this.drag = Some(ActiveDrag {
                            id: ev.drag(cx).id,
                            last_cursor: pos,
                        });
                        return;
                    };
                    let delta = pos - active.last_cursor;
                    let Some(geom) = this.geoms.get_mut(&active.id) else {
                        return;
                    };
                    geom.origin =
                        clamp_origin(geom.origin + delta, geom.size, window.viewport_size());
                    active.last_cursor = pos;
                    cx.notify();
                }),
            )
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(title),
            )
            .child(
                Button::new(format!("palette-panel-close-{slug}"))
                    .ghost()
                    .icon(Icon::new(IconName::Close))
                    .accessibility_label(crate::i18n::localize("floating_ui_close"))
                    .on_click(move |_, _, cx| {
                        session.update(cx, |session, cx| session.set_panel_open(id, false, cx));
                    }),
            );
        v_flex()
            .id(SharedString::from(format!("palette-panel-{slug}")))
            .absolute()
            .top(origin.y)
            .left(origin.x)
            .w(size.width)
            .h(size.height)
            .a11y(Role::Pane, id.localized_title())
            .bg(cx.theme().popover)
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .shadow_md()
            .on_drop::<PanelDrag>(cx.listener(|this, _: &PanelDrag, window, cx| {
                let Some(active) = this.drag.take() else {
                    return;
                };
                this.commit_drag(active.id, window, cx);
            }))
            .child(header)
            .when_some(content, |this, content| {
                this.child(div().flex_1().min_h_0().overflow_hidden().child(content))
            })
            .into_any_element()
    }

    /// Factored drag-release path: snap, persist geometry, redraw.
    fn commit_drag(&mut self, id: PaletteId, window: &mut Window, cx: &mut Context<Self>) {
        let Some(geom) = self.geoms.get_mut(&id) else {
            return;
        };
        geom.origin = snap_origin(geom.origin, geom.size, window.viewport_size());
        let origin = geom.origin;
        let size = geom.size;
        persistence::update_slot(id, cx, |entry| {
            entry.x = f32::from(origin.x);
            entry.y = f32::from(origin.y);
            entry.width = f32::from(size.width);
            entry.height = f32::from(size.height);
        });
        cx.notify();
    }
}

#[cfg(test)]
#[path = "panel_host.test.rs"]
mod panel_host_test;
