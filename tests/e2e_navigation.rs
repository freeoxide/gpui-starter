//! End-to-end tests for navigation: sidebar page registration, deep-link
//! route parsing, and view construction.

use gpui_starter::routes::AppRoute;
use gpui_starter::sidebar::Page;

// ---------------------------------------------------------------------------
// Sidebar pages
// ---------------------------------------------------------------------------

#[test]
fn test_sidebar_pages_exist() {
    let all = Page::all();
    // The app must expose exactly nine pages.
    assert_eq!(all.len(), 9, "expected 9 sidebar pages, got {}", all.len());

    // Every page variant that AppRoot::active_page_view matches on must be
    // present in the canonical list.
    let expected = [
        Page::Home,
        Page::Form,
        Page::Settings,
        Page::Notifications,
        Page::Diagnostics,
        Page::QueryPlayground,
        Page::QueryDevToolsV2,
        Page::ErrorPlayground,
        Page::About,
    ];

    for page in &expected {
        assert!(
            all.contains(page),
            "Page::{page:?} missing from Page::all()"
        );
    }

    // Each page must return a non-empty title.
    for page in all {
        assert!(
            !page.title().is_empty(),
            "Page::{page:?} has an empty title"
        );
    }
}

// ---------------------------------------------------------------------------
// Route parsing (deep links)
// ---------------------------------------------------------------------------

#[test]
fn test_route_parsing() {
    // --- Top-level page routes -------------------------------------------
    let cases = &[
        ("gpui-starter://home", AppRoute::Page(Page::Home)),
        ("gpui-starter://form", AppRoute::Page(Page::Form)),
        ("gpui-starter://settings", AppRoute::Page(Page::Settings)),
        (
            "gpui-starter://notifications",
            AppRoute::Page(Page::Notifications),
        ),
        (
            "gpui-starter://diagnostics",
            AppRoute::Page(Page::Diagnostics),
        ),
        ("gpui-starter://about", AppRoute::Page(Page::About)),
        // Sub-route
        (
            "gpui-starter://settings/notifications",
            AppRoute::SettingsNotifications,
        ),
    ];

    for (url, expected) in cases {
        let parsed =
            AppRoute::parse_deep_link(url).unwrap_or_else(|e| panic!("failed to parse {url}: {e}"));
        assert_eq!(
            parsed, *expected,
            "parse_deep_link({url}): expected {expected:?}, got {parsed:?}"
        );
    }

    // --- Round-trip: parse → to_url → parse ------------------------------
    for (url, _expected) in cases {
        let route = AppRoute::parse_deep_link(url).unwrap();
        let serialized = route.to_url();
        let reparsed = AppRoute::parse_deep_link(&serialized).unwrap();
        assert_eq!(
            reparsed, route,
            "round-trip failed for {url}: {route:?} → {serialized} → {reparsed:?}"
        );
    }

    // --- Invalid inputs --------------------------------------------------
    let bad = &[
        "https://example.com",
        "gpui-starter://missing-page",
        "gpui-starter://settings/..%2Fetc",
        "",
    ];

    for url in bad {
        assert!(
            AppRoute::parse_deep_link(url).is_err(),
            "expected parse failure for {url:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// View construction
// ---------------------------------------------------------------------------

/// SettingsPage is exported with the expected constructor; full construction
/// needs GPUI globals this test deliberately avoids.
#[test]
fn test_settings_page_loads() {
    // Verify the route parses correctly.
    let route = AppRoute::parse_deep_link("gpui-starter://settings").unwrap();
    assert_eq!(route, AppRoute::Page(Page::Settings));
    assert_eq!(route.page_for_render(), Page::Settings);

    // Compile-time check of the constructor signature only.
    let _type_check: fn() = || {
        fn _assert_settings_page_constructible() {
            // Compile-time proof of the public constructor signature.
            let _: fn(
                &mut gpui_kit::Window,
                &mut gpui_kit::Context<gpui_starter::views::SettingsPage>,
            ) -> gpui_starter::views::SettingsPage = gpui_starter::views::SettingsPage::new;
        }
    };
}

/// AboutPage is a zero-argument struct, instantiable without GPUI context.
#[test]
fn test_about_page_loads() {
    let _page = gpui_starter::views::AboutPage::new();
}
