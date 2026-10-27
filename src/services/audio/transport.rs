use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

pub(crate) struct SharedTransport {
    cursor: AtomicU64,
    playing: AtomicBool,
    volume_bits: AtomicU32,
    eof: AtomicBool,
}

impl SharedTransport {
    pub(crate) fn new(volume: f32) -> Self {
        Self {
            cursor: AtomicU64::new(0),
            playing: AtomicBool::new(false),
            volume_bits: AtomicU32::new(volume.to_bits()),
            eof: AtomicBool::new(false),
        }
    }

    pub(crate) fn cursor_frames(&self) -> u64 {
        self.cursor.load(Ordering::Relaxed)
    }

    pub(crate) fn set_cursor(&self, frames: u64) {
        self.cursor.store(frames, Ordering::Relaxed);
    }

    pub(crate) fn is_playing(&self) -> bool {
        self.playing.load(Ordering::Relaxed)
    }

    pub(crate) fn set_playing(&self, playing: bool) {
        self.playing.store(playing, Ordering::Relaxed);
    }

    pub(crate) fn volume(&self) -> f32 {
        f32::from_bits(self.volume_bits.load(Ordering::Relaxed))
    }

    pub(crate) fn set_volume(&self, volume: f32) {
        self.volume_bits.store(volume.to_bits(), Ordering::Relaxed);
    }

    pub(crate) fn set_eof(&self) {
        self.eof.store(true, Ordering::Relaxed);
    }

    pub(crate) fn take_eof(&self) -> bool {
        self.eof.swap(false, Ordering::Relaxed)
    }
}

#[cfg(test)]
#[path = "transport.test.rs"]
mod transport_test;
