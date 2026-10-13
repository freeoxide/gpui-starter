use std::sync::Arc;
use std::time::Duration;

use gpui_kit::{Context, Task};

use super::model::{Library, Track, TrackId};

pub(crate) const TICK_MS: u64 = 250;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TransportStatus {
    #[default]
    Stopped,
    Playing,
    Paused,
}

/// Simulated playback engine; the simulated clock is the source of truth for
/// position. Every state-changing mutator notifies exactly once; the
/// documented no-ops return without notifying. Consumers observe the entity.
pub struct PlayerEngine {
    library: Arc<Library>,
    queue: Vec<TrackId>,
    current: Option<usize>,
    status: TransportStatus,
    position_ms: u64,
    volume: f32,
    _tick: Task<()>,
}

impl PlayerEngine {
    pub fn new(library: Arc<Library>, cx: &mut Context<Self>) -> Self {
        let tick = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(TICK_MS))
                    .await;
                if this.update(cx, |engine, cx| engine.tick(cx)).is_err() {
                    break;
                }
            }
        });
        Self {
            library,
            queue: Vec::new(),
            current: None,
            status: TransportStatus::Stopped,
            position_ms: 0,
            volume: 0.8,
            _tick: tick,
        }
    }

    /// Replaces the queue and starts playback at `start_ix`. No-op when
    /// `start_ix >= tracks.len()` or the queue would be empty.
    pub fn play_queue(&mut self, tracks: Vec<TrackId>, start_ix: usize, cx: &mut Context<Self>) {
        if start_ix >= tracks.len() {
            return;
        }
        self.queue = tracks;
        self.current = Some(start_ix);
        self.position_ms = 0;
        self.status = TransportStatus::Playing;
        cx.notify();
    }

    /// Jumps to an existing queue index and plays.
    pub fn play_queue_index(&mut self, ix: usize, cx: &mut Context<Self>) {
        if ix >= self.queue.len() {
            return;
        }
        self.current = Some(ix);
        self.position_ms = 0;
        self.status = TransportStatus::Playing;
        cx.notify();
    }

    /// Playing -> Paused; Paused -> Playing; Stopped -> Playing when a
    /// current track exists, otherwise no-op.
    pub fn toggle_playback(&mut self, cx: &mut Context<Self>) {
        match self.status {
            TransportStatus::Playing => {
                self.status = TransportStatus::Paused;
                cx.notify();
            }
            TransportStatus::Paused | TransportStatus::Stopped => {
                if self.current.is_some() {
                    self.status = TransportStatus::Playing;
                    cx.notify();
                }
            }
        }
    }

    /// Advances; at the end of the queue the engine stops (position 0,
    /// `Stopped`, current stays on the last index). `next`/`previous` keep
    /// a Paused status Paused.
    pub fn next(&mut self, cx: &mut Context<Self>) {
        self.advance_to_end_of_track(cx);
    }

    /// Rewinds; at index 0 it only resets position to 0.
    pub fn previous(&mut self, cx: &mut Context<Self>) {
        let Some(ix) = self.current else {
            return;
        };
        if ix == 0 {
            if self.position_ms != 0 {
                self.position_ms = 0;
                cx.notify();
            }
            return;
        }
        self.current = Some(ix - 1);
        self.position_ms = 0;
        cx.notify();
    }

    /// Clamped to the current track's duration; no-op with no current track.
    pub fn seek_ms(&mut self, ms: u64, cx: &mut Context<Self>) {
        if self.current_track().is_none() {
            return;
        }
        let clamped = ms.min(self.duration_ms());
        if self.position_ms == clamped {
            return;
        }
        self.position_ms = clamped;
        cx.notify();
    }

    /// Clamped to 0.0..=1.0.
    pub fn set_volume(&mut self, volume: f32, cx: &mut Context<Self>) {
        let clamped = volume.clamp(0., 1.);
        if self.volume == clamped {
            return;
        }
        self.volume = clamped;
        cx.notify();
    }

    /// Appends; if nothing was ever selected the new track becomes current
    /// (status stays Stopped until play).
    pub fn enqueue(&mut self, track: TrackId, cx: &mut Context<Self>) {
        self.queue.push(track);
        if self.current.is_none() {
            self.current = Some(self.queue.len() - 1);
        }
        cx.notify();
    }

    pub fn clear_queue(&mut self, cx: &mut Context<Self>) {
        self.queue.clear();
        self.current = None;
        self.position_ms = 0;
        self.status = TransportStatus::Stopped;
        cx.notify();
    }

    pub fn status(&self) -> TransportStatus {
        self.status
    }

    pub fn current_track(&self) -> Option<Track> {
        let ix = self.current?;
        self.library.track(self.queue[ix]).cloned()
    }

    pub fn position_ms(&self) -> u64 {
        self.position_ms
    }

    pub fn duration_ms(&self) -> u64 {
        self.current_track().map_or(0, |track| track.duration_ms)
    }

    pub fn progress(&self) -> f32 {
        let duration = self.duration_ms();
        if duration == 0 {
            return 0.;
        }
        (self.position_ms as f32 / duration as f32).clamp(0., 1.)
    }

    pub fn volume(&self) -> f32 {
        self.volume
    }

    pub fn queue(&self) -> &[TrackId] {
        &self.queue
    }

    pub fn current_ix(&self) -> Option<usize> {
        self.current
    }

    pub(crate) fn tick(&mut self, cx: &mut Context<Self>) {
        if self.status != TransportStatus::Playing {
            return;
        }
        self.position_ms += TICK_MS;
        if self.position_ms >= self.duration_ms() {
            self.position_ms = 0;
            self.advance_to_end_of_track(cx);
            return;
        }
        cx.notify();
    }

    fn advance_to_end_of_track(&mut self, cx: &mut Context<Self>) {
        let Some(ix) = self.current else {
            return;
        };
        if ix + 1 < self.queue.len() {
            self.current = Some(ix + 1);
            self.position_ms = 0;
        } else {
            self.position_ms = 0;
            self.status = TransportStatus::Stopped;
        }
        cx.notify();
    }
}

#[cfg(test)]
#[path = "engine.test.rs"]
mod engine_test;
