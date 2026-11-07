#[cfg(not(target_family = "wasm"))]
use std::path::{Path, PathBuf};

#[cfg(not(target_family = "wasm"))]
use super::types::*;
use gpui_kit::App;

/// Wasm: no filesystem — a pending-swap marker can never exist. No-op.
#[cfg(target_family = "wasm")]
pub fn check_pending_swap(_cx: &mut App) {}

#[cfg(not(target_family = "wasm"))]
pub fn check_pending_swap(cx: &mut App) {
    let Some(marker_path) = pending_swap_path() else {
        return;
    };
    if !marker_path.exists() {
        return;
    }

    tracing::info!(
        target: "gpui_starter::updater",
        path = %marker_path.display(),
        "pending swap marker found, attempting binary swap"
    );

    // Hash + Ed25519 verify plus the swap itself block for seconds; init.rs
    // calls this during startup, so none of it may run on the UI thread.
    cx.spawn(async move |cx| {
        let outcome = cx
            .background_executor()
            .spawn(async move { run_pending_swap(marker_path) })
            .await;
        cx.update(|cx| {
            if let Some(status) = outcome {
                super::set_status(status, cx);
            }
        });
    })
    .detach();
}

/// Runs on a background executor. `Some(status)` is applied on the UI thread
/// once the swap settles; `None` leaves the current status untouched.
#[cfg(not(target_family = "wasm"))]
pub(super) fn run_pending_swap(marker_path: PathBuf) -> Option<UpdateStatus> {
    let data = match std::fs::read_to_string(&marker_path) {
        Ok(d) => d,
        Err(err) => {
            tracing::error!(
                target: "gpui_starter::updater",
                error = %err,
                "failed to read pending swap marker"
            );
            return None;
        }
    };

    let pending: serde_json::Value = match serde_json::from_str(&data) {
        Ok(v) => v,
        Err(err) => {
            tracing::error!(
                target: "gpui_starter::updater",
                error = %err,
                "failed to parse pending swap marker"
            );
            // Corrupt marker: drop it so we don't retry indefinitely.
            let _ = std::fs::remove_file(&marker_path);
            return None;
        }
    };

    let source_path = match pending.get("source_path").and_then(|v| v.as_str()) {
        Some(p) => PathBuf::from(p),
        None => {
            tracing::error!(
                target: "gpui_starter::updater",
                "pending swap marker missing source_path"
            );
            let _ = std::fs::remove_file(&marker_path);
            return None;
        }
    };

    let version = pending
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    // Downgrade protection: a validly-signed marker for anything but a
    // strictly newer version must never swap in.
    let running_version = current_app_version();
    let marker_is_newer = matches!(
        (
            semver::Version::parse(version),
            semver::Version::parse(&running_version),
        ),
        (Ok(marker), Ok(running)) if marker > running
    );
    if !marker_is_newer {
        tracing::warn!(
            target: "gpui_starter::updater",
            marker_version = %version,
            running_version = %running_version,
            "pending swap marker is not newer than the running app — removing marker"
        );
        let _ = std::fs::remove_file(&marker_path);
        return None;
    }

    // Fail closed: the swap re-verifies the manifest signature, so a tampered
    // file can never replace the executable; unsigned markers are refused.
    let signature = pending
        .get("signature")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    if signature.is_empty() {
        tracing::error!(
            target: "gpui_starter::updater",
            "pending swap marker has no signature — refusing unverified swap"
        );
        let _ = std::fs::remove_file(&marker_path);
        return Some(UpdateStatus::Error(
            "pending swap refused: no signature in marker".to_string(),
        ));
    }
    if let Err(err) = verify_ed25519_signature(&source_path, signature, UPDATER_PUBLIC_KEY) {
        tracing::error!(
            target: "gpui_starter::updater",
            path = %source_path.display(),
            error = %err,
            "pending swap source failed signature re-verification — removing marker"
        );
        let _ = std::fs::remove_file(&marker_path);
        return Some(UpdateStatus::Error(format!("pending swap refused: {err}")));
    }

    let current_exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(err) => {
            tracing::error!(
                target: "gpui_starter::updater",
                error = %err,
                "failed to determine current executable path"
            );
            return None;
        }
    };

    match swap_installed_app(&source_path, &current_exe) {
        Ok(()) => {
            tracing::info!(
                target: "gpui_starter::updater",
                version = %version,
                "pending swap applied successfully"
            );
            let _ = std::fs::remove_file(&marker_path);
            Some(UpdateStatus::Idle)
        }
        Err(err) => {
            tracing::error!(
                target: "gpui_starter::updater",
                error = %err,
                "pending swap failed"
            );
            // Marker stays so the user/admin can investigate.
            Some(UpdateStatus::Error(format!("swap failed: {err}")))
        }
    }
}

#[cfg(all(not(target_family = "wasm"), target_os = "macos"))]
fn swap_installed_app(source_path: &Path, current_exe: &Path) -> Result<(), String> {
    // Inside a .app bundle: swap the whole bundle when the source is one,
    // else move the staged binary over the executable in place.
    if let Some(bundle_path) = current_exe
        .ancestors()
        .find(|a| a.extension().is_some_and(|ext| ext == "app"))
    {
        let source_is_bundle =
            source_path.extension().is_some_and(|ext| ext == "app") || source_path.is_dir();
        if source_is_bundle {
            return swap_bundle(source_path, bundle_path);
        }
        return mv(source_path, current_exe);
    }
    rename_binary(source_path, current_exe)
}

#[cfg(all(not(target_family = "wasm"), not(target_os = "macos")))]
fn swap_installed_app(source_path: &Path, current_exe: &Path) -> Result<(), String> {
    rename_binary(source_path, current_exe)
}

#[cfg(not(target_family = "wasm"))]
fn rename_binary(source_path: &Path, dest: &Path) -> Result<(), String> {
    std::fs::rename(source_path, dest).map_err(|e| format!("failed to rename binary: {e}"))
}

/// Stage aside-then-move so a failed `mv` never uninstalls the app; the
/// `.old` copy is cleared by the next swap, not here.
#[cfg(target_os = "macos")]
fn swap_bundle(source_bundle: &Path, bundle_path: &Path) -> Result<(), String> {
    tracing::info!(
        target: "gpui_starter::updater",
        source = %source_bundle.display(),
        dest = %bundle_path.display(),
        "swapping .app bundle"
    );
    let mut aside_name = bundle_path.as_os_str().to_os_string();
    aside_name.push(".old");
    let aside = PathBuf::from(aside_name);
    if aside.exists() {
        std::fs::remove_dir_all(&aside)
            .map_err(|e| format!("failed to clear stale aside bundle: {e}"))?;
    }
    if !bundle_path.exists() {
        return mv(source_bundle, bundle_path);
    }
    std::fs::rename(bundle_path, &aside)
        .map_err(|e| format!("failed to stage old bundle aside: {e}"))?;
    if let Err(err) = mv(source_bundle, bundle_path) {
        if let Err(rollback) = std::fs::rename(&aside, bundle_path) {
            return Err(format!(
                "{err}; restoring the old bundle failed: {rollback}"
            ));
        }
        return Err(err);
    }
    Ok(())
}

/// `mv` instead of fs::rename because the staged bundle can sit on another
/// volume; `.output()` is required because `.status()` is Ok for any exit code.
#[cfg(target_os = "macos")]
fn mv(source: &Path, dest: &Path) -> Result<(), String> {
    let output = std::process::Command::new("mv")
        .arg("-f")
        .arg(source)
        .arg(dest)
        .output()
        .map_err(|e| format!("failed to spawn mv: {e}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "mv {} -> {} exited with {}: {}",
            source.display(),
            dest.display(),
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

/// Verify the base64 Ed25519 `signature_b64` over the SHA-256 of the file at
/// `file_path` against `public_key`. Synchronous; dispatch on a background
/// executor for large files (see `check_pending_swap`).
#[cfg(not(target_family = "wasm"))]
pub(super) fn verify_ed25519_signature(
    file_path: &Path,
    signature_b64: &str,
    public_key: &[u8; 32],
) -> Result<(), String> {
    use base64::Engine as _;
    use ed25519_dalek::{Signature, Verifier as _, VerifyingKey};
    use sha2::Digest;
    use std::io::Read as _;

    let sig_bytes = base64::engine::general_purpose::STANDARD
        .decode(signature_b64)
        .map_err(|e| format!("failed to decode base64 signature: {e}"))?;
    let sig_len = sig_bytes.len();
    let sig_array: [u8; 64] = sig_bytes
        .try_into()
        .map_err(|_| format!("invalid signature length: expected 64 bytes, got {sig_len}"))?;
    let signature = Signature::from_bytes(&sig_array);

    let verifying_key = VerifyingKey::from_bytes(public_key)
        .map_err(|e| format!("invalid updater public key: {e}"))?;

    let mut file = std::fs::File::open(file_path)
        .map_err(|e| format!("failed to open downloaded file for signature check: {e}"))?;
    let mut hasher = sha2::Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| format!("failed to read downloaded file for signature check: {e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let hash = hasher.finalize();

    verifying_key
        .verify(&hash, &signature)
        .map_err(|e| format!("Ed25519 signature verification failed: {e}"))?;

    Ok(())
}
