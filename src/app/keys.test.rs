use super::*;

/// `secondary` must parse to control outside macOS so Windows builds fire on
/// Ctrl, not the OS-intercepted Windows key.
#[cfg(target_os = "macos")]
#[test]
fn secondary_maps_to_platform_modifier() {
    let stroke = Keystroke::parse(TOGGLE_SEARCH).unwrap();
    assert!(stroke.modifiers.platform);
    assert!(!stroke.modifiers.control);
    assert_eq!(stroke.key.as_str(), "k");
    assert_eq!(label(TOGGLE_SEARCH), "⌘K");
}

#[cfg(not(target_os = "macos"))]
#[test]
fn secondary_maps_to_control() {
    let stroke = Keystroke::parse(TOGGLE_SEARCH).unwrap();
    assert!(stroke.modifiers.control);
    assert!(!stroke.modifiers.platform);
    assert_eq!(stroke.key.as_str(), "k");
    assert_eq!(label(TOGGLE_SEARCH), "Ctrl+K");
}

#[test]
fn page_bindings_use_secondary() {
    for i in 0..9 {
        let stroke = Keystroke::parse(&page(i)).unwrap();
        assert!(stroke.modifiers.control || stroke.modifiers.platform);
        assert_eq!(stroke.key.as_str(), (i + 1).to_string());
    }
}
