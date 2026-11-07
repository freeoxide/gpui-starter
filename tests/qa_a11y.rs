//! Accessibility contracts that hold without a live window, each pinned
//! against code or docs rather than a running UI.

#[allow(dead_code)]
mod common;

use common::repo_file;
use gpui_kit::{Role, accesskit::Live, div, prelude::*};

use gpui_starter::accessibility::{A11yExt, AccessibilitySnapshot, capability_status};
use gpui_starter::ui::widgets::virtual_list_item;

/// `initialize` registers exactly `capability_status(snapshot.accesskit_linked)`,
/// so these pin what that registration may claim.
#[test]
fn capability_matches_the_compiled_bridge() {
    let bridge = cfg!(not(target_family = "wasm"));
    let status = capability_status(bridge);

    assert_eq!(
        status.supported, bridge,
        "supported must equal bridge availability"
    );
    assert_eq!(
        status.enabled, bridge,
        "enabled must never outrun the bridge"
    );
    assert!(!status.degraded, "init-time registration is never degraded");
    assert!(status.last_error.is_none());
    assert!(status.reason.is_some_and(|reason| !reason.is_empty()));
}

#[test]
fn capability_without_bridge_is_unsupported_not_degraded() {
    let status = capability_status(false);

    assert!(!status.supported);
    assert!(!status.enabled);
    // Repo convention: no platform bridge is unsupported-and-disabled; degraded
    // means supported-but-impaired.
    assert!(!status.degraded);
}

#[test]
fn default_snapshot_reflects_compiled_bridge_without_windows() {
    let snap = AccessibilitySnapshot::default();

    assert_eq!(snap.accesskit_linked, cfg!(not(target_family = "wasm")));
    // No assistive tech can be active with zero windows, so never enabled here.
    assert!(!snap.bridge_enabled);
    assert_eq!(snap.active_windows, 0);
    assert_eq!(snap.total_windows, 0);
    assert!(!snap.status.is_empty());
}

// A11y nodes are only observable through a live window with assistive tech
// attached, so these pin the public surface and its gpui interop.

#[test]
fn a11y_helpers_are_public_and_chainable() {
    let _element = div()
        .id("save")
        .a11y(Role::Button, "Save")
        .aria_description("Stores the current form")
        .a11y_live(Live::Polite);
}

#[test]
fn virtual_list_item_chains_with_selection_state() {
    let _row = virtual_list_item("row-0", "gpx-fetch 200 1h ago", 0).aria_selected(true);
}

#[test]
fn checklist_documents_the_real_bridge() {
    let content = repo_file("docs/accessibility-checklist.md");
    let normalized = content.to_lowercase();

    assert!(normalized.contains("accesskit"));
    assert!(normalized.contains("live region"));
    assert!(normalized.contains("wasm"));
    assert!(normalized.contains("diagnostics"));
    // The old aspirational checklist wording must not return.
    assert!(!normalized.contains("track accesskit integration points"));
}

#[test]
fn checklist_bridge_claims_anchor_in_code() {
    let palette = repo_file("src/features/command_palette.rs");
    assert!(
        palette.contains("launcher-status"),
        "palette live region must exist as documented"
    );

    let shell = repo_file("src/shell/root/app_root/render.rs");
    assert!(
        shell.contains("Role::Navigation"),
        "navigation landmark must exist as documented"
    );
    assert!(
        shell.contains("sidebar_main_navigation"),
        "the landmark's name must carry the localization key"
    );

    let virtual_list = repo_file("src/ui/widgets/virtual_list.rs");
    assert!(
        virtual_list.contains("Role::ListItem"),
        "virtual list rows must be list items as documented"
    );
}

// Keyed ids prevent the duplicate-a11y-node-id panic; virtual_list's anchors
// pin the 0-based position contract. Both need a live window (see header).

#[test]
fn virtual_list_pins_zero_based_positions_and_container_setsize() {
    let virtual_list = repo_file("src/ui/widgets/virtual_list.rs");

    assert!(
        virtual_list.contains(".aria_position_in_set(index)"),
        "accesskit stores position_in_set 0-based; AT bridges report stored+1"
    );
    assert!(
        !virtual_list.contains(".aria_position_in_set(index + 1)"),
        "a +1 here reads one high through every AT bridge"
    );
    // The ancestor walk reads only the List container's declaration, so the
    // row builder must not carry an item-level setsize.
    assert_eq!(
        virtual_list.matches(".aria_size_of_set(total)").count(),
        1,
        "only the Role::List container may declare setsize"
    );
}

#[test]
fn playground_helpers_derive_ids_from_call_site_keys() {
    let helpers = repo_file("src/features/pages/query_playground/ui_helpers.rs");

    for (keyed, text_derived) in [
        ("pg-mini-title-{key}", "pg-mini-title-{}"),
        ("pg-status-{key}", "pg-status-{}"),
        ("pg-chip-{key}", "pg-chip-{}"),
    ] {
        assert!(
            helpers.contains(keyed),
            "{keyed} must stay: ids derive from the per-call-site key"
        );
        assert!(
            !helpers.contains(text_derived),
            "{text_derived} is the collision shape: text repeats across call sites"
        );
    }
}
