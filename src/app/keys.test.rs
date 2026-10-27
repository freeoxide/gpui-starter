use super::*;

/// `secondary` must resolve per host OS so Windows builds fire on Ctrl,
/// not the OS-intercepted Windows key, and macOS fires on ⌘.
#[cfg(target_os = "macos")]
#[test]
fn secondary_maps_to_platform_modifier() {
    let stroke = Keystroke::parse_with_os(TOGGLE_SEARCH, OperatingSystem::Mac).unwrap();
    assert!(stroke.modifiers.platform);
    assert!(!stroke.modifiers.control);
    assert_eq!(stroke.key.as_str(), "k");
    assert_eq!(label(TOGGLE_SEARCH, OperatingSystem::Mac), "⌘K");
}

#[cfg(not(target_os = "macos"))]
#[test]
fn secondary_maps_to_control() {
    let stroke = Keystroke::parse_with_os(TOGGLE_SEARCH, OperatingSystem::Linux).unwrap();
    assert!(stroke.modifiers.control);
    assert!(!stroke.modifiers.platform);
    assert_eq!(stroke.key.as_str(), "k");
    assert_eq!(label(TOGGLE_SEARCH, OperatingSystem::Linux), "Ctrl+K");
}

#[test]
fn page_bindings_use_secondary() {
    for i in 0..9 {
        let mac = Keystroke::parse_with_os(&page(i), OperatingSystem::Mac).unwrap();
        assert!(mac.modifiers.platform);
        let other = Keystroke::parse_with_os(&page(i), OperatingSystem::Linux).unwrap();
        assert!(other.modifiers.control);
        assert_eq!(other.key.as_str(), (i + 1).to_string());
    }
}
