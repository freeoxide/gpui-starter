// Do NOT `use super::*` here: views.rs does `use gpui_kit::{prelude::*, *}`,
// which re-exports gpui's `test` proc-macro and turns a bare `#[test]` (emitted
// by `#[gpui_kit::test]`) into infinite recursion. Import only what is needed.
use gpui_kit::prelude::*;
use gpui_kit::{AnyView, Context, TestAppContext, Window, div, px, size};

use super::super::{
    FloatingUiPage, PaletteId, PaletteSession, PaletteSyncState, apply_change, init,
};
use super::{emit_asset_change, emit_color_change, palette_view_factory};

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
async fn all_three_views_construct_in_a_window(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    for id in PaletteId::ALL {
        let factory = palette_view_factory(*id);
        cx.open_window(size(px(300.), px(300.)), |window, cx| PaletteHost {
            view: factory(window, cx),
        });
    }
}

#[gpui_kit::test]
async fn page_constructs_with_a_seeded_session_and_reflects_sync_state(cx: &mut TestAppContext) {
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
    cx.update(|cx| {
        window
            .update(cx, |_, window, app| {
                page.update(app, |page, page_cx| {
                    let _frame = page.render(window, page_cx);
                });
            })
            .unwrap();
    });
    cx.read(|cx| {
        let sync = cx.global::<PaletteSyncState>();
        assert_eq!(sync.color_hex, "#30A46C");
    });
}
