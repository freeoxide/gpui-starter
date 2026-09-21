use super::*;
use gpui::{Role, accesskit::Live, div, prelude::*};

#[test]
fn default_snapshot_matches_compiled_bridge() {
    let snap = AccessibilitySnapshot::default();
    assert_eq!(snap.accesskit_linked, cfg!(not(target_family = "wasm")));
    assert!(!snap.bridge_enabled);
    assert_eq!(snap.active_windows, 0);
    assert_eq!(snap.total_windows, 0);
    assert!(!snap.status.is_empty());
}

#[test]
fn observe_reports_active_windows() {
    let snap = observe(true, 2, 3);
    assert!(snap.accesskit_linked);
    assert!(snap.bridge_enabled);
    assert_eq!(snap.active_windows, 2);
    assert_eq!(snap.total_windows, 3);
    assert!(snap.status.contains("2 of 3"));
}

#[test]
fn observe_without_active_windows_reports_ready() {
    let snap = observe(true, 0, 1);
    assert!(snap.accesskit_linked);
    assert!(!snap.bridge_enabled);
    assert!(snap.status.contains("ready"));
}

#[test]
fn observe_without_bridge_never_enables() {
    let snap = observe(false, 1, 1);
    assert!(!snap.accesskit_linked);
    assert!(!snap.bridge_enabled);
    assert!(snap.status.contains("wasm"));
}

#[test]
fn capability_status_reflects_bridge_availability() {
    let native = capability_status(true);
    assert!(native.supported);
    assert!(native.enabled);
    assert!(!native.degraded);
    assert!(native.reason.is_some());

    let wasm = capability_status(false);
    assert!(!wasm.supported);
    assert!(!wasm.enabled);
    assert!(!wasm.degraded);
    assert!(wasm.reason.is_some());
}

#[test]
fn element_helpers_chain_on_stateful_divs() {
    // Compile-shape guard: the helpers resolve on Stateful<Div> and stay
    // chainable with gpui's own aria builders.
    let _element = div()
        .id("save")
        .a11y(Role::Button, "Save")
        .aria_description("Stores the current form")
        .a11y_live(Live::Polite);
}
