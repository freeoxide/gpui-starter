pub struct DemoSignal {
    phase_a: f32,
    phase_b: f32,
    lfo: f32,
    seed: u64,
}

const SAMPLE_RATE: f32 = 48_000.;
const TAU: f32 = std::f32::consts::TAU;
// Zero would pin the xorshift stream at 0; any fixed odd constant works and
// keeps new() deterministic.
const SEED: u64 = 0x9E37_79B9_7F4A_7C15;

impl DemoSignal {
    pub fn new() -> Self {
        Self {
            phase_a: 0.,
            phase_b: 0.,
            lfo: 0.,
            seed: SEED,
        }
    }

    /// Appends `frames` stereo interleaved f32 frames at 48 kHz to `out` and
    /// returns (rms, peak) of the chunk. Identical channels keep the display
    /// honest about mono content.
    pub fn fill_chunk(&mut self, out: &mut Vec<f32>, frames: usize) -> (f32, f32) {
        out.reserve(frames * 2);
        let mut peak = 0f32;
        let mut sum_sq = 0f32;
        for _ in 0..frames {
            self.phase_a = (self.phase_a + TAU * 220. / SAMPLE_RATE) % TAU;
            self.phase_b = (self.phase_b + TAU * 523.25 / SAMPLE_RATE) % TAU;
            self.lfo = (self.lfo + TAU * 0.25 / SAMPLE_RATE) % TAU;
            self.seed ^= self.seed >> 12;
            self.seed ^= self.seed << 25;
            self.seed ^= self.seed >> 27;
            let n = self.seed.wrapping_mul(0x2545_F491_4F6C_DD1D);
            let noise = (n >> 11) as f32 / (1u64 << 53) as f32 * 2. - 1.;
            let env = 0.55 + 0.45 * self.lfo.sin();
            let mono = env * (0.35 * self.phase_a.sin() + 0.20 * self.phase_b.sin()) + 0.02 * noise;
            out.extend_from_slice(&[mono, mono]);
            peak = peak.max(mono.abs());
            sum_sq += mono * mono;
        }
        let rms = (sum_sq / frames.max(1) as f32).sqrt();
        (rms, peak)
    }
}

impl Default for DemoSignal {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[path = "demo.test.rs"]
mod demo_test;
