use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use super::{EngineFence, FenceError, run_bounded};

/// Blocks the worker thread forever: a stand-in for GPU work wedged
/// inside the driver, which this host cannot reproduce for real.
fn park_forever() -> ! {
    let (_tx, rx) = mpsc::channel::<()>();
    let _ = rx.recv();
    unreachable!("the live sender keeps recv blocking")
}

#[test]
fn run_bounded_returns_work_result_within_deadline() {
    let value = run_bounded("fence-test-fast", Duration::from_secs(5), || {
        anyhow::Ok(7u32)
    })
    .expect("fast work must succeed");
    assert_eq!(value, 7);
}

#[test]
fn run_bounded_surfaces_work_errors() {
    let err = run_bounded(
        "fence-test-err",
        Duration::from_secs(5),
        || -> anyhow::Result<()> { Err(anyhow::anyhow!("gpu said no")) },
    )
    .expect_err("failed work must surface");
    assert!(matches!(err, FenceError::Work { .. }));
    assert_eq!(err.to_string(), "gpu said no");
}

#[test]
fn run_bounded_reports_worker_panics() {
    let err = run_bounded(
        "fence-test-panic",
        Duration::from_secs(5),
        || -> anyhow::Result<()> { panic!("boom") },
    )
    .expect_err("panicked work must surface as an error");
    assert!(matches!(err, FenceError::Panic { .. }));
    assert!(err.to_string().contains("boom"));
}

#[test]
fn run_bounded_trips_deadline_on_parked_work() {
    let started = std::time::Instant::now();
    let err = run_bounded(
        "fence-test-parked",
        Duration::from_millis(200),
        || -> anyhow::Result<()> { park_forever() },
    )
    .expect_err("parked work must trip the deadline");
    assert!(matches!(err, FenceError::Deadline { secs: 0 }));
    assert!(err.to_string().contains("engine poisoned"));
    assert!(started.elapsed() >= Duration::from_millis(200));
}

#[test]
fn fence_poisons_after_missed_deadline_and_refuses_work() {
    let fence = EngineFence::new();
    let err = fence
        .run(Duration::from_millis(150), || -> anyhow::Result<()> {
            park_forever()
        })
        .expect_err("parked work must poison the fence");
    assert!(matches!(err, FenceError::Deadline { .. }));
    assert!(fence.poisoned());

    let ran = Arc::new(AtomicBool::new(false));
    let marker = ran.clone();
    let err = fence
        .run(Duration::from_secs(5), move || {
            marker.store(true, Ordering::Release);
            anyhow::Ok(())
        })
        .expect_err("a poisoned fence must refuse work");
    assert!(matches!(err, FenceError::Poisoned));
    assert!(!ran.load(Ordering::Acquire), "refused work must not run");
}

#[test]
fn fence_stays_healthy_across_successful_calls() {
    let fence = EngineFence::new();
    let first = fence
        .run(Duration::from_secs(5), || anyhow::Ok(1u32))
        .expect("first call must succeed");
    let second = fence
        .run(Duration::from_secs(5), || anyhow::Ok(2u32))
        .expect("second call must succeed");
    assert_eq!((first, second), (1, 2));
    assert!(!fence.poisoned());
}
