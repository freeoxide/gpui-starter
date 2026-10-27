use std::collections::VecDeque;
use std::time::Duration;

use gpui_kit::{App, Context, Entity};

/// Clears sysinfo's MINIMUM_CPU_UPDATE_INTERVAL (200 ms) on every native
/// platform while keeping the refresh bounded at the spec's 1 Hz.
const INTERVAL: Duration = Duration::from_millis(1000);
const MAX_SAMPLES: usize = 120;
#[cfg(any(test, target_family = "wasm"))]
const FIXTURE_TOTAL_BYTES: u64 = 16 * 1024 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Sample {
    pub seq: u64,
    pub cpu_percent: f32,
    pub mem_used_bytes: u64,
    pub mem_total_bytes: u64,
}

pub(crate) struct Sampler {
    seq: u64,
    inner: Inner,
}

enum Inner {
    #[cfg(not(target_family = "wasm"))]
    System(sysinfo::System),
    #[cfg(any(test, target_family = "wasm"))]
    Fixture { phase: f64 },
}

impl Sampler {
    #[cfg(not(target_family = "wasm"))]
    fn system() -> Self {
        let mut system = sysinfo::System::new();
        // Prime usage accounting: the first read after construction is 0 until
        // two refreshes clear MINIMUM_CPU_UPDATE_INTERVAL.
        system.refresh_cpu_usage();
        Self {
            seq: 0,
            inner: Inner::System(system),
        }
    }

    #[cfg(any(test, target_family = "wasm"))]
    fn fixture() -> Self {
        Self {
            seq: 0,
            inner: Inner::Fixture { phase: 0. },
        }
    }

    pub(crate) fn sample(&mut self) -> Sample {
        let seq = self.seq;
        self.seq += 1;
        match &mut self.inner {
            #[cfg(not(target_family = "wasm"))]
            Inner::System(system) => {
                system.refresh_cpu_usage();
                system.refresh_memory();
                Sample {
                    seq,
                    cpu_percent: system.global_cpu_usage(),
                    mem_used_bytes: system.used_memory(),
                    mem_total_bytes: system.total_memory(),
                }
            }
            #[cfg(any(test, target_family = "wasm"))]
            Inner::Fixture { phase } => {
                let current = *phase;
                let cpu = (25. + 20. * current.sin()).clamp(0., 100.);
                let total = FIXTURE_TOTAL_BYTES as f64;
                let used = total * (0.55 + 0.08 * (current * 0.5).sin());
                *phase = current + 0.35;
                Sample {
                    seq,
                    cpu_percent: cpu as f32,
                    mem_used_bytes: used as u64,
                    mem_total_bytes: FIXTURE_TOTAL_BYTES,
                }
            }
        }
    }
}

pub(crate) struct MetricsState {
    samples: VecDeque<Sample>,
    sampler: Sampler,
    paused: bool,
}

impl MetricsState {
    pub(crate) fn new() -> Self {
        #[cfg(not(target_family = "wasm"))]
        let sampler = Sampler::system();
        #[cfg(target_family = "wasm")]
        let sampler = Sampler::fixture();
        Self {
            samples: VecDeque::new(),
            sampler,
            paused: false,
        }
    }

    /// Spawn the 1 Hz sampling loop. It exits when the entity is released, so
    /// closing the popover or sheet stops all sampling.
    pub(crate) fn start(this: &Entity<Self>, cx: &mut App) {
        let this = this.downgrade();
        cx.spawn(async move |cx| {
            loop {
                cx.background_executor().timer(INTERVAL).await;
                if this.update(cx, |this, cx| this.tick(cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
    }

    pub(crate) fn tick(&mut self, cx: &mut Context<Self>) {
        if self.paused {
            return;
        }
        let sample = self.sampler.sample();
        self.samples.push_back(sample);
        if self.samples.len() > MAX_SAMPLES {
            self.samples.pop_front();
        }
        cx.notify();
    }

    pub(crate) fn set_paused(&mut self, paused: bool, cx: &mut Context<Self>) {
        self.paused = paused;
        cx.notify();
    }

    pub(crate) fn samples(&self) -> &VecDeque<Sample> {
        &self.samples
    }

    pub(crate) fn is_paused(&self) -> bool {
        self.paused
    }
}

#[cfg(test)]
#[path = "metrics.test.rs"]
mod metrics_test;
