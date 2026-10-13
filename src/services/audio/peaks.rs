/// Columns in every [`WaveformPeaks`] this version produces.
pub const PEAK_COLUMNS: usize = 2048;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PeakColumn {
    pub min: f32,
    pub max: f32,
    pub rms: f32,
}

pub struct WaveformPeaks {
    columns: Vec<PeakColumn>,
}

impl WaveformPeaks {
    /// Min/max/rms decimation of interleaved `samples` to exactly
    /// [`PEAK_COLUMNS`] columns, or all-zero columns for empty input.
    pub fn from_interleaved(samples: &[f32], channels: usize) -> Self {
        let channels = channels.max(1);
        let frames = samples.len() / channels;
        let mut columns = Vec::with_capacity(PEAK_COLUMNS);
        for column in 0..PEAK_COLUMNS {
            let start = frames * column / PEAK_COLUMNS;
            let end = frames * (column + 1) / PEAK_COLUMNS;
            let mut min = f32::INFINITY;
            let mut max = f32::NEG_INFINITY;
            let mut sq_sum = 0f32;
            let mut count = 0usize;
            for frame in start..end {
                for channel in 0..channels {
                    let sample = samples[frame * channels + channel];
                    min = min.min(sample);
                    max = max.max(sample);
                    sq_sum += sample * sample;
                    count += 1;
                }
            }
            columns.push(if count == 0 {
                PeakColumn {
                    min: 0.,
                    max: 0.,
                    rms: 0.,
                }
            } else {
                PeakColumn {
                    min,
                    max,
                    rms: (sq_sum / count as f32).sqrt(),
                }
            });
        }
        Self { columns }
    }

    /// Decimated column at `ix`, or `None` past [`PEAK_COLUMNS`].
    pub fn peak(&self, ix: usize) -> Option<PeakColumn> {
        self.columns.get(ix).copied()
    }
}

#[cfg(test)]
#[path = "peaks.test.rs"]
mod peaks_test;
