#[cfg(not(target_family = "wasm"))]
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

// Serialize is load-bearing: tests/snapshot_tests.rs pins these as yaml.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum UpdateStatus {
    Idle,
    Checking,
    UpToDate,
    Available { version: String, notes: String },
    Error(String),
}

impl Default for UpdateStatus {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct UpdateSnapshot {
    pub status: UpdateStatus,
    pub current_version: String,
    pub last_check: Option<String>,
    pub update_channel: String,
    pub check_retry_count: u32,
    // In-memory cache so a `check_for_updates` within one interval skips the
    // network fetch. Skipped from serialization — it does not outlive the run.
    #[serde(skip)]
    pub cached_manifest: Option<UpdateManifest>,
}

impl gpui_kit::Global for UpdateSnapshot {}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UpdateManifest {
    pub version: String,
    #[serde(default)]
    pub release_notes: String,
    #[serde(default)]
    pub platforms: std::collections::HashMap<String, PlatformAsset>,
}

// Wire-format struct; no field is read yet, but the manifest parse keeps the
// shape so a future consumer sees the real payload.
#[cfg_attr(target_family = "wasm", allow(dead_code))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PlatformAsset {
    pub url: String,
    #[serde(default)]
    pub signature: String,
    #[serde(default)]
    pub size: u64,
}

pub(crate) const PLACEHOLDER_MANIFEST_URL: &str = "https://releases.example.com/manifest.json";

pub(crate) const DEFAULT_MANIFEST_URL: &str = match option_env!("GPUI_UPDATE_MANIFEST_URL") {
    Some(url) => url,
    None => PLACEHOLDER_MANIFEST_URL,
};

// The items below exist only for the native check/swap paths; on wasm they
// would compile as dead code.

/// Ed25519 public key matching `UPDATE_SIGNING_KEY` in CI; swap it to deploy.
#[cfg(not(target_family = "wasm"))]
pub(crate) const UPDATER_PUBLIC_KEY: &[u8; 32] = include_bytes!("../updater_public_key.bin");

pub(crate) const MAX_UPDATE_RETRIES: u32 = 3;
pub(crate) const RETRY_BASE_DELAY_SECS: u64 = 30;
#[cfg(not(target_family = "wasm"))]
pub(crate) const STARTUP_CHECK_DELAY_SECS: u64 = 5;
#[cfg(not(target_family = "wasm"))]
pub(crate) const PERIODIC_CHECK_INTERVAL_SECS: u64 = 4 * 60 * 60;

// Native-only: file paths panic at runtime on wasm32-unknown-unknown, and
// the only caller chain (check_pending_swap) is a wasm no-op.

/// App-owned downloads + swap directory under the user's data dir, so the
/// write target is not a world-shared, attacker-creatable temp path.
/// `None` (no data dir / cannot create it) fails closed.
#[cfg(not(target_family = "wasm"))]
pub(crate) fn updates_dir() -> Option<PathBuf> {
    let dir = crate::platform::filesystem::paths::project_dirs()?
        .data_dir()
        .join("updates");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

#[cfg(not(target_family = "wasm"))]
pub(crate) fn pending_swap_path() -> Option<PathBuf> {
    Some(updates_dir()?.join("pending-swap.json"))
}

pub(crate) fn current_app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
