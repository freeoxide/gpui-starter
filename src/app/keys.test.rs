use super::*;

/// `secondary-` must resolve per host OS so Windows builds fire on Ctrl,
/// not the OS-intercepted Windows key, and macOS fires on ⌘.
#[test]
fn secondary_constants_resolve_per_os() {
    for (keystroke, key) in [(TOGGLE_SEARCH, "k"), (TOGGLE_SIDEBAR, "b")] {
        let mac = Keystroke::parse_with_os(keystroke, OperatingSystem::Mac).unwrap();
        assert!(
            mac.modifiers.platform,
            "{keystroke}: platform modifier missing on mac"
        );
        assert!(!mac.modifiers.control);
        assert_eq!(mac.key.as_str(), key);

        let linux = Keystroke::parse_with_os(keystroke, OperatingSystem::Linux).unwrap();
        assert!(
            linux.modifiers.control,
            "{keystroke}: control missing on linux"
        );
        assert!(!linux.modifiers.platform);
        assert_eq!(linux.key.as_str(), key);
    }

    // Kbd::format renders the platform modifier for the host OS (compile-time
    // cfg), so each label assertion only holds on the host it names.
    #[cfg(target_os = "macos")]
    assert_eq!(label(TOGGLE_SEARCH, OperatingSystem::Mac), "⌘K");
    #[cfg(not(target_os = "macos"))]
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

/// binding() panics on malformed sources at window construction, so every
/// keystroke it can receive must fail here in CI instead of at startup.
#[test]
fn keystroke_sources_parse_on_every_os() {
    let mut sources = vec![
        TOGGLE_SEARCH.to_string(),
        TOGGLE_SIDEBAR.to_string(),
        TOGGLE_QUICK_FLOAT.to_string(),
    ];
    sources.extend((0..9).map(page));
    for os in [
        OperatingSystem::Mac,
        OperatingSystem::Linux,
        OperatingSystem::Windows,
    ] {
        for source in &sources {
            Keystroke::parse_with_os(source, os)
                .unwrap_or_else(|e| panic!("{source} on {os:?}: {e}"));
        }
    }
}
