//! Self-reload support for native targets: a flag set via [`request_reload`],
//! polled post-shutdown to relaunch the same binary with the original argv —
//! exec() in place on Unix, spawn-and-exit on Windows.

#![cfg(not(target_family = "wasm"))]

use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::{Context, Result};
use tracing::{error, info};

const LOG: &str = "gpui_starter::reload";

/// Set by [`request_reload`], polled by the host after GPUI has shut down.
static RELOAD_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Set child-only by the Windows spawner so preflight retries the instance
/// lock while the old process dies.
pub const RESTART_HANDOFF_ENV: &str = "GPUI_STARTER_RESTART_HANDOFF";

/// Request an in-place relaunch after shutdown; call [`perform_reload`] only
/// once GPUI has fully torn down.
pub fn request_reload() {
    RELOAD_REQUESTED.store(true, Ordering::SeqCst);
    info!(target: LOG, "reload requested");
}

pub fn is_reload_requested() -> bool {
    RELOAD_REQUESTED.load(Ordering::SeqCst)
}

/// Relaunch after GPUI shutdown: exec in place on Unix, spawn-and-return on
/// Windows.
pub fn perform_reload() -> Result<()> {
    #[cfg(unix)]
    {
        exec_reload()
    }
    #[cfg(windows)]
    {
        spawn_reload()
    }
}

/// Spawn the replacement with the original argv, then return so the caller
/// can exit; the child retries the instance lock while this process dies.
#[cfg(windows)]
fn spawn_reload() -> Result<()> {
    let exe = std::env::current_exe().context("failed to resolve current executable path")?;
    let mut cmd = std::process::Command::new(&exe);
    // Skip argv[0] (the program name) since Command::new already supplies
    // argv[0] from `exe`; args_os round-trips non-UTF8 argv where args() panics.
    cmd.args(std::env::args_os().skip(1));
    cmd.env(RESTART_HANDOFF_ENV, "1");

    let child = cmd.spawn().map_err(|err| {
        error!(target: LOG, error = %err, "reload spawn failed");
        err
    })?;
    info!(target: LOG, pid = child.id(), "spawned replacement process");
    drop(child); // releasing the handle does not kill the child
    Ok(())
}

/// Re-exec with the original argv after GPUI shutdown; never returns on
/// success, returns `Err` without panicking otherwise.
#[cfg(unix)]
fn exec_reload() -> Result<()> {
    use std::os::unix::process::CommandExt;

    info!(target: LOG, "executing in-place reload");

    let exe = std::env::current_exe().context("failed to resolve current executable path")?;

    // Skip argv[0] (the program name) since Command::new already supplies
    // argv[0] from `exe`.
    let mut cmd = std::process::Command::new(&exe);
    cmd.args(std::env::args().skip(1));

    let err = cmd.exec();

    // exec() returns the io::Error only when it fails — success never returns.
    error!(target: LOG, error = %err, "exec failed");
    Err(anyhow::anyhow!("failed to exec reload: {err}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_roundtrip() {
        // Reset to a known state; tests run concurrently on the same static,
        // but this process does not actually exec so a benign toggle is fine.
        RELOAD_REQUESTED.store(false, Ordering::SeqCst);
        assert!(!is_reload_requested());
        request_reload();
        assert!(is_reload_requested());
        RELOAD_REQUESTED.store(false, Ordering::SeqCst);
    }
}
