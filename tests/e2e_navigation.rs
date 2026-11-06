//! End-to-end tests for navigation: sidebar page registration, deep-link
//! route parsing, and view construction.

use gpui_starter::page::Page;
use gpui_starter::routes::AppRoute;

#[test]
fn test_sidebar_pages_exist() {
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
        Page::SystemControls,
        Page::Gallery,
        Page::CanvasShowcase,
        Page::InfiniteCanvas,
        Page::MultiuserCanvas,
        Page::FloatingUi,
        Page::ImageEditor,
        Page::Microphone,
        Page::Audio,
        Page::Analytics,
        Page::Music,
        Page::SignIn,
        Page::DesktopWidgets,
        Page::Wgpu,
        Page::ShaderCanvas,
        Page::ModelViewer,
    ];

    let all = Page::all();
    // Length equality plus membership: dropping a variant from Page::all()
    // cannot pass silently.
    assert_eq!(
        all.len(),
        expected.len(),
        "Page::all() has {} pages, expected {}",
        all.len(),
        expected.len()
    );

    for page in &expected {
        assert!(
            all.contains(page),
            "Page::{page:?} missing from Page::all()"
        );
    }

    for page in all {
        assert!(
            !page.title().is_empty(),
            "Page::{page:?} has an empty title"
        );
    }
}

#[test]
fn test_route_parsing() {
    // Top-level page routes
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
        (
            "gpui-starter://system-controls",
            AppRoute::Page(Page::SystemControls),
        ),
        (
            "gpui-starter://canvas-showcase",
            AppRoute::Page(Page::CanvasShowcase),
        ),
        (
            "gpui-starter://infinite-canvas",
            AppRoute::Page(Page::InfiniteCanvas),
        ),
        (
            "gpui-starter://multiuser-canvas",
            AppRoute::Page(Page::MultiuserCanvas),
        ),
        (
            "gpui-starter://floating-ui",
            AppRoute::Page(Page::FloatingUi),
        ),
        ("gpui-starter://analytics", AppRoute::Page(Page::Analytics)),
        ("gpui-starter://sign-in", AppRoute::Page(Page::SignIn)),
        (
            "gpui-starter://desktop-widgets",
            AppRoute::Page(Page::DesktopWidgets),
        ),
        // Sub-route
        (
            "gpui-starter://settings/notifications",
            AppRoute::SettingsNotifications,
        ),
        // Query parameters never select a route.
        (
            "gpui-starter://home?ref=test&source=menu",
            AppRoute::Page(Page::Home),
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

    // Round-trip: parse → to_url → parse
    for (url, _expected) in cases {
        let route = AppRoute::parse_deep_link(url).unwrap();
        let serialized = route.to_url();
        let reparsed = AppRoute::parse_deep_link(&serialized).unwrap();
        assert_eq!(
            reparsed, route,
            "round-trip failed for {url}: {route:?} → {serialized} → {reparsed:?}"
        );
    }

    // Invalid inputs
    let bad = &[
        "https://example.com",
        "gpui-starter://missing-page",
        "gpui-starter://home/extra",
        "not-a-url-at-all",
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

/// SettingsPage is exported with the expected constructor; full construction
/// needs GPUI globals this test deliberately avoids.
#[test]
fn test_settings_page_route_and_constructor() {
    let route = AppRoute::parse_deep_link("gpui-starter://settings").unwrap();
    assert_eq!(route, AppRoute::Page(Page::Settings));
    assert_eq!(route.page_for_render(), Page::Settings);

    let _: fn(
        &mut gpui_kit::Window,
        &mut gpui_kit::Context<gpui_starter::features::pages::SettingsPage>,
    ) -> gpui_starter::features::pages::SettingsPage =
        gpui_starter::features::pages::SettingsPage::new;
}

/// AboutPage is a zero-argument struct, instantiable without GPUI context.
#[test]
fn test_about_page_loads() {
    let _page = gpui_starter::features::pages::AboutPage::new();
}
