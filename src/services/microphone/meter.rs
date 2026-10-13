use std::sync::atomic::{AtomicU32, Ordering};

const SCALE: f32 = 1_048_576.;

/// Lock-free level meter shared between the audio side and render. rms is
/// the most recent callback's RMS; peak accumulates via fetch_max until
/// [`LevelMeter::read`] resets it.
pub struct LevelMeter {
    rms: AtomicU32,
    peak: AtomicU32,
}

fn encode(amplitude: f32) -> u32 {
    (amplitude.clamp(0., 1.) * SCALE) as u32
}

fn decode(value: u32) -> f32 {
    value as f32 / SCALE
}

impl LevelMeter {
    pub fn new() -> Self {
        Self {
            rms: AtomicU32::new(0),
            peak: AtomicU32::new(0),
        }
    }

    /// Audio-side write; amplitudes clamped to 0..=1.
    pub fn update(&self, rms: f32, peak: f32) {
        self.rms.store(encode(rms), Ordering::Relaxed);
        self.peak.fetch_max(encode(peak), Ordering::Relaxed);
    }

    /// UI-side read: (rms, peak since last read); resets the peak.
    pub fn read(&self) -> (f32, f32) {
        let peak = decode(self.peak.swap(0, Ordering::Relaxed));
        (decode(self.rms.load(Ordering::Relaxed)), peak)
    }

    pub fn reset(&self) {
        self.rms.store(0, Ordering::Relaxed);
        self.peak.store(0, Ordering::Relaxed);
    }
}

impl Default for LevelMeter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "meter.test.rs"]
mod meter_test;
