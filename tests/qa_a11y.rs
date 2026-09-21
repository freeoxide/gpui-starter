//! Accessibility contracts that hold without a live window: capability
//! registration truthfulness, snapshot semantics, the public element-helper
//! surface, checklist claims, and runtime-proven rules, all anchored in code.

use gpui::{Role, accesskit::Live, div, prelude::*};

use gpui_starter::accessibility::{A11yExt, AccessibilitySnapshot, capability_status};
use gpui_starter::ui::widgets::virtual_list_item;

// ---------------------------------------------------------------------------
// Capability registration truthfulness
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Snapshot semantics
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Element helper surface
// ---------------------------------------------------------------------------

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
    let _row = virtual_list_item("row-0", "gpx-fetch 200 1h ago", 0, 4).aria_selected(true);
}

// ---------------------------------------------------------------------------
// Checklist claims must match code
// ---------------------------------------------------------------------------

#[test]
fn checklist_documents_the_real_bridge() {
    let content = std::fs::read_to_string("docs/accessibility-checklist.md")
        .expect("read accessibility checklist");
    let normalized = content.to_lowercase();

    assert!(normalized.contains("accesskit"));
    assert!(normalized.contains("live region"));
    assert!(normalized.contains("wasm"));
    assert!(normalized.contains("diagnostics"));
    // The bridge is real; the old aspirational wording must stay gone.
    assert!(!normalized.contains("track accesskit integration points"));
}

#[test]
fn checklist_bridge_claims_anchor_in_code() {
    let palette =
        std::fs::read_to_string("src/features/command_palette.rs").expect("read command palette");
    assert!(
        palette.contains("launcher-status"),
        "palette live region must exist as documented"
    );

    let shell =
        std::fs::read_to_string("src/shell/root/app_root/render.rs").expect("read shell render");
    assert!(
        shell.contains("Main navigation"),
        "navigation landmark must exist as documented"
    );

    let virtual_list =
        std::fs::read_to_string("src/ui/widgets/virtual_list.rs").expect("read virtual list");
    assert!(
        virtual_list.contains("Role::ListItem"),
        "virtual list rows must be list items as documented"
    );
}

// ---------------------------------------------------------------------------
// Runtime-verified rules must match code
// ---------------------------------------------------------------------------

// Keyed ids prevent the duplicate-a11y-node-id panic; virtual_list's anchors
// pin the 0-based position contract. Both need a live window (see header).

#[test]
fn virtual_list_pins_zero_based_positions_and_container_setsize() {
    let virtual_list =
        std::fs::read_to_string("src/ui/widgets/virtual_list.rs").expect("read virtual list");

    assert!(
        virtual_list.contains(".aria_position_in_set(index)"),
        "accesskit stores position_in_set 0-based; AT bridges report stored+1"
    );
    assert!(
        !virtual_list.contains(".aria_position_in_set(index + 1)"),
        "a +1 here reads one high through every AT bridge"
    );
    // 2 = the container call (the only one the bridge reads) + the item-level
    // call; item-only setsize surfaces as none through the ancestor walk.
    assert_eq!(
        virtual_list.matches(".aria_size_of_set(total)").count(),
        2,
        "the bridge walks ancestors only, so the List container must declare setsize"
    );
}

#[test]
fn playground_helpers_derive_ids_from_call_site_keys() {
    let helpers = std::fs::read_to_string("src/features/pages/query_playground/ui_helpers.rs")
        .expect("read playground ui helpers");

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
