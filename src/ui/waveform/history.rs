use std::collections::VecDeque;

pub(crate) const LEVEL_STRIDE: usize = 8;
pub(crate) const RAW_RING_SAMPLES: usize = 1 << 14;

#[derive(Clone, Copy, Debug)]
pub(crate) struct MinMaxRms {
    pub(crate) min: f32,
    pub(crate) max: f32,
    pub(crate) rms: f32,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct ColumnAgg {
    pub(crate) min: f32,
    pub(crate) max: f32,
    pub(crate) rms: f32,
    pub(crate) populated: bool,
}

#[derive(Clone, Copy)]
struct Acc {
    min: f32,
    max: f32,
    sq_sum: f32,
    count: usize,
}

impl Acc {
    fn new() -> Self {
        Self {
            min: f32::INFINITY,
            max: f32::NEG_INFINITY,
            sq_sum: 0.,
            count: 0,
        }
    }

    fn push_sample(&mut self, s: f32) {
        self.min = self.min.min(s);
        self.max = self.max.max(s);
        self.sq_sum += s * s;
        self.count += 1;
    }

    fn absorb_entry(&mut self, entry: MinMaxRms, weight: usize) {
        self.min = self.min.min(entry.min);
        self.max = self.max.max(entry.max);
        self.sq_sum += entry.rms * entry.rms * weight as f32;
        self.count += weight;
    }

    fn finish(&self) -> MinMaxRms {
        MinMaxRms {
            min: self.min,
            max: self.max,
            rms: (self.sq_sum / self.count as f32).sqrt(),
        }
    }
}

struct Level {
    entries: VecDeque<MinMaxRms>,
    factor: usize,
    cap: usize,
    acc: Acc,
}

pub(crate) struct ChannelHistory {
    raw: VecDeque<f32>,
    levels: Vec<Level>,
    acc: Acc,
    total_pushed: u64,
}

impl ChannelHistory {
    pub(crate) fn new(retention_frames: usize) -> Self {
        let factor_cap = retention_frames.min(RAW_RING_SAMPLES);
        let mut levels = Vec::new();
        let mut factor = LEVEL_STRIDE;
        while factor <= factor_cap {
            levels.push(Level {
                entries: VecDeque::new(),
                factor,
                cap: retention_frames.div_ceil(factor),
                acc: Acc::new(),
            });
            factor *= LEVEL_STRIDE;
        }
        Self {
            raw: VecDeque::with_capacity(RAW_RING_SAMPLES),
            levels,
            acc: Acc::new(),
            total_pushed: 0,
        }
    }

    pub(crate) fn push(&mut self, s: f32) {
        self.total_pushed += 1;
        self.raw.push_back(s);
        if self.raw.len() > RAW_RING_SAMPLES {
            self.raw.pop_front();
        }
        self.acc.push_sample(s);
        if self.acc.count < LEVEL_STRIDE {
            return;
        }
        let mut entry = self.acc.finish();
        self.acc = Acc::new();
        let mut level_ix = 0;
        loop {
            let factor = self.levels[level_ix].factor;
            let level = &mut self.levels[level_ix];
            level.entries.push_back(entry);
            if level.entries.len() > level.cap {
                level.entries.pop_front();
            }
            let Some(next) = self.levels.get_mut(level_ix + 1) else {
                break;
            };
            next.acc.absorb_entry(entry, factor);
            if next.acc.count < next.factor {
                break;
            }
            entry = next.acc.finish();
            next.acc = Acc::new();
            level_ix += 1;
        }
    }

    pub(crate) fn len_raw(&self) -> usize {
        self.raw.len()
    }

    pub(crate) fn provisional(&self, level_ix: usize) -> Option<MinMaxRms> {
        let factor = self.levels[level_ix].factor;
        let n = (self.total_pushed % factor as u64) as usize;
        if n == 0 || n > self.raw.len() {
            return None;
        }
        let mut acc = Acc::new();
        for &s in self.raw.iter().rev().take(n) {
            acc.push_sample(s);
        }
        Some(acc.finish())
    }

    pub(crate) fn query(&self, window_samples: usize, columns: usize) -> Vec<ColumnAgg> {
        if columns == 0 {
            return Vec::new();
        }
        let mut out = vec![
            ColumnAgg {
                min: 0.,
                max: 0.,
                rms: 0.,
                populated: false,
            };
            columns
        ];
        if window_samples <= self.raw.len() {
            self.query_raw(window_samples, columns, &mut out);
        } else {
            self.query_levels(window_samples, columns, &mut out);
        }
        out
    }

    fn query_raw(&self, window_samples: usize, columns: usize, out: &mut [ColumnAgg]) {
        let m = window_samples.min(self.raw.len());
        let start = self.raw.len() - m;
        for (c, col) in out.iter_mut().enumerate() {
            let lo = start + m * c / columns;
            let hi = start + m * (c + 1) / columns;
            if lo >= hi {
                continue;
            }
            let mut acc = Acc::new();
            for &s in self.raw.iter().skip(lo).take(hi - lo) {
                acc.push_sample(s);
            }
            let entry = acc.finish();
            *col = ColumnAgg {
                min: entry.min,
                max: entry.max,
                rms: entry.rms,
                populated: true,
            };
        }
    }

    fn query_levels(&self, window_samples: usize, columns: usize, out: &mut [ColumnAgg]) {
        let spc = window_samples.div_ceil(columns);
        let level_ix = self
            .levels
            .iter()
            .position(|level| level.factor >= spc)
            .unwrap_or(self.levels.len() - 1);
        let factor = self.levels[level_ix].factor;
        let provisional = self.provisional(level_ix);
        let available = self.levels[level_ix].entries.len() + usize::from(provisional.is_some());
        let n = available.min(window_samples.div_ceil(factor));
        if n == 0 {
            return;
        }
        let take_complete = n - usize::from(provisional.is_some());
        let complete = &self.levels[level_ix].entries;
        let start = complete.len() - take_complete;
        let entry = |ix: usize| match provisional {
            Some(p) if ix == n - 1 => p,
            _ => complete[start + ix],
        };
        if columns >= n {
            for e in 0..n {
                let entry = entry(e);
                let from = e * columns / n;
                let to = (e + 1) * columns / n;
                for col in out.iter_mut().take(to).skip(from) {
                    *col = ColumnAgg {
                        min: entry.min,
                        max: entry.max,
                        rms: entry.rms,
                        populated: true,
                    };
                }
            }
        } else {
            for (c, col) in out.iter_mut().enumerate() {
                let from = c * n / columns;
                let to = (c + 1) * n / columns;
                if from >= to {
                    continue;
                }
                let mut min = f32::INFINITY;
                let mut max = f32::NEG_INFINITY;
                let mut sq_sum = 0.;
                for e in from..to {
                    let entry = entry(e);
                    min = min.min(entry.min);
                    max = max.max(entry.max);
                    sq_sum += entry.rms * entry.rms;
                }
                *col = ColumnAgg {
                    min,
                    max,
                    rms: (sq_sum / (to - from) as f32).sqrt(),
                    populated: true,
                };
            }
        }
    }

    pub(crate) fn last_samples(&self, n: usize, out: &mut Vec<f32>) {
        out.clear();
        let take = n.min(self.raw.len());
        let skip = self.raw.len() - take;
        out.extend(self.raw.iter().skip(skip).copied());
    }
}

#[cfg(test)]
#[path = "history.test.rs"]
mod history_test;
