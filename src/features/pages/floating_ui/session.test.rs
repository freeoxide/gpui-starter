use gpui_kit::{AppContext as _, TestAppContext};

use super::*;
use crate::ui::floating::init as init_floating;

#[gpui_kit::test]
async fn toggle_window_spawns_and_closes_the_palette_window(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init_floating);
    let session = cx.new(PaletteSession::new);
    session
        .update(cx, |session, cx| {
            session.toggle_window(PaletteId::Color, cx)
        })
        .unwrap();
    cx.read(|cx| {
        assert!(
            crate::ui::floating::floating_snapshot(cx)
                .iter()
                .any(|entry| entry.id.0.as_ref() == "floating-ui-color")
        );
    });
    assert!(session.read_with(cx, |session, _| {
        session.is_open(PaletteId::Color)
            && session.surface(PaletteId::Color) == Some(PaletteSurface::Window)
    }));

    session
        .update(cx, |session, cx| {
            session.toggle_window(PaletteId::Color, cx)
        })
        .unwrap();
    cx.read(|cx| {
        assert!(
            !crate::ui::floating::floating_snapshot(cx)
                .iter()
                .any(|entry| entry.id.0.as_ref() == "floating-ui-color")
        );
    });
    assert!(!session.read_with(cx, |session, _| session.is_open(PaletteId::Color)));
}

#[gpui_kit::test]
async fn set_panel_open_closes_an_open_window_first(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init_floating);
    let session = cx.new(PaletteSession::new);
    session
        .update(cx, |session, cx| {
            session.toggle_window(PaletteId::Color, cx)
        })
        .unwrap();
    session.update(cx, |session, cx| {
        session.set_panel_open(PaletteId::Color, true, cx)
    });
    cx.read(|cx| {
        assert!(crate::ui::floating::floating_snapshot(cx).is_empty());
    });
    assert!(session.read_with(cx, |session, _| {
        session.is_open(PaletteId::Color)
            && session.surface(PaletteId::Color) == Some(PaletteSurface::Panel)
    }));
}

#[gpui_kit::test]
async fn external_close_drains_into_the_slot_after_an_update_boundary(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init_floating);
    let session = cx.new(PaletteSession::new);
    session
        .update(cx, |session, cx| session.toggle_window(PaletteId::Note, cx))
        .unwrap();
    cx.update(|cx| {
        let entry = crate::ui::floating::floating_snapshot(cx)
            .into_iter()
            .find(|entry| entry.id.0.as_ref() == "floating-ui-note")
            .expect("note window registered");
        crate::ui::floating::close_floating(&entry.handle, cx);
    });
    cx.update(|_| {});
    assert!(!session.read_with(cx, |session, _| session.is_open(PaletteId::Note)));
}

#[gpui_kit::test]
async fn restore_windows_spawns_persisted_window_slots_once(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init_floating);
    let session = cx.new(|cx| {
        let mut session = PaletteSession::new(cx);
        let slot = session.slots.get_mut(&PaletteId::Assets).unwrap();
        slot.open = true;
        slot.surface = PaletteSurface::Window;
        session
    });
    session.update(cx, |session, cx| session.restore_windows(cx));
    cx.read(|cx| {
        assert!(
            crate::ui::floating::floating_snapshot(cx)
                .iter()
                .any(|entry| entry.id.0.as_ref() == "floating-ui-assets")
        );
    });
    session.update(cx, |session, cx| session.restore_windows(cx));
    cx.read(|cx| {
        let count = crate::ui::floating::floating_snapshot(cx)
            .iter()
            .filter(|entry| entry.id.0.as_ref() == "floating-ui-assets")
            .count();
        assert_eq!(count, 1, "restore must be one-shot");
    });
}

#[gpui_kit::test]
async fn reset_window_position_respawns_the_window_open(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init_floating);
    let session = cx.new(PaletteSession::new);
    session
        .update(cx, |session, cx| {
            session.toggle_window(PaletteId::Color, cx)
        })
        .unwrap();
    session.update(cx, |session, cx| {
        session.reset_window_position(PaletteId::Color, cx)
    });
    cx.read(|cx| {
        assert!(
            crate::ui::floating::floating_snapshot(cx)
                .iter()
                .any(|entry| entry.id.0.as_ref() == "floating-ui-color")
        );
    });
    assert!(session.read_with(cx, |session, _| session.is_open(PaletteId::Color)));
}
