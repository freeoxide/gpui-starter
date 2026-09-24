use super::*;

#[test]
fn parses_supported_deep_links() {
    let home = AppRoute::parse_deep_link("gpui-starter://home").unwrap();
    assert_eq!(home, AppRoute::Page(Page::Home));
    assert_eq!(home.to_url(), "gpui-starter://home");
    assert_eq!(
        AppRoute::parse_deep_link("gpui-starter://settings/notifications").unwrap(),
        AppRoute::SettingsNotifications
    );
    assert_eq!(
        AppRoute::parse_deep_link("gpui-starter://diagnostics").unwrap(),
        AppRoute::Page(Page::Diagnostics)
    );
    assert_eq!(
        AppRoute::parse_deep_link("gpui-starter://notifications").unwrap(),
        AppRoute::Page(Page::Notifications)
    );
}

#[test]
fn rejects_unknown_deep_links() {
    assert!(AppRoute::parse_deep_link("https://example.com").is_err());
    assert!(AppRoute::parse_deep_link("gpui-starter://missing").is_err());
}

#[test]
fn rejects_wrong_scheme() {
    let err = AppRoute::parse_deep_link("https://home").unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("unsupported scheme"),
        "expected scheme rejection, got: {msg}"
    );
}

#[test]
fn rejects_unexpected_host() {
    let err = AppRoute::parse_deep_link("gpui-starter://evil-host").unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("unexpected host"),
        "expected host rejection, got: {msg}"
    );
}

#[test]
fn rejects_path_traversal_in_segment() {
    let err = AppRoute::parse_deep_link("gpui-starter://settings/..%2Fetc").unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("path traversal") || msg.contains("forbidden character"),
        "expected path-traversal rejection, got: {msg}"
    );
}

#[test]
fn rejects_null_byte_in_segment() {
    let err = AppRoute::parse_deep_link("gpui-starter://settings/\0notifications").unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("forbidden character") || msg.contains("invalid deep link"),
        "expected null-byte rejection, got: {msg}"
    );
}

#[test]
fn accepts_deep_link_with_clean_query_params() {
    let result = AppRoute::parse_deep_link("gpui-starter://home?ref=test&source=menu");
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), AppRoute::Page(Page::Home));
}

#[test]
fn validate_path_segment_helper() {
    assert!(validate_path_segment("notifications").is_ok());
    assert!(validate_path_segment("..").is_err());
    assert!(validate_path_segment("foo/../bar").is_err());
    assert!(validate_path_segment("seg\x00ment").is_err());
}

/// `to_hash` must cover EVERY page (exhaustive over `Page::all`), plus the
/// settings sub-route — a new Page variant fails here until it is added.
#[test]
fn to_hash_round_trips_every_page() {
    for page in Page::all() {
        let route = AppRoute::Page(*page);
        let hash = route.to_hash();
        assert!(
            hash.starts_with("#/"),
            "hash must be #-prefixed with a leading slash: {hash}"
        );
        assert_eq!(AppRoute::from_hash(&hash).unwrap(), route, "page {page:?}");
    }
    let route = AppRoute::SettingsNotifications;
    assert_eq!(route.to_hash(), "#/settings/notifications");
    assert_eq!(
        AppRoute::from_hash(&route.to_hash()).unwrap(),
        AppRoute::SettingsNotifications
    );
}

#[test]
fn from_hash_matches_to_hash_for_every_valid_host() {
    for host in VALID_HOSTS {
        let route = AppRoute::parse_deep_link(&format!("{}://{}", APP_URL_SCHEME, host)).unwrap();
        assert_eq!(AppRoute::from_hash(&route.to_hash()).unwrap(), route);
    }
}

#[test]
fn parses_hash_deep_links() {
    assert_eq!(
        AppRoute::from_hash("#/home").unwrap(),
        AppRoute::Page(Page::Home)
    );
    assert_eq!(
        AppRoute::from_hash("#/settings").unwrap(),
        AppRoute::Page(Page::Settings)
    );
    assert_eq!(
        AppRoute::from_hash("#/settings/notifications").unwrap(),
        AppRoute::SettingsNotifications
    );
    assert_eq!(
        AppRoute::from_hash("#/diagnostics").unwrap(),
        AppRoute::Page(Page::Diagnostics)
    );
    // Leading `#` optional (lenient for stripped inputs).
    assert_eq!(
        AppRoute::from_hash("/home").unwrap(),
        AppRoute::Page(Page::Home)
    );
}

#[test]
fn empty_hash_resolves_to_default_route() {
    assert_eq!(AppRoute::from_hash("").unwrap(), AppRoute::default());
    assert_eq!(AppRoute::from_hash("#").unwrap(), AppRoute::default());
    assert_eq!(AppRoute::from_hash("#/").unwrap(), AppRoute::default());
    assert_eq!(AppRoute::from_hash("  #  ").unwrap(), AppRoute::default());
}

#[test]
fn from_hash_rejects_unknown_and_unsafe_hashes() {
    assert!(AppRoute::from_hash("#/missing").is_err());
    // Encoded path traversal survives URL parsing and is rejected by the
    // deep-link validator (same input the parse_deep_link tests use).
    assert!(AppRoute::from_hash("#/settings/..%2Fetc").is_err());
    assert!(AppRoute::from_hash("#/settings/\0notifications").is_err());
    assert!(AppRoute::from_hash("#/home/extra/segments").is_err());
    assert!(AppRoute::from_hash("https://example.com").is_err());
}

#[test]
fn from_hash_neutralizes_plain_dot_segments_via_url_parsing() {
    // Plain `..` is dot-segment-normalized by the `url` crate before the
    // validator sees it; the safe outcome must stay deterministic.
    assert_eq!(
        AppRoute::from_hash("#/settings/..").unwrap(),
        AppRoute::Page(Page::Settings)
    );
}

#[test]
fn hash_and_url_forms_agree_on_the_route_set() {
    for page in Page::all() {
        let route = AppRoute::Page(*page);
        let url = route.to_url();
        let expected_hash = format!("#/{}", url.trim_start_matches("gpui-starter://"));
        assert_eq!(route.to_hash(), expected_hash);
    }
    let route = AppRoute::SettingsNotifications;
    assert_eq!(
        route.to_hash(),
        format!("#/{}", route.to_url().trim_start_matches("gpui-starter://"))
    );
}
