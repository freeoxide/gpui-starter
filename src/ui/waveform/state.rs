use std::time::Duration;

use gpui_kit::*;

use super::history::{ChannelHistory, ColumnAgg};
use super::ring::{SampleConsumer, SampleProducer, sample_channel};
use super::spectrum::{FFT_SIZE, SpectrumAnalyzer, SpectrumFrame};

pub(crate) const REFRESH_INTERVAL: Duration = Duration::from_millis(33);

pub const MIN_WINDOW: Duration = Duration::from_millis(50);
pub const DEFAULT_WINDOW: Duration = Duration::from_secs(5);
pub const DEFAULT_RETENTION: Duration = Duration::from_secs(10);
pub const MIN_RETENTION: Duration = Duration::from_secs(1);
pub const MAX_RETENTION: Duration = Duration::from_secs(30);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum WaveformMode {
    #[default]
    Oscilloscope,
    Spectrum,
}

pub(crate) enum WaveformSamples {
    Line(Vec<f32>),
    Envelope(Vec<ColumnAgg>),
}

struct SourceAttachment {
    consumer: SampleConsumer,
    sample_rate: u32,
    channels: usize,
}

/// Retained waveform state. Mount with `cx.new(|_| WaveformState::new())`,
/// render with `Waveform::new(id, entity)`, and observe
/// `cx.observe(&state, |_, _, cx| cx.notify())`. Mutators notify once.
pub struct WaveformState {
    source: Option<SourceAttachment>,
    channels: Vec<ChannelHistory>,
    paused: bool,
    window: Duration,
    mode: WaveformMode,
    retention: Duration,
    spectrum: Option<SpectrumFrame>,
    analyzer: Option<SpectrumAnalyzer>,
    tick: Option<Task<()>>,
    drain_buf: Vec<f32>,
    mixdown: Vec<f32>,
}

impl Default for WaveformState {
    fn default() -> Self {
        Self::new()
    }
}

impl WaveformState {
    pub fn new() -> Self {
        Self {
            source: None,
            channels: Vec::new(),
            paused: false,
            window: DEFAULT_WINDOW,
            mode: WaveformMode::default(),
            retention: DEFAULT_RETENTION,
            spectrum: None,
            analyzer: None,
            tick: None,
            drain_buf: Vec::new(),
            mixdown: Vec::new(),
        }
    }

    /// Creates the sample channel, starts the 33 ms refresh tick, returns the
    /// producer for the audio thread. Panics unless `channels` is 1 or 2 and
    /// `sample_rate` > 0; `retention` clamps; replaces any previous source.
    pub fn attach_source(
        &mut self,
        sample_rate: u32,
        channels: usize,
        retention: Duration,
        cx: &mut Context<Self>,
    ) -> SampleProducer {
        assert!(channels == 1 || channels == 2, "channels must be 1 or 2");
        assert!(sample_rate > 0, "sample_rate must be nonzero");
        self.tick = None;
        self.source = None;
        let retention = retention.clamp(MIN_RETENTION, MAX_RETENTION);
        let retention_frames = (retention.as_secs_f32() * sample_rate as f32).ceil() as usize;
        self.channels = (0..channels)
            .map(|_| ChannelHistory::new(retention_frames))
            .collect();
        self.retention = retention;
        let (producer, consumer) = sample_channel(sample_rate, channels);
        self.source = Some(SourceAttachment {
            consumer,
            sample_rate,
            channels,
        });
        self.paused = false;
        self.spectrum = None;
        self.tick = Some(cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(REFRESH_INTERVAL).await;
                if this.update(cx, |state, cx| state.refresh(cx)).is_err() {
                    break;
                }
            }
        }));
        cx.notify();
        producer
    }

    /// Drops the channel, cancels the tick, clears history and spectrum.
    pub fn detach_source(&mut self, cx: &mut Context<Self>) {
        self.tick = None;
        self.source = None;
        self.channels.clear();
        self.spectrum = None;
        self.retention = DEFAULT_RETENTION;
        self.window = self.window.clamp(MIN_WINDOW, DEFAULT_RETENTION);
        cx.notify();
    }

    pub fn has_source(&self) -> bool {
        self.source.is_some()
    }

    pub fn sample_rate(&self) -> Option<u32> {
        self.source.as_ref().map(|source| source.sample_rate)
    }

    pub fn channels(&self) -> usize {
        self.source.as_ref().map_or(0, |source| source.channels)
    }

    /// Paused: the tick keeps draining and discarding, display freezes.
    pub fn set_paused(&mut self, paused: bool, cx: &mut Context<Self>) {
        self.paused = paused;
        cx.notify();
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    /// Clamped to [MIN_WINDOW, retention]. Only changes the query range.
    pub fn set_window(&mut self, window: Duration, cx: &mut Context<Self>) {
        self.window = window.clamp(MIN_WINDOW, self.retention);
        cx.notify();
    }

    pub fn window(&self) -> Duration {
        self.window
    }

    pub fn set_mode(&mut self, mode: WaveformMode, cx: &mut Context<Self>) {
        self.mode = mode;
        cx.notify();
    }

    pub fn mode(&self) -> WaveformMode {
        self.mode
    }

    pub fn retention(&self) -> Duration {
        self.retention
    }

    /// Samples the producer could not push since attach. Diagnostics readout.
    pub fn dropped_samples(&self) -> u64 {
        self.source
            .as_ref()
            .map_or(0, |source| source.consumer.dropped())
    }

    /// Drains the ring, integrates history, recomputes the spectrum, notifies
    /// once if unpaused data arrived. The internal tick calls this; tests and
    /// embedders may call it directly to force a synchronous update.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        if self.source.is_none() {
            return;
        }
        self.drain_buf.clear();
        if let Some(source) = self.source.as_mut() {
            source.consumer.drain_into(&mut self.drain_buf);
        }
        if self.drain_buf.is_empty() || self.paused {
            return;
        }
        let channels = self.channels().max(1);
        for (i, &sample) in self.drain_buf.iter().enumerate() {
            self.channels[i % channels].push(sample);
        }
        let sample_rate = self.sample_rate().unwrap_or(1);
        self.rebuild_mixdown();
        self.spectrum = Some(
            self.analyzer
                .get_or_insert_with(SpectrumAnalyzer::new)
                .analyze(&self.mixdown, sample_rate),
        );
        cx.notify();
    }

    fn rebuild_mixdown(&mut self) {
        if self.channels.len() == 1 {
            self.channels[0].last_samples(FFT_SIZE, &mut self.mixdown);
            return;
        }
        let common = self
            .channels
            .iter()
            .map(|channel| channel.len_raw())
            .min()
            .unwrap_or(0)
            .min(FFT_SIZE);
        self.channels[0].last_samples(common, &mut self.mixdown);
        self.channels[1].last_samples(common, &mut self.drain_buf);
        for (mixed, &right) in self.mixdown.iter_mut().zip(self.drain_buf.iter()) {
            *mixed = (*mixed + right) * 0.5;
        }
    }

    /// Latest spectrum over the 1024-frame mixdown; None before first data.
    pub fn spectrum(&self) -> Option<&SpectrumFrame> {
        self.spectrum.as_ref()
    }

    pub(crate) fn window_samples(&self) -> usize {
        let Some(source) = &self.source else {
            return 0;
        };
        (self.window.as_secs_f32() * source.sample_rate as f32) as usize
    }

    pub(crate) fn channel_view(&self, ch: usize, columns: usize) -> WaveformSamples {
        let Some(history) = self.channels.get(ch) else {
            return WaveformSamples::Line(Vec::new());
        };
        let window_samples = self.window_samples().max(1);
        if window_samples <= columns {
            let mut samples = Vec::new();
            history.last_samples(window_samples, &mut samples);
            WaveformSamples::Line(samples)
        } else {
            WaveformSamples::Envelope(history.query(window_samples, columns))
        }
    }
}

#[cfg(test)]
#[path = "state.test.rs"]
mod state_test;
