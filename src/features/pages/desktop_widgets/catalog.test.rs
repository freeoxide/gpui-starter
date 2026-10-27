use super::*;

fn entry(kind: &str, placement: &str, layer: &str) -> PersistedWidget {
    PersistedWidget {
        kind: kind.to_string(),
        enabled: true,
        placement: placement.to_string(),
        always_on_top: true,
        click_through: false,
        layer: layer.to_string(),
    }
}

#[test]
fn kind_strings_round_trip_for_all_kinds() {
    for kind in available_kinds() {
        assert_eq!(WidgetKind::from_str(kind.as_str()), Some(kind));
    }
    assert_eq!(WidgetKind::from_str("nope"), None);
}

#[test]
fn title_keys_are_unique_per_kind() {
    let kinds = available_kinds();
    for (i, kind) in kinds.iter().enumerate() {
        for other in &kinds[i + 1..] {
            assert_ne!(kind.title_key(), other.title_key());
        }
    }
}

#[test]
fn placement_strings_round_trip_for_all_nine() {
    let all = [
        Placement9::TopLeft,
        Placement9::Top,
        Placement9::TopRight,
        Placement9::Left,
        Placement9::Center,
        Placement9::Right,
        Placement9::BottomLeft,
        Placement9::Bottom,
        Placement9::BottomRight,
    ];
    for placement in all {
        assert_eq!(
            placement_from_str(placement_to_str(placement)),
            Some(placement)
        );
    }
    assert_eq!(placement_from_str("garbage"), None);
}

#[test]
fn placement_offsets_push_away_from_the_touched_edges() {
    let inset = px(16.);
    let zero = px(0.);
    let cases = [
        (Placement9::TopLeft, point(inset, inset)),
        (Placement9::Top, point(zero, inset)),
        (Placement9::TopRight, point(-inset, inset)),
        (Placement9::Left, point(inset, zero)),
        (Placement9::Center, point(zero, zero)),
        (Placement9::Right, point(-inset, zero)),
        (Placement9::BottomLeft, point(inset, -inset)),
        (Placement9::Bottom, point(zero, -inset)),
        (Placement9::BottomRight, point(-inset, -inset)),
    ];
    for (placement, expected) in cases {
        assert_eq!(placement_offset(placement), expected);
    }
}

#[test]
fn sanitize_drops_unknown_kinds_and_keeps_first_of_each_kind() {
    let raw = vec![
        entry("clock", "top_left", "above"),
        entry("nope", "center", "above"),
        entry("clock", "bottom_right", "pinned"),
        entry("metrics", "garbage", "garbage"),
    ];
    let out = sanitize(&raw);
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].kind, "clock");
    assert_eq!(out[0].placement, "top_left");
    assert_eq!(out[1].kind, "metrics");
    assert_eq!(out[1].placement, "center");
    assert_eq!(out[1].layer, "above");
}

#[test]
fn sanitize_bounds_a_long_input() {
    let raw: Vec<PersistedWidget> = (0..12)
        .map(|i| {
            if i % 2 == 0 {
                entry("clock", "center", "above")
            } else {
                entry("battery", "center", "above")
            }
        })
        .collect();
    let out = sanitize(&raw);
    assert!(out.len() <= 8);
    assert_eq!(out.len(), 2);
}

#[test]
fn sanitize_normalizes_layer_values() {
    let out = sanitize(&[
        entry("clock", "center", "pinned"),
        entry("battery", "center", "weird"),
    ]);
    assert_eq!(out[0].layer, "pinned");
    assert_eq!(out[1].layer, "above");
}

#[test]
fn the_full_build_catalog_offers_four_kinds() {
    assert_eq!(available_kinds().len(), 4);
}

#[test]
fn defaults_carry_distinct_window_geometry_per_kind() {
    let kinds = available_kinds();
    for (i, kind) in kinds.iter().enumerate() {
        for other in &kinds[i + 1..] {
            assert_ne!(defaults(*kind).size, defaults(*other).size);
        }
    }
}
