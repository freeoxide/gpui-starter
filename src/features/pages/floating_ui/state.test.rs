use gpui_kit::{TestAppContext, rgba};

use super::*;

#[test]
fn slug_round_trip_is_exhaustive() {
    assert_eq!(PaletteId::ALL.len(), 3);
    for id in PaletteId::ALL {
        assert_eq!(PaletteId::from_slug(id.slug()), Some(*id));
    }
    assert_eq!(PaletteId::from_slug("bogus"), None);
}

#[test]
fn window_id_carries_the_plan_02_prefix() {
    assert_eq!(PaletteId::Color.window_id().0.as_ref(), "floating-ui-color");
    assert_eq!(
        PaletteId::Assets.window_id().0.as_ref(),
        "floating-ui-assets"
    );
    assert_eq!(PaletteId::Note.window_id().0.as_ref(), "floating-ui-note");
}

#[test]
fn hex_round_trip_covers_six_and_eight_digit_forms() {
    let color = rgba(0x30A46CFF);
    let hex = rgba_to_hex(color);
    assert_eq!(hex, "#30A46C");
    assert_eq!(hex_to_rgba(&hex), Some(color));
    assert_eq!(hex_to_rgba("#30A46C80"), Some(rgba(0x30A46C80)));
}

#[test]
fn hex_to_rgba_rejects_malformed_input() {
    for bad in [
        "", "30A46C", "#30A4", "#30A46", "#30A46CC", "#GGGGGG", "#30A46G",
    ] {
        assert_eq!(hex_to_rgba(bad), None, "{bad} must not parse");
    }
}

#[test]
fn presets_match_the_documented_counts() {
    assert_eq!(COLOR_PRESETS.len(), 12);
    assert_eq!(ASSET_PRESETS.len(), 24);
}

#[gpui_kit::test]
async fn apply_change_writes_the_color_into_the_global(cx: &mut TestAppContext) {
    cx.update(|cx| {
        cx.set_global(PaletteSyncState::default());
    });
    cx.update(|cx| apply_change("color", "#FF0000", cx));
    cx.read(|cx| {
        assert_eq!(cx.global::<PaletteSyncState>().color_hex, "#FF0000");
    });
}

#[gpui_kit::test]
async fn apply_change_writes_the_last_asset_into_the_global(cx: &mut TestAppContext) {
    cx.update(|cx| {
        cx.set_global(PaletteSyncState::default());
    });
    cx.update(|cx| apply_change("assets", "🚀", cx));
    cx.read(|cx| {
        assert_eq!(
            cx.global::<PaletteSyncState>().last_asset.as_deref(),
            Some("🚀")
        );
    });
}

#[gpui_kit::test]
async fn apply_change_with_an_unknown_slug_leaves_the_global_unchanged(cx: &mut TestAppContext) {
    cx.update(|cx| {
        cx.set_global(PaletteSyncState {
            color_hex: "#123456".to_string(),
            last_asset: None,
        });
    });
    cx.update(|cx| apply_change("bogus", "#FF0000", cx));
    cx.read(|cx| {
        assert_eq!(cx.global::<PaletteSyncState>().color_hex, "#123456");
    });
}

#[gpui_kit::test]
async fn apply_change_fires_global_observers(cx: &mut TestAppContext) {
    cx.update(|cx| {
        cx.set_global(PaletteSyncState::default());
    });
    let fired = cx.update(|_| std::rc::Rc::new(std::cell::Cell::new(false)));
    let observer = {
        let fired = fired.clone();
        cx.update(|cx| {
            cx.observe_global::<PaletteSyncState>(move |_| {
                fired.set(true);
            })
        })
    };
    cx.update(|cx| apply_change("color", "#FF0000", cx));
    assert!(fired.get());
    observer.detach();
}
