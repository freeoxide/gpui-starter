use super::*;

use gpui_kit::TestAppContext;

use crate::app_state::{AppConfig, debounce_test_lock, force_save, install_for_tests};

fn entry(id: &str) -> PersistedPalette {
    PersistedPalette {
        id: id.to_string(),
        open: true,
        surface: "window".to_string(),
        x: 10.0,
        y: 20.0,
        width: 260.0,
        height: 320.0,
        color: Some("#FF0000".to_string()),
        note: Some("hello".to_string()),
    }
}

#[test]
fn sanitize_keeps_known_ids_once_each() {
    let raw = vec![entry("color"), entry("assets"), entry("bogus")];
    let clean = sanitize(&raw);
    assert_eq!(clean.len(), 2);
    assert!(clean.contains_key(&PaletteId::Color));
    assert!(clean.contains_key(&PaletteId::Assets));
    assert!(!clean.contains_key(&PaletteId::Note));
}

#[test]
fn sanitize_first_wins_on_duplicates() {
    let mut first = entry("color");
    first.x = 1.0;
    let mut second = entry("color");
    second.x = 999.0;
    let clean = sanitize(&[first, second]);
    assert_eq!(clean[&PaletteId::Color].x, 1.0);
}

#[test]
fn sanitize_drops_non_finite_coords() {
    for bad in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let mut raw = entry("note");
        raw.x = bad;
        assert!(!sanitize(&[raw]).contains_key(&PaletteId::Note));
    }
}

#[test]
fn sanitize_drops_non_positive_and_oversized_dimensions() {
    for (w, h) in [(0., 100.), (100., -1.), (100_001., 100.), (100., 100_001.)] {
        let mut raw = entry("color");
        raw.width = w;
        raw.height = h;
        assert!(!sanitize(&[raw]).contains_key(&PaletteId::Color));
    }
}

#[test]
fn sanitize_normalizes_unknown_surfaces_to_panel() {
    let mut raw = entry("color");
    raw.surface = "desktop".to_string();
    let clean = sanitize(&[raw]);
    assert_eq!(clean[&PaletteId::Color].surface, "panel");
}

#[test]
fn sanitize_first_wins_across_many_duplicates() {
    let raw: Vec<PersistedPalette> = (0..20)
        .map(|i| {
            let mut e = entry("color");
            e.x = i as f32;
            e
        })
        .collect();
    let clean = sanitize(&raw);
    assert_eq!(clean.len(), 1);
    assert_eq!(clean[&PaletteId::Color].x, 0.0);
}

#[test]
fn persisted_palette_serde_round_trip() {
    let original = entry("color");
    let json = serde_json::to_string(&original).unwrap();
    assert!(json.contains("\"surface\""));
    let restored: PersistedPalette = serde_json::from_str(&json).unwrap();
    assert_eq!(original, restored);
}

#[gpui_kit::test]
async fn update_slot_merges_its_fields_without_clobbering_the_rest(cx: &mut TestAppContext) {
    let _guard = debounce_test_lock();
    let mut seeded = entry("note");
    seeded.note = Some("seeded".to_string());
    cx.update(|cx| {
        install_for_tests(
            AppConfig {
                floating_palettes: vec![seeded],
                ..AppConfig::default()
            },
            cx,
        )
    });

    cx.update(|cx| {
        update_slot(PaletteId::Note, cx, |e| e.note = Some("hello".to_string()));
        update_slot(PaletteId::Color, cx, |e| {
            e.color = Some("#E5484D".to_string())
        });
    });

    cx.read(|cx| {
        let note = slot(PaletteId::Note, cx).expect("the seeded note slot persists");
        assert_eq!(note.note.as_deref(), Some("hello"));
        assert!(note.open);
        assert_eq!(note.surface, "window");
        assert_eq!(note.x, 10.0);
        assert_eq!(note.y, 20.0);
        assert_eq!(note.width, 260.0);
        assert_eq!(note.height, 320.0);
        assert_eq!(note.color.as_deref(), Some("#FF0000"));

        let color = slot(PaletteId::Color, cx).expect("an unknown slot seeds defaults");
        assert_eq!(color.color.as_deref(), Some("#E5484D"));
        assert_eq!(color.surface, "panel");
    });
    cx.update(|cx| {
        crate::app_state::with_config(cx, |config| {
            let note_entries = config
                .floating_palettes
                .iter()
                .filter(|p| p.id == "note")
                .count();
            assert_eq!(note_entries, 1, "save_slot must replace, never append");
        });
    });
    cx.update(force_save);
}
