use std::sync::Arc;

use rustfft::FftPlanner;
use rustfft::num_complex::Complex;

pub(crate) const FFT_SIZE: usize = 1024;
pub(crate) const MIN_DB: f32 = -60.;

/// Bar count of every [`SpectrumFrame`] this version produces.
pub const SPECTRUM_BARS: usize = 64;

/// One computed spectrum. `bars()` holds normalized 0.0..=1.0 heights mapped
/// from [MIN_DB, 0] dB, log-spaced from max(20 Hz, fft bin width) to Nyquist.
pub struct SpectrumFrame {
    bars: Vec<f32>,
    fft_size: usize,
    sample_rate: u32,
}

impl SpectrumFrame {
    pub fn bars(&self) -> &[f32] {
        &self.bars
    }

    pub fn fft_size(&self) -> usize {
        self.fft_size
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Inclusive Hz range bar `ix` aggregates; None when out of range.
    pub fn bar_band(&self, ix: usize) -> Option<(f32, f32)> {
        bar_edges(ix, self.sample_rate, self.fft_size)
    }
}

fn bar_edges(ix: usize, sample_rate: u32, fft_size: usize) -> Option<(f32, f32)> {
    if ix >= SPECTRUM_BARS {
        return None;
    }
    let bin_hz = sample_rate as f32 / fft_size as f32;
    let lo = 20f32.max(bin_hz);
    let hi = sample_rate as f32 / 2.;
    let ratio = (hi / lo).powf(ix as f32 / SPECTRUM_BARS as f32);
    let edge = lo * ratio;
    let next = lo * (hi / lo).powf((ix + 1) as f32 / SPECTRUM_BARS as f32);
    Some((edge, next))
}

pub(crate) struct SpectrumAnalyzer {
    fft: Arc<dyn rustfft::Fft<f32>>,
    scratch: Vec<Complex<f32>>,
    buffer: Vec<Complex<f32>>,
}

impl SpectrumAnalyzer {
    pub(crate) fn new() -> Self {
        let fft = FftPlanner::<f32>::new().plan_fft_forward(FFT_SIZE);
        let scratch_len = fft.get_inplace_scratch_len();
        Self {
            fft,
            scratch: vec![Complex::ZERO; scratch_len],
            buffer: vec![Complex::ZERO; FFT_SIZE],
        }
    }

    pub(crate) fn analyze(&mut self, mixdown: &[f32], sample_rate: u32) -> SpectrumFrame {
        let m = mixdown.len().min(FFT_SIZE);
        let tail = &mixdown[mixdown.len() - m..];
        self.buffer[..FFT_SIZE - m].fill(Complex::ZERO);
        for (slot, &s) in self.buffer[FFT_SIZE - m..].iter_mut().zip(tail) {
            *slot = Complex::new(s, 0.);
        }
        self.fft
            .process_with_scratch(&mut self.buffer, &mut self.scratch);

        let scale = FFT_SIZE as f32 / 2.;
        let magnitude = |k: usize| {
            let bin = self.buffer[k];
            (bin.re * bin.re + bin.im * bin.im).sqrt() / scale
        };

        let mut bars = Vec::with_capacity(SPECTRUM_BARS);
        for ix in 0..SPECTRUM_BARS {
            let (edge, next) = bar_edges(ix, sample_rate, FFT_SIZE).unwrap();
            let bin_hz = sample_rate as f32 / FFT_SIZE as f32;
            let from = ((edge / bin_hz).ceil() as usize).clamp(1, FFT_SIZE / 2);
            let to = (((next / bin_hz).ceil() as usize).max(from + 1)).min(FFT_SIZE / 2);
            let peak = (from..to).map(magnitude).fold(0_f32, f32::max);
            let db = 20. * (peak + 1e-9).log10();
            bars.push(((db - MIN_DB) / -MIN_DB).clamp(0., 1.));
        }
        SpectrumFrame {
            bars,
            fft_size: FFT_SIZE,
            sample_rate,
        }
    }
}

#[cfg(test)]
#[path = "spectrum.test.rs"]
mod spectrum_test;
