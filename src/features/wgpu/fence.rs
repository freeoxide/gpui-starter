//! Deadline fence around blocking wgpu calls. A wedged GPU driver hangs
//! below wgpu's own 30-second waits, so every fenced call runs on a
//! dedicated OS thread bounded here. A missed deadline detaches that
//! thread (it cannot be cancelled; at most one leaked thread per poisoned
//! engine) and trips a poison latch that refuses further fenced work.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::thread;
use std::time::Duration;

use anyhow::Result;

/// Overall budget for one fenced render+readback. wgpu's internal waits
/// allow 30s each; the Metal wedge sits below them, so this trips first.
pub(in crate::features::wgpu) const FRAME_DEADLINE: Duration = Duration::from_secs(5);

/// Budget for adapter/device init before the caller gives up and
/// surfaces a timeout error.
pub(in crate::features::wgpu) const INIT_DEADLINE: Duration = Duration::from_secs(15);

/// Why a fenced call failed. `Deadline`, `Poisoned`, and `InitTimeout`
/// are the wedge fence tripping; the rest are ordinary worker failures.
#[derive(Debug, thiserror::Error)]
pub enum FenceError {
    /// The call outlived its overall deadline. The worker thread is
    /// detached and the engine poisoned; rebuild the engine to retry.
    #[error(
        "wgpu render missed its {secs}s deadline; engine poisoned (wedged render thread detached)"
    )]
    Deadline { secs: u64 },

    /// The engine was poisoned by an earlier missed render deadline.
    #[error(
        "wgpu engine poisoned by an earlier missed render deadline; rebuild the engine to retry"
    )]
    Poisoned,

    /// Engine init outlived its deadline; the init thread is detached.
    #[error("wgpu adapter/device init timed out after {secs}s")]
    InitTimeout { secs: u64 },

    /// The fenced work itself failed.
    #[error("{work}")]
    Work { work: anyhow::Error },

    /// The worker thread could not be spawned.
    #[error("spawning wgpu worker thread: {reason}")]
    Spawn { reason: String },

    /// The worker thread panicked.
    #[error("wgpu worker panicked: {message}")]
    Panic { message: String },
}

impl FenceError {
    /// Flattens into anyhow; work errors keep their original chain.
    pub(in crate::features::wgpu) fn into_anyhow(self) -> anyhow::Error {
        match self {
            Self::Work { work } => work,
            other => anyhow::Error::new(other),
        }
    }
}

/// Runs `work` on a dedicated OS thread, waiting at most `deadline` for
/// its result. On timeout the thread is left detached: a wedged driver
/// cannot be cancelled, so callers must poison their state and refuse
/// further work rather than wait. Blocks the caller; never run from a
/// UI thread.
pub(in crate::features::wgpu) fn run_bounded<T, F>(
    name: &'static str,
    deadline: Duration,
    work: F,
) -> std::result::Result<T, FenceError>
where
    F: FnOnce() -> Result<T> + Send + 'static,
    T: Send + 'static,
{
    let (tx, rx) = mpsc::channel();
    let spawned = thread::Builder::new()
        .name(name.into())
        .spawn(move || {
            let _ = tx.send(work());
        })
        .map_err(|err| FenceError::Spawn {
            reason: err.to_string(),
        })?;
    match rx.recv_timeout(deadline) {
        Ok(worked) => {
            // Result already delivered; join only reaps the finished thread.
            let _ = spawned.join();
            worked.map_err(|work| FenceError::Work { work })
        }
        Err(RecvTimeoutError::Timeout) => Err(FenceError::Deadline {
            secs: deadline.as_secs(),
        }),
        // Only a panic drops `tx` without sending first.
        Err(RecvTimeoutError::Disconnected) => Err(FenceError::Panic {
            message: panic_message(spawned.join()),
        }),
    }
}

fn panic_message(joined: thread::Result<()>) -> String {
    match joined {
        Ok(()) => String::from("worker exited without reporting"),
        Err(payload) => {
            if let Some(msg) = payload.downcast_ref::<&'static str>() {
                (*msg).to_string()
            } else if let Some(msg) = payload.downcast_ref::<String>() {
                msg.clone()
            } else {
                String::from("unknown panic payload")
            }
        }
    }
}

/// The engine's render fence: a one-shot poison latch plus the fenced
/// runner. The first call to miss its deadline trips the latch; every
/// later call refuses work without running it.
pub(in crate::features::wgpu) struct EngineFence {
    tripped: AtomicBool,
}

impl EngineFence {
    pub(in crate::features::wgpu) const fn new() -> Self {
        Self {
            tripped: AtomicBool::new(false),
        }
    }

    /// True once a fenced render missed its deadline.
    pub(in crate::features::wgpu) fn poisoned(&self) -> bool {
        self.tripped.load(Ordering::Acquire)
    }

    /// Runs `work` on a dedicated thread bounded by `deadline`, refusing
    /// work once poisoned and poisoning on a missed deadline. The wedged
    /// thread stays detached; that leak is bounded by the latch refusing
    /// every later call.
    pub(in crate::features::wgpu) fn run<T, F>(
        &self,
        deadline: Duration,
        work: F,
    ) -> std::result::Result<T, FenceError>
    where
        F: FnOnce() -> Result<T> + Send + 'static,
        T: Send + 'static,
    {
        if self.poisoned() {
            return Err(FenceError::Poisoned);
        }
        match run_bounded("wgpu-render", deadline, work) {
            Err(err @ FenceError::Deadline { .. }) => {
                self.tripped.store(true, Ordering::Release);
                Err(err)
            }
            other => other,
        }
    }
}

#[cfg(test)]
#[path = "fence.test.rs"]
mod fence_test;
