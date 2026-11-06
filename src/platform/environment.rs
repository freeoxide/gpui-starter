//! Linux / sandbox environment detection.

/// Whether the process runs inside Flatpak, Snap, or similar. Reimplemented
/// instead of `ashpd::is_sandboxed()` so it exists with the portal feature off.
pub fn is_sandboxed() -> bool {
    std::env::var_os("FLATPAK_ID").is_some()
        || std::path::Path::new("/.flatpak-info").exists()
        || std::env::var_os("SNAP").is_some()
}
