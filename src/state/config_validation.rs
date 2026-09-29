//! Non-fatal config validation: lints an [`AppConfig`] into [`ConfigLint`]
//! warnings without ever rejecting the load.

use std::collections::HashSet;

use crate::app::{LOCALE_EN, LOCALE_ZH_CN};
use crate::state::config_store::{AppConfig, PersistedWindowBounds};

const LOG_TARGET: &str = "gpui_starter::config_validation";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LintSeverity {
    /// Informational note; no action required (e.g. an unusual-but-legal value).
    Info,
    /// Something looks off and probably warrants attention, but the app will
    /// still run correctly (e.g. a window position that may be off-screen).
    Warning,
    /// A value the app cannot interpret; it will fall back to a default
    /// (e.g. an unknown locale or update channel).
    Error,
}

impl LintSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            LintSeverity::Info => "info",
            LintSeverity::Warning => "warning",
            LintSeverity::Error => "error",
        }
    }
}

impl std::fmt::Display for LintSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Clone, Debug)]
pub struct ConfigLint {
    /// Dotted path to the offending field (e.g. `theme`, `window_bounds.x`).
    pub field: String,
    /// Human-readable description of the issue and the likely fallback.
    pub message: String,
    pub severity: LintSeverity,
}

impl ConfigLint {
    fn new(field: impl Into<String>, severity: LintSeverity, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            severity,
            message: message.into(),
        }
    }
}

const KNOWN_UPDATE_CHANNELS: &[&str] = &["stable", "beta", "nightly"];

/// Locales the app ships translations for. Mirrors the two constants the rest
/// of the crate treats as authoritative (`LOCALE_EN`, `LOCALE_ZH_CN`).
const KNOWN_LOCALES: &[&str] = &[LOCALE_EN, LOCALE_ZH_CN];

/// Permission keys the app understands; extend when a new permission ships.
const KNOWN_PERMISSIONS: &[&str] = &["notifications"];

/// Below this the window is unusably small; above this it likely exceeds a
/// typical display and will be clamped by the platform.
const MIN_WINDOW_DIM: f32 = 100.0;
/// Lint ceiling for one window dimension; the store's corruption ceiling
/// (`MAX_PLAUSIBLE_DIM` in config_store) is what drops bounds outright.
const MAX_WINDOW_DIM: f32 = 8192.0;

pub fn validate_config(cfg: &AppConfig) -> Vec<ConfigLint> {
    let mut lints = Vec::new();

    lints.extend(validate_window_bounds(cfg.window_bounds.as_ref()));
    lints.extend(validate_update_channel(&cfg.update_channel));
    lints.extend(validate_locale(&cfg.locale));
    lints.extend(validate_theme(&cfg.theme));
    lints.extend(validate_granted_permissions(&cfg.granted_permissions));

    if !lints.is_empty() {
        tracing::warn!(
            target: LOG_TARGET,
            count = lints.len(),
            "non-fatal config validation produced warnings; config will still load"
        );
    }

    lints
}

fn validate_window_bounds(bounds: Option<&PersistedWindowBounds>) -> Vec<ConfigLint> {
    let Some(b) = bounds else {
        return Vec::new();
    };

    let mut out = Vec::new();
    let dims = [
        ("window_bounds.x", b.x),
        ("window_bounds.y", b.y),
        ("window_bounds.width", b.width),
        ("window_bounds.height", b.height),
    ];

    for (field, value) in dims {
        if value.is_nan() {
            out.push(ConfigLint::new(
                field,
                LintSeverity::Error,
                format!("{field} is NaN; falling back to default window placement"),
            ));
        } else if value.is_infinite() {
            out.push(ConfigLint::new(
                field,
                LintSeverity::Error,
                format!("{field} is infinite; falling back to default window placement"),
            ));
        } else if value < 0.0 {
            // A negative x/y is legal on some multi-monitor setups, but a
            // negative width/height is never meaningful.
            let severity = if field.ends_with("width") || field.ends_with("height") {
                LintSeverity::Error
            } else {
                LintSeverity::Warning
            };
            out.push(ConfigLint::new(
                field,
                severity,
                format!("{field} = {value} is negative; the window may not be visible"),
            ));
        }
    }

    if b.width.is_finite() && (b.width < MIN_WINDOW_DIM || b.width > MAX_WINDOW_DIM) {
        out.push(ConfigLint::new(
            "window_bounds.width",
            LintSeverity::Warning,
            format!(
                "window_bounds.width = {} is outside the expected range [{}, {}]; \
                 it may be clamped by the platform",
                b.width, MIN_WINDOW_DIM, MAX_WINDOW_DIM
            ),
        ));
    }
    if b.height.is_finite() && (b.height < MIN_WINDOW_DIM || b.height > MAX_WINDOW_DIM) {
        out.push(ConfigLint::new(
            "window_bounds.height",
            LintSeverity::Warning,
            format!(
                "window_bounds.height = {} is outside the expected range [{}, {}]; \
                 it may be clamped by the platform",
                b.height, MIN_WINDOW_DIM, MAX_WINDOW_DIM
            ),
        ));
    }

    // Complements the per-dimension ceiling: two legal-but-large dimensions.
    if b.width.is_finite()
        && b.height.is_finite()
        && b.width > 0.0
        && b.height > 0.0
        && (b.width * b.height) > (MAX_WINDOW_DIM * MAX_WINDOW_DIM)
    {
        out.push(ConfigLint::new(
            "window_bounds",
            LintSeverity::Warning,
            format!(
                "window area {}x{} is implausibly large; likely a corrupted value",
                b.width, b.height
            ),
        ));
    }

    out
}

fn validate_update_channel(channel: &str) -> Vec<ConfigLint> {
    if channel.is_empty() {
        return vec![ConfigLint::new(
            "update_channel",
            LintSeverity::Warning,
            "update_channel is empty; the updater will default to \"stable\"",
        )];
    }
    if !KNOWN_UPDATE_CHANNELS.contains(&channel) {
        return vec![ConfigLint::new(
            "update_channel",
            LintSeverity::Warning,
            format!(
                "update_channel = {channel:?} is not recognised (expected one of {:?}); \
                 the updater will fall back to \"stable\"",
                KNOWN_UPDATE_CHANNELS
            ),
        )];
    }
    Vec::new()
}

fn validate_locale(locale: &str) -> Vec<ConfigLint> {
    if locale.is_empty() {
        return vec![ConfigLint::new(
            "locale",
            LintSeverity::Warning,
            format!("locale is empty; falling back to {LOCALE_EN:?}"),
        )];
    }
    if !KNOWN_LOCALES.contains(&locale) {
        return vec![ConfigLint::new(
            "locale",
            LintSeverity::Warning,
            format!(
                "locale = {locale:?} has no bundled translations (expected one of {:?}); \
                 falling back to {LOCALE_EN:?}",
                KNOWN_LOCALES
            ),
        )];
    }
    Vec::new()
}

/// `theme` must be non-empty; registry existence needs GPUI context this
/// pure-data tier deliberately avoids (the runtime handles unknown names).
fn validate_theme(theme: &str) -> Vec<ConfigLint> {
    if theme.trim().is_empty() {
        return vec![ConfigLint::new(
            "theme",
            LintSeverity::Warning,
            "theme is empty or blank; the default theme will be used",
        )];
    }
    Vec::new()
}

fn validate_granted_permissions(granted: &HashSet<String>) -> Vec<ConfigLint> {
    let known: HashSet<&str> = KNOWN_PERMISSIONS.iter().copied().collect();
    let mut out = Vec::new();
    // Sort for deterministic lint ordering (HashSet iteration is randomized).
    let mut unknown: Vec<&String> = granted
        .iter()
        .filter(|p| !known.contains(p.as_str()))
        .collect();
    unknown.sort();
    for perm in unknown {
        out.push(ConfigLint::new(
            format!("granted_permissions.{perm}"),
            LintSeverity::Info,
            format!(
                "granted permission {perm:?} is not recognised by this build \
                 (known: {:?}); it will be ignored",
                KNOWN_PERMISSIONS
            ),
        ));
    }
    out
}

#[cfg(test)]
#[path = "config_validation.tests.rs"]
mod config_validation_tests;
