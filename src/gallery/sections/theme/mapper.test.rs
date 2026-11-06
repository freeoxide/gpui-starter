use gpui_kit::component::ThemeColor;

use super::parse_theme_key;

/// A canonical spelling `ThemeConfigColors` rejects would make the viewer
/// mark every occurrence of that field inherited.
#[test]
fn canonical_keys_stay_accepted_by_theme_config_colors() {
    let theme = serde_json::to_value(ThemeColor::dark().as_ref()).expect("ThemeColor serializes");
    let serde_json::Value::Object(theme_map) = theme else {
        panic!("ThemeColor must serialize to an object");
    };

    let mut payload = serde_json::Map::new();
    for key in theme_map.keys() {
        let canonical = parse_theme_key(key).canonical_key;
        payload.insert(canonical, serde_json::Value::String("#000000".into()));
    }

    // ThemeConfigColors ignores unknown keys, so acceptance is observed by
    // populating every key and checking it survives a serialize round-trip.
    let config: gpui_kit::component::theme::ThemeConfigColors =
        serde_json::from_value(serde_json::Value::Object(payload.clone()))
            .expect("populated config deserializes");
    let round_tripped = serde_json::to_value(&config).expect("config serializes");
    let accepted = round_tripped.as_object().expect("config is an object");

    let offenders: Vec<&String> = payload
        .keys()
        .filter(|canonical| !accepted.contains_key(*canonical))
        .collect();
    assert!(
        offenders.is_empty(),
        "canonical keys ThemeConfigColors rejects: {offenders:?}"
    );
}
