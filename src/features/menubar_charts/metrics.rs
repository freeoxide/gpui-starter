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
    // None while the background task owns the sampler mid-tick.
    sampler: Option<Sampler>,
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
            sampler: Some(sampler),
            paused: false,
        }
    }

    /// Spawn the 1 Hz sampling loop. It exits when the entity is released, so
    /// closing the popover or sheet stops all sampling.
    pub(crate) fn start(this: &Entity<Self>, cx: &mut App) {
        let this = this.downgrade();
        cx.spawn(async move |cx| {
            let executor = cx.background_executor().clone();
            loop {
                executor.timer(INTERVAL).await;
                // Sampling reads /proc, so it never runs on the main thread;
                // the sampler moves in and out of the task to keep its state.
                let taken = this.update(cx, |this, _| {
                    if this.paused {
                        None
                    } else {
                        this.sampler.take()
                    }
                });
                let (sampler, sample) = match taken {
                    Ok(Some(mut sampler)) => {
                        executor
                            .spawn(async move {
                                let sample = sampler.sample();
                                (sampler, sample)
                            })
                            .await
                    }
                    Ok(None) => continue,
                    Err(_) => break,
                };
                if this
                    .update(cx, |this, cx| {
                        this.sampler = Some(sampler);
                        this.push(sample, cx);
                    })
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
    }

    /// Samples synchronously; tests drive the ring directly instead of
    /// waiting on the 1 Hz timer.
    #[cfg(test)]
    pub(crate) fn tick(&mut self, cx: &mut Context<Self>) {
        if self.paused {
            return;
        }
        let Some(sample) = self.sampler.as_mut().map(Sampler::sample) else {
            return;
        };
        self.push(sample, cx);
    }

    fn push(&mut self, sample: Sample, cx: &mut Context<Self>) {
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
