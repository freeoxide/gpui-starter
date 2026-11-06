// Do NOT `use super::*` here: views.rs does `use gpui_kit::{prelude::*, *}`,
// which re-exports gpui's `test` proc-macro and turns a bare `#[test]` (emitted
// by `#[gpui_kit::test]`) into infinite recursion. Import only what is needed.
use std::collections::HashSet;

use gpui_kit::component::input::InputEvent;
use gpui_kit::prelude::*;
use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{AnyView, Context, Entity, TestAppContext, Window, div, px, size};

use super::super::persistence;
use super::super::{FloatingUiPage, PaletteId, PaletteSession, apply_change, init};
use super::{StickyNoteView, emit_asset_change, emit_color_change, palette_view_factory};
use crate::app_state::{AppConfig, debounce_test_lock, force_save, install_for_tests};

#[gpui_kit::test]
async fn emit_color_change_pushes_one_palette_event(cx: &mut TestAppContext) {
    cx.update(|cx| {
        emit_color_change("#30A46C".to_string(), cx);
    });
    let events = cx.update(|cx| crate::events::drain(cx));
    assert_eq!(events.len(), 1);
    assert!(matches!(
        &events[0].kind,
        crate::events::AppEventKind::PaletteChanged { palette, value }
            if palette == "color" && value == "#30A46C"
    ));
}

#[gpui_kit::test]
async fn emit_asset_change_pushes_one_palette_event(cx: &mut TestAppContext) {
    cx.update(|cx| {
        emit_asset_change("🚀".to_string(), cx);
    });
    let events = cx.update(|cx| crate::events::drain(cx));
    assert_eq!(events.len(), 1);
    assert!(matches!(
        &events[0].kind,
        crate::events::AppEventKind::PaletteChanged { palette, value }
            if palette == "assets" && value == "🚀"
    ));
}

struct PaletteHost {
    view: AnyView,
}

impl Render for PaletteHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div().size_full().child(self.view.clone())
    }
}

#[gpui_kit::test]
async fn each_factory_yields_its_own_rendering_view(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let mut ids = Vec::new();
    for id in PaletteId::ALL {
        let factory = palette_view_factory(*id);
        cx.open_window(size(px(300.), px(300.)), |window, cx| {
            let view = factory(window, cx);
            ids.push(view.entity_id());
            PaletteHost { view }
        });
    }
    let unique: HashSet<_> = ids.into_iter().collect();
    assert_eq!(
        unique.len(),
        PaletteId::ALL.len(),
        "each palette must get its own view entity"
    );
}

#[gpui_kit::test]
async fn the_page_renders_the_seeded_sync_color_in_its_status_line(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init);
    cx.update(|cx| apply_change("color", "#30A46C", cx));
    let page = cx.update(|cx| {
        let session = cx.new(PaletteSession::new);
        cx.new(|cx| FloatingUiPage::new(session, cx))
    });
    let window = cx.open_window(size(px(400.), px(400.)), |_, _| PaletteHost {
        view: page.clone().into(),
    });
    let label = cx
        .update(|cx| {
            window
                .update(cx, |_, window, cx| {
                    window.render_frame(cx);
                    window
                        .find("floating-ui-sync-color")
                        .label()
                        .map(str::to_string)
                })
                .unwrap()
        })
        .expect("the sync status line renders");
    let expected = format!(
        "{} #30A46C",
        crate::i18n::localize("floating_ui_sync_color")
    );
    assert_eq!(label, expected);
}

#[gpui_kit::test]
async fn sticky_note_seeds_from_the_slot_and_persists_edits(cx: &mut TestAppContext) {
    let _guard = debounce_test_lock();
    cx.update(gpui_kit::init);
    cx.update(|cx| {
        install_for_tests(
            AppConfig {
                floating_palettes: vec![persistence::PersistedPalette {
                    id: "note".to_string(),
                    open: false,
                    surface: "panel".to_string(),
                    x: 40.0,
                    y: 50.0,
                    width: 260.0,
                    height: 220.0,
                    color: None,
                    note: Some("seeded".to_string()),
                }],
                ..AppConfig::default()
            },
            cx,
        )
    });
    let mut note_view = None;
    let window = cx.open_window(size(px(300.), px(300.)), |window, cx| {
        let view: Entity<StickyNoteView> =
            cx.new(|cx| StickyNoteView::new(PaletteId::Note, window, cx));
        note_view = Some(view.clone());
        PaletteHost { view: view.into() }
    });
    let note = note_view.expect("note view entity");
    cx.read(|cx| {
        assert_eq!(note.read(cx).textarea.read(cx).value(), "seeded");
    });

    window
        .update(cx, |_, window, cx| {
            note.update(cx, |note, _| {
                note.textarea.update(cx, |state, cx| {
                    state.set_value("hello".to_string(), window, cx);
                    cx.emit(InputEvent::Change);
                });
            });
        })
        .unwrap();
    cx.read(|cx| {
        let entry = persistence::slot(PaletteId::Note, cx).expect("the note slot persists");
        assert_eq!(entry.note.as_deref(), Some("hello"));
        assert_eq!(entry.x, 40.0, "an edit must not clobber the geometry");
        assert_eq!(entry.y, 50.0);
        assert_eq!(entry.width, 260.0);
        assert_eq!(entry.height, 220.0);
    });
    cx.update(force_save);
}
