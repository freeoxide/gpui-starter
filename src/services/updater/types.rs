#[cfg(not(target_family = "wasm"))]
use std::path::PathBuf;

use gpui::actions;
use serde::{Deserialize, Serialize};

actions!(updater, [CheckForUpdates]);

// Serialize is load-bearing: tests/snapshot_tests.rs pins these as yaml.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub enum UpdateStatus {
    Idle,
    Checking,
    UpToDate,
    Available {
        version: String,
        notes: String,
    },
    Downloading {
        progress: u32, // 0–100
    },
    Downloaded {
        version: String,
        path: String,
    },
    ReadyToInstall,
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
    pub download_retry_count: u32,
    // In-memory caches so `download_update` can reuse the manifest/asset that
    // `check_for_updates` already fetched, instead of hitting the network twice.
    // Skipped from serialization — they are ephemeral and do not outlive the run.
    #[serde(skip)]
    pub cached_manifest: Option<UpdateManifest>,
    #[serde(skip)]
    pub cached_asset: Option<PlatformAsset>,
}

impl gpui::Global for UpdateSnapshot {}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UpdateManifest {
    pub version: String,
    #[serde(default)]
    pub release_notes: String,
    #[serde(default)]
    pub platforms: std::collections::HashMap<String, PlatformAsset>,
}

// Fields are read only by the native download path; wasm keeps the parse shape.
#[cfg_attr(target_family = "wasm", allow(dead_code))]
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct PlatformAsset {
    pub url: String,
    #[serde(default)]
    pub signature: String,
    #[serde(default)]
    pub size: u64,
}

pub(crate) const DEFAULT_MANIFEST_URL: &str = match option_env!("GPUI_UPDATE_MANIFEST_URL") {
    Some(url) => url,
    None => "https://releases.example.com/manifest.json",
};

/// Ed25519 public key matching `UPDATE_SIGNING_KEY` in CI; swap it to deploy.
pub(crate) const UPDATER_PUBLIC_KEY: &[u8; 32] = include_bytes!("../updater_public_key.bin");

pub(crate) const MAX_UPDATE_RETRIES: u32 = 3;
pub(crate) const RETRY_BASE_DELAY_SECS: u64 = 30;
pub(crate) const STARTUP_CHECK_DELAY_SECS: u64 = 5;
pub(crate) const PERIODIC_CHECK_INTERVAL_SECS: u64 = 4 * 60 * 60;

pub(crate) fn platform_key() -> String {
    let os = if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "unknown"
    };
    let arch = std::env::consts::ARCH;
    format!("{os}-{arch}")
}

// Native-only: file paths panic at runtime on wasm32-unknown-unknown, and
// both callers (apply_update / check_pending_swap) are wasm no-ops.

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
