//! Native self-relaunch: [`request_reload`] arms a flag the host polls after
//! GPUI shutdown; [`perform_reload`] execs (Unix) or spawns (Windows) the same
//! argv. RCA §5.4 rate limits keep a crashing build from restart-looping.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use tracing::{error, info};

const LOG: &str = "gpui_starter::reload";

/// Set by [`request_reload`], polled by the host after GPUI has shut down.
static RELOAD_REQUESTED: AtomicBool = AtomicBool::new(false);

/// Set child-only by the Windows spawner so preflight retries the instance
/// lock while the old process dies.
pub const RESTART_HANDOFF_ENV: &str = "GPUI_STARTER_RESTART_HANDOFF";

/// Restart timestamps (unix millis, comma-separated), set child-only: statics
/// reset across exec, so this env var is the child's only memory of the chain.
const RESTART_HISTORY_ENV: &str = "GPUI_STARTER_RESTART_HISTORY";

const RESTART_WINDOW_MS: u64 = 60_000;
const MAX_RESTARTS_PER_WINDOW: usize = 3;
const BACKOFF_BASE_MS: u64 = 1_000;
/// Bounds the sleep if MAX_RESTARTS_PER_WINDOW ever grows past the shift width.
const BACKOFF_CAP_MS: u64 = 10_000;

/// What [`perform_reload`] should do given the recent restart history.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelaunchVerdict {
    /// The relaunch may proceed after waiting out `delay_ms`.
    Proceed { delay_ms: u64 },
    /// The restart budget inside the window is spent: no relaunch (safe mode).
    SafeMode,
}

/// Decide a relaunch from an explicit clock so the policy is testable:
/// first restart immediate, then exponential backoff, and a hard cap of
/// [`MAX_RESTARTS_PER_WINDOW`] inside [`RESTART_WINDOW_MS`].
pub fn relaunch_verdict(now_ms: u64, prior_restarts_ms: &[u64]) -> RelaunchVerdict {
    let recent = prior_restarts_ms
        .iter()
        .filter(|&&started| now_ms.saturating_sub(started) < RESTART_WINDOW_MS)
        .count();
    if recent >= MAX_RESTARTS_PER_WINDOW {
        return RelaunchVerdict::SafeMode;
    }
    let delay_ms = if recent == 0 {
        0
    } else {
        (BACKOFF_BASE_MS << (recent - 1).min(10)).min(BACKOFF_CAP_MS)
    };
    RelaunchVerdict::Proceed { delay_ms }
}

/// [`relaunch_verdict`] against the real clock and the inherited history.
pub fn relaunch_verdict_now() -> RelaunchVerdict {
    relaunch_verdict(now_unix_ms(), &inherited_restarts())
}

/// Arm a relaunch; the host calls [`perform_reload`] once GPUI is torn down,
/// and that re-checks the budget, so arming alone cannot bypass the cap.
pub fn request_reload() {
    RELOAD_REQUESTED.store(true, Ordering::SeqCst);
    info!(target: LOG, "reload requested");
}

pub fn is_reload_requested() -> bool {
    RELOAD_REQUESTED.load(Ordering::SeqCst)
}

/// Relaunch after GPUI shutdown: sleeps out the backoff, then execs (Unix) or
/// spawns and returns (Windows); safe mode returns Ok without relaunching.
pub fn perform_reload() -> Result<()> {
    let history = inherited_restarts();
    match relaunch_verdict(now_unix_ms(), &history) {
        RelaunchVerdict::SafeMode => {
            // A capped relaunch stays down rather than feeding a restart
            // loop; stderr + recent-errors carry why (tracing is torn down).
            let reason = format!(
                "relaunch refused: {} restarts within {} s (safe mode)",
                MAX_RESTARTS_PER_WINDOW,
                RESTART_WINDOW_MS / 1000
            );
            eprintln!("[gpui_starter::reload] {reason}");
            crate::lifecycle::track_recent_error(reason);
            Ok(())
        }
        RelaunchVerdict::Proceed { delay_ms } => {
            if delay_ms > 0 {
                info!(target: LOG, delay_ms, "backing off before relaunch");
                std::thread::sleep(Duration::from_millis(delay_ms));
            }
            // Record the exec time (post-sleep), not the verdict time.
            let updated = history_with(now_unix_ms(), &history);
            #[cfg(unix)]
            {
                exec_reload(&updated)
            }
            #[cfg(windows)]
            {
                spawn_reload(&updated)
            }
        }
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as u64)
        .unwrap_or(0)
}

fn parse_history(raw: &str) -> Vec<u64> {
    raw.split(',')
        .filter_map(|entry| entry.trim().parse().ok())
        .collect()
}

/// Pure half of the env round-trip: append `now_ms`, dropping entries the
/// window would expire anyway so the inherited value stays bounded.
fn history_with(now_ms: u64, prior_restarts_ms: &[u64]) -> String {
    let mut history: Vec<u64> = prior_restarts_ms
        .iter()
        .copied()
        .filter(|&started| now_ms.saturating_sub(started) < RESTART_WINDOW_MS)
        .collect();
    history.push(now_ms);
    history
        .iter()
        .map(|started| started.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn inherited_restarts() -> Vec<u64> {
    std::env::var(RESTART_HISTORY_ENV)
        .map(|raw| parse_history(&raw))
        .unwrap_or_default()
}

/// Spawn the replacement with the original argv, then return so the caller
/// can exit; the child retries the instance lock while this process dies.
#[cfg(windows)]
fn spawn_reload(restart_history: &str) -> Result<()> {
    let exe = std::env::current_exe().context("failed to resolve current executable path")?;
    let mut cmd = std::process::Command::new(exe);
    // Skip argv[0] (the program name) since Command::new already supplies
    // argv[0] from `exe`; args_os round-trips non-UTF8 argv where args() panics.
    cmd.args(std::env::args_os().skip(1));
    cmd.env(RESTART_HANDOFF_ENV, "1");
    cmd.env(RESTART_HISTORY_ENV, restart_history);

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
fn exec_reload(restart_history: &str) -> Result<()> {
    use std::os::unix::process::CommandExt;

    info!(target: LOG, "executing in-place reload");

    let exe = std::env::current_exe().context("failed to resolve current executable path")?;

    // Skip argv[0] (the program name) since Command::new already supplies
    // argv[0] from `exe`; args_os round-trips non-UTF8 argv where args() panics.
    let mut cmd = std::process::Command::new(exe);
    cmd.args(std::env::args_os().skip(1));
    cmd.env(RESTART_HISTORY_ENV, restart_history);

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

    #[test]
    fn first_relaunch_after_a_quiet_start_is_immediate() {
        let verdict = relaunch_verdict(10_000_000, &[]);
        assert_eq!(verdict, RelaunchVerdict::Proceed { delay_ms: 0 });
    }

    #[test]
    fn backoff_delay_escalates_with_each_recent_relaunch() {
        let now = 10_000_000;
        let one_back = now - 1_000;
        assert_eq!(
            relaunch_verdict(now, &[one_back]),
            RelaunchVerdict::Proceed {
                delay_ms: BACKOFF_BASE_MS
            }
        );
        let two_back = now - 2_000;
        assert_eq!(
            relaunch_verdict(now, &[two_back, one_back]),
            RelaunchVerdict::Proceed {
                delay_ms: BACKOFF_BASE_MS * 2
            }
        );
    }

    #[test]
    fn third_relaunch_in_the_window_refuses_to_relaunch_again() {
        let now = 10_000_000;
        let history = [now - 1_000, now - 2_000, now - 3_000];
        assert_eq!(relaunch_verdict(now, &history), RelaunchVerdict::SafeMode);
    }

    #[test]
    fn budget_resets_once_the_window_has_gone_quiet() {
        let now = 10_000_000;
        let quiet_start = now - 5 * RESTART_WINDOW_MS;
        let history = [quiet_start, quiet_start - 1, quiet_start - 2];
        assert_eq!(
            relaunch_verdict(now, &history),
            RelaunchVerdict::Proceed { delay_ms: 0 }
        );
    }

    #[test]
    fn history_round_trip_drops_entries_the_window_would_expire() {
        let now = 10_000_000;
        let raw = history_with(now, &[now - 5 * RESTART_WINDOW_MS, now - 1_000]);
        assert_eq!(parse_history(&raw), vec![now - 1_000, now]);
    }

    #[test]
    fn history_parsing_skips_malformed_entries() {
        assert_eq!(parse_history("12, junk ,34"), vec![12, 34]);
        assert_eq!(parse_history(""), Vec::<u64>::new());
    }
}
