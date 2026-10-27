// Do NOT `use super::*` here: panel_host.rs does `use gpui_kit::{prelude::*, *}`,
// which re-exports gpui's `test` proc-macro and turns a bare `#[test]` (emitted
// by `#[gpui_kit::test]`) into infinite recursion. Import only what is needed.
use gpui_kit::prelude::*;
use gpui_kit::{Context, Entity, TestAppContext, Window, div, point, px, size};

use super::super::{PaletteId, PaletteSession};
use super::PalettePanelHost;
use crate::ui::floating::init as init_floating;

struct HostRoot {
    host: Entity<PalettePanelHost>,
}

impl Render for HostRoot {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.host.clone())
    }
}

fn draw_host(
    cx: &mut TestAppContext,
    window: gpui_kit::AnyWindowHandle,
    host: &Entity<PalettePanelHost>,
) {
    cx.update(|cx| {
        window
            .update(cx, |_, window, app| {
                host.update(app, |host, host_cx| {
                    let _frame = host.render(window, host_cx);
                });
            })
            .unwrap();
    });
}

#[gpui_kit::test]
async fn zero_open_slots_render_no_panes(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init_floating);
    let session = cx.new(PaletteSession::new);
    let window = {
        let session = session.clone();
        cx.open_window(size(px(800.), px(600.)), |window, cx| {
            let host = cx.new(|cx| PalettePanelHost::new(session, window, cx));
            HostRoot { host }
        })
    };
    let host = cx.update(|cx| window.read_with(cx, |root, _| root.host.clone()).unwrap());
    draw_host(cx, window.into(), &host);
    assert!(host.read_with(cx, |host, _| host.open_panels().is_empty()));
}

#[gpui_kit::test]
async fn open_panel_slots_render_one_pane_each(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init_floating);
    let session = cx.new(PaletteSession::new);
    let window = {
        let session = session.clone();
        cx.open_window(size(px(800.), px(600.)), |window, cx| {
            let host = cx.new(|cx| PalettePanelHost::new(session, window, cx));
            HostRoot { host }
        })
    };
    let host = cx.update(|cx| window.read_with(cx, |root, _| root.host.clone()).unwrap());
    session.update(cx, |session, cx| {
        session.set_panel_open(PaletteId::Color, true, cx);
        session.set_panel_open(PaletteId::Note, true, cx);
    });
    draw_host(cx, window.into(), &host);
    assert_eq!(host.read_with(cx, |host, _| host.open_panels().len()), 2);
    session.update(cx, |session, cx| {
        session.set_panel_open(PaletteId::Color, false, cx);
    });
    draw_host(cx, window.into(), &host);
    assert_eq!(host.read_with(cx, |host, _| host.open_panels().len()), 1);
}

#[gpui_kit::test]
async fn commit_drag_snaps_a_pane_parked_near_the_left_edge(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init_floating);
    let session = cx.new(PaletteSession::new);
    let window = {
        let session = session.clone();
        cx.open_window(size(px(800.), px(600.)), |window, cx| {
            let host = cx.new(|cx| PalettePanelHost::new(session, window, cx));
            HostRoot { host }
        })
    };
    let host = cx.update(|cx| window.read_with(cx, |root, _| root.host.clone()).unwrap());
    host.update(cx, |host, _| {
        let geom = host.geoms.get_mut(&PaletteId::Assets).unwrap();
        geom.origin = point(px(10.), px(120.));
    });
    cx.update(|cx| {
        window
            .update(cx, |_, window, app| {
                host.update(app, |host, host_cx| {
                    host.commit_drag(PaletteId::Assets, window, host_cx);
                });
            })
            .unwrap();
    });
    assert_eq!(
        host.read_with(cx, |host, _| host.geoms[&PaletteId::Assets].origin),
        point(px(0.), px(120.))
    );
}
