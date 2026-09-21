//! App accessibility foundation on top of gpui's accesskit integration.
//! gpui owns the platform bridge: each window builds an accesskit tree every
//! frame and dispatches assistive-tech actions back to elements (native
//! targets only — the gpui web stack ships no bridge, so accessibility is
//! inactive on wasm). Live state is read via [`snapshot`], recomputed by
//! [`refresh`].
//!
//! Element code should prefer gpui's own builders on
//! [`StatefulInteractiveElement`]: `role`, `aria_label`, `aria_description`,
//! `aria_keyshortcuts`, `aria_selected`, `aria_expanded`, `aria_toggled`,
//! `aria_value`, `aria_numeric_value` (+ min/max/step), `aria_placeholder`,
//! `aria_orientation`, `aria_level`, `aria_position_in_set`,
//! `aria_size_of_set`, `aria_row/column_index/count`, `aria_active_descendant`,
//! and `on_a11y_action`. A node is announced only when it has both an
//! `.id(...)` and a `.role(...)`; focused elements are announced when they
//! also `.track_focus(&handle)`. [`A11yExt`] adds the app-level helpers.

use gpui::{App, Global, Role, SharedString, StatefulInteractiveElement, accesskit::Live};

use crate::capabilities::CapabilityStatus;

/// App-level accessibility conveniences layered on gpui's element builders.
pub trait A11yExt: StatefulInteractiveElement {
    /// Set the accessible role and label together — the minimum for the
    /// element to be announced as one meaningful node. Requires `.id(...)`
    /// on the same element.
    fn a11y(self, role: Role, label: impl Into<SharedString>) -> Self {
        self.role(role).aria_label(label)
    }

    /// Report the node as a live region: assistive technology announces
    /// content changes at `politeness`. Only takes effect on elements that
    /// have an id and a role; gpui exposes live-region state only through
    /// its synthetic-children hook, which this wraps.
    fn a11y_live(self, politeness: Live) -> Self {
        self.a11y_synthetic_children(move |builder| {
            builder.parent_node().set_live(politeness);
        })
    }
}

impl<T: StatefulInteractiveElement> A11yExt for T {}

/// Point-in-time accessibility state, stored as a global so observers (the
/// diagnostics page) re-render when it changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccessibilitySnapshot {
    /// gpui's accesskit platform bridge is compiled in: true on native
    /// targets, false on wasm where the gpui web stack ships no bridge.
    pub accesskit_linked: bool,
    /// At least one open window is building an a11y tree right now, i.e.
    /// assistive technology is connected. Dynamic; see [`refresh`].
    pub bridge_enabled: bool,
    pub status: String,
    /// Windows whose a11y tree is currently active.
    pub active_windows: usize,
    /// Windows currently open.
    pub total_windows: usize,
}

impl Global for AccessibilitySnapshot {}

impl Default for AccessibilitySnapshot {
    fn default() -> Self {
        observe(bridge_available(), 0, 0)
    }
}

/// Compile-time fact: only native gpui targets carry the accesskit bridge.
fn bridge_available() -> bool {
    cfg!(not(target_family = "wasm"))
}

/// Pure snapshot constructor so the platform branches are testable without
/// open windows.
fn observe(
    bridge_available: bool,
    active_windows: usize,
    total_windows: usize,
) -> AccessibilitySnapshot {
    let bridge_enabled = bridge_available && active_windows > 0;
    let status = if !bridge_available {
        "no accesskit bridge in the wasm gpui stack".to_string()
    } else if bridge_enabled {
        format!("accesskit bridge active in {active_windows} of {total_windows} windows")
    } else {
        "accesskit bridge ready; no assistive technology connected".to_string()
    };
    AccessibilitySnapshot {
        accesskit_linked: bridge_available,
        bridge_enabled,
        status,
        active_windows,
        total_windows,
    }
}

/// Recompute bridge state from the open windows and store it. Call at App
/// level (after window open/close, or deferred) — from inside a window
/// update, that window itself reports inactive because gpui takes it out of
/// the map for the duration.
pub fn refresh(cx: &mut App) {
    let total_windows = cx.windows().len();
    let mut active_windows = 0;
    for handle in cx.windows() {
        let active = handle
            .update(cx, |_, window, _| window.is_a11y_active())
            .unwrap_or(false);
        active_windows += usize::from(active);
    }
    cx.set_global(observe(bridge_available(), active_windows, total_windows));
}

/// The last stored snapshot. Call [`refresh`] first when current counts
/// matter.
pub fn snapshot(cx: &App) -> AccessibilitySnapshot {
    cx.try_global::<AccessibilitySnapshot>()
        .cloned()
        .unwrap_or_default()
}

/// Capability entry for the registry: the a11y infrastructure is enabled
/// wherever the bridge exists; per-window activation stays dynamic and is
/// tracked in the snapshot instead.
pub fn capability_status(bridge_available: bool) -> CapabilityStatus {
    if bridge_available {
        CapabilityStatus {
            supported: true,
            enabled: true,
            degraded: false,
            reason: Some(
                "accesskit bridge activates per window when assistive technology connects".into(),
            ),
            last_error: None,
        }
    } else {
        // Unsupported-and-disabled, matching the wasm capability convention
        // (e.g. single_instance): a missing platform bridge is not degraded.
        CapabilityStatus {
            supported: false,
            enabled: false,
            degraded: false,
            reason: Some("accesskit bridge unavailable on wasm".into()),
            last_error: None,
        }
    }
}

pub fn initialize(cx: &mut App) {
    refresh(cx);
    let stored = snapshot(cx);
    crate::capabilities::set(
        "accessibility",
        capability_status(stored.accesskit_linked),
        cx,
    );
    tracing::info!(
        target: "gpui_starter::accessibility",
        status = %stored.status,
        "accessibility initialized"
    );
}

#[cfg(test)]
#[path = "accessibility.test.rs"]
mod accessibility_test;
