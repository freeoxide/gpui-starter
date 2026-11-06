// No `use super::*`: gpui's `test` proc-macro glob would make bare `#[test]`
// recurse infinitely. Import only what we need.
use std::collections::BTreeMap;

use gpui_kit::prelude::*;
use gpui_kit::{Context, Entity, TestAppContext, Window, div, point, px, size};

use super::super::persistence;
use super::super::{PaletteId, PaletteSession};
use super::{ActiveDrag, PalettePanelHost, PanelGeom, drag_step};
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
async fn reopening_a_note_panel_reseeds_its_content_from_persistence(cx: &mut TestAppContext) {
    let _guard = crate::app_state::debounce_test_lock();
    cx.update(gpui_kit::init);
    cx.update(init_floating);
    cx.update(|cx| {
        crate::app_state::install_for_tests(
            crate::app_state::AppConfig {
                floating_palettes: vec![persistence::PersistedPalette {
                    id: "note".to_string(),
                    open: false,
                    surface: "panel".to_string(),
                    x: 40.0,
                    y: 50.0,
                    width: 260.0,
                    height: 220.0,
                    color: None,
                    note: Some("first".to_string()),
                }],
                ..crate::app_state::AppConfig::default()
            },
            cx,
        )
    });
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
        session.set_panel_open(PaletteId::Note, true, cx)
    });
    draw_host(cx, window.into(), &host);
    let first_view = host.read_with(cx, |host, _| host.content[&PaletteId::Note].clone());

    // The window surface persists an edit the panel's live view never sees.
    cx.update(|cx| {
        persistence::update_slot(PaletteId::Note, cx, |entry| {
            entry.note = Some("second".to_string());
        })
    });

    session.update(cx, |session, cx| {
        session.set_panel_open(PaletteId::Note, false, cx)
    });
    draw_host(cx, window.into(), &host);
    session.update(cx, |session, cx| {
        session.set_panel_open(PaletteId::Note, true, cx)
    });
    draw_host(cx, window.into(), &host);

    let reopened = host.read_with(cx, |host, _| host.content[&PaletteId::Note].clone());
    assert_ne!(
        reopened.entity_id(),
        first_view.entity_id(),
        "a reopen must recreate the panel's content view"
    );
    let note: Entity<super::super::views::StickyNoteView> = reopened
        .downcast()
        .expect("the note panel hosts StickyNoteView");
    cx.read(|cx| {
        assert_eq!(
            note.read(cx).textarea.read(cx).value(),
            "second",
            "the reopened panel must show the persisted note"
        );
    });
    cx.update(crate::app_state::force_save);
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

#[test]
fn drag_step_accumulates_cursor_deltas_and_clamps_to_the_viewport() {
    let viewport = size(px(800.), px(600.));
    let mut geoms: BTreeMap<PaletteId, PanelGeom> = std::iter::once((
        PaletteId::Assets,
        PanelGeom {
            origin: point(px(10.), px(10.)),
            size: size(px(100.), px(80.)),
        },
    ))
    .collect();
    let mut drag = None;

    assert!(
        !drag_step(
            &mut drag,
            PaletteId::Assets,
            point(px(5.), px(5.)),
            &mut geoms,
            viewport
        ),
        "the first move only seeds the drag"
    );
    assert_eq!(geoms[&PaletteId::Assets].origin, point(px(10.), px(10.)));

    assert!(drag_step(
        &mut drag,
        PaletteId::Assets,
        point(px(45.), px(25.)),
        &mut geoms,
        viewport
    ));
    assert_eq!(
        geoms[&PaletteId::Assets].origin,
        point(px(50.), px(30.)),
        "origin moves by the cursor delta, not to the cursor"
    );

    assert!(drag_step(
        &mut drag,
        PaletteId::Assets,
        point(px(30.), px(20.)),
        &mut geoms,
        viewport
    ));
    assert_eq!(
        geoms[&PaletteId::Assets].origin,
        point(px(35.), px(25.)),
        "deltas accumulate across moves in both directions"
    );

    assert!(drag_step(
        &mut drag,
        PaletteId::Assets,
        point(px(9000.), px(9000.)),
        &mut geoms,
        viewport
    ));
    assert_eq!(
        geoms[&PaletteId::Assets].origin,
        point(px(700.), px(520.)),
        "a jump past the edge clamps inside the viewport"
    );
}

#[test]
fn drag_step_replaces_a_stale_drag_of_a_different_panel() {
    let viewport = size(px(800.), px(600.));
    let mut geoms = BTreeMap::from([
        (
            PaletteId::Color,
            PanelGeom {
                origin: point(px(0.), px(0.)),
                size: size(px(100.), px(80.)),
            },
        ),
        (
            PaletteId::Assets,
            PanelGeom {
                origin: point(px(10.), px(10.)),
                size: size(px(100.), px(80.)),
            },
        ),
    ]);
    // A mouse-up that missed every pane left the Assets drag behind.
    let mut drag = Some(ActiveDrag {
        id: PaletteId::Assets,
        last_cursor: point(px(5.), px(5.)),
    });

    assert!(
        !drag_step(
            &mut drag,
            PaletteId::Color,
            point(px(50.), px(50.)),
            &mut geoms,
            viewport
        ),
        "a mismatched drag id reseeds instead of moving the stale pane"
    );
    assert_eq!(geoms[&PaletteId::Assets].origin, point(px(10.), px(10.)));
    assert_eq!(geoms[&PaletteId::Color].origin, point(px(0.), px(0.)));

    assert!(drag_step(
        &mut drag,
        PaletteId::Color,
        point(px(70.), px(60.)),
        &mut geoms,
        viewport
    ));
    assert_eq!(
        geoms[&PaletteId::Color].origin,
        point(px(20.), px(10.)),
        "deltas track the reseeded cursor, not the stale one"
    );
    assert_eq!(geoms[&PaletteId::Assets].origin, point(px(10.), px(10.)));
}
