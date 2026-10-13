use super::*;

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
