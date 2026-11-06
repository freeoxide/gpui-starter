use gpui_kit::{AppContext as _, TestAppContext};

use super::*;
use crate::app_state::{AppConfig, debounce_test_lock, force_save, install_for_tests};
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
    let _guard = debounce_test_lock();
    cx.update(gpui_kit::init);
    cx.update(init_floating);
    cx.update(|cx| {
        install_for_tests(
            AppConfig {
                floating_palettes: vec![persistence::PersistedPalette {
                    id: "assets".to_string(),
                    open: true,
                    surface: "window".to_string(),
                    x: 100.0,
                    y: 120.0,
                    width: 300.0,
                    height: 260.0,
                    color: None,
                    note: None,
                }],
                ..AppConfig::default()
            },
            cx,
        )
    });
    let session = cx.new(PaletteSession::new);
    assert!(session.read_with(cx, |session, _| session.is_open(PaletteId::Assets)));
    assert_eq!(
        session.read_with(cx, |session, _| session.surface(PaletteId::Assets)),
        Some(PaletteSurface::Window),
        "the persisted surface must survive the config mapping"
    );

    session.update(cx, |session, cx| session.restore_windows(cx));
    let origin = cx.read(|cx| {
        crate::ui::floating::floating_snapshot(cx)
            .into_iter()
            .find(|entry| entry.id.0.as_ref() == "floating-ui-assets")
            .map(|entry| entry.bounds.origin)
            .expect("the persisted assets window spawned")
    });
    assert_eq!(
        origin,
        point(px(100.), px(120.)),
        "restore must respawn at the persisted origin"
    );

    session.update(cx, |session, cx| session.restore_windows(cx));
    cx.read(|cx| {
        let count = crate::ui::floating::floating_snapshot(cx)
            .iter()
            .filter(|entry| entry.id.0.as_ref() == "floating-ui-assets")
            .count();
        assert_eq!(count, 1, "restore must be one-shot");
    });
    cx.update(force_save);
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

#[cfg(not(target_family = "wasm"))]
#[gpui_kit::test]
async fn open_windows_carry_a_bounds_watch_that_persists_the_origin(cx: &mut TestAppContext) {
    let _guard = debounce_test_lock();
    cx.update(gpui_kit::init);
    cx.update(init_floating);
    // slot()/update_slot() need an installed AppState; the default config
    // seeds nothing, so every entry the test reads is one it wrote.
    cx.update(|cx| install_for_tests(AppConfig::default(), cx));
    let session = cx.new(PaletteSession::new);
    session
        .update(cx, |session, cx| {
            session.toggle_window(PaletteId::Color, cx)
        })
        .unwrap();
    cx.run_until_parked();

    let (window, origin) = cx.read(|cx| {
        let entry = crate::ui::floating::floating_snapshot(cx)
            .into_iter()
            .find(|entry| entry.id == PaletteId::Color.window_id())
            .expect("the palette window spawned");
        let origin = entry.bounds.origin;
        match entry.handle {
            crate::ui::floating::FloatingHandle::Native(window) => (window, origin),
            _ => panic!("the palette window must be native on desktop"),
        }
    });
    let watched = session.read_with(cx, |session, _| {
        session.origin_watches.contains_key(&PaletteId::Color)
    });
    assert!(watched, "an open palette window must carry a bounds watch");

    cx.update(|cx| {
        persistence::update_slot(PaletteId::Color, cx, |entry| {
            entry.x = -777.;
            entry.y = -777.;
        });
    });
    // A window move fires the platform resize callback; bounds_changed is
    // its observer fan-out, exposed for exactly this kind of test.
    window
        .update(cx, |_, window, cx| {
            window.bounds_changed(cx);
        })
        .unwrap();
    cx.read(|cx| {
        let entry = persistence::slot(PaletteId::Color, cx).expect("the color slot persists");
        assert_eq!(entry.x, f32::from(origin.x));
        assert_eq!(entry.y, f32::from(origin.y));
    });

    session
        .update(cx, |session, cx| {
            session.toggle_window(PaletteId::Color, cx)
        })
        .unwrap();
    cx.run_until_parked();
    let dropped = session.read_with(cx, |session, _| {
        !session.origin_watches.contains_key(&PaletteId::Color)
    });
    assert!(dropped, "closing the window must drop its bounds watch");
    cx.update(force_save);
}
