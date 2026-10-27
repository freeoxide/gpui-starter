use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use rtrb::{Consumer, Producer, RingBuffer};

pub(crate) const SOURCE_RING_FRAMES: usize = 1 << 14;

/// Producer half of the sample channel: `Send`, single-owner, not `Sync`.
/// Move it onto the audio thread. Samples are interleaved f32 frames
/// normalized to ±1.0; pushing never blocks, overflow is counted.
pub struct SampleProducer {
    producer: Producer<f32>,
    dropped: Arc<AtomicU64>,
    sample_rate: u32,
    channels: usize,
}

impl SampleProducer {
    /// Pushes whole interleaved frames; a trailing partial frame is rejected.
    /// Returns whole frames copied; the uncopied remainder (partial frame
    /// included) is counted in [`SampleProducer::dropped_samples`].
    pub fn push_interleaved(&mut self, samples: &[f32]) -> usize {
        let usable = samples.len() - samples.len() % self.channels;
        // The push is frame-aligned so the ring never holds a partial frame;
        // an unaligned split would desync the channel deinterleave forever.
        let slots = self.producer.slots();
        let push_len = usable.min(slots - slots % self.channels);
        let (copied, _) = self.producer.push_partial_slice(&samples[..push_len]);
        self.dropped
            .fetch_add((samples.len() - copied.len()) as u64, Ordering::Relaxed);
        copied.len() / self.channels
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    pub fn capacity_frames(&self) -> usize {
        SOURCE_RING_FRAMES
    }

    pub fn dropped_samples(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }

    /// False once the UI side detached or the state entity was dropped.
    pub fn is_attached(&self) -> bool {
        !self.producer.is_abandoned()
    }
}

pub(crate) struct SampleConsumer {
    consumer: Consumer<f32>,
    dropped: Arc<AtomicU64>,
}

impl SampleConsumer {
    pub(crate) fn drain_into(&mut self, out: &mut Vec<f32>) {
        let mut chunk = [0f32; 4096];
        loop {
            let (filled, _) = self.consumer.pop_partial_slice(&mut chunk);
            if filled.is_empty() {
                break;
            }
            out.extend_from_slice(filled);
        }
    }

    pub(crate) fn dropped(&self) -> u64 {
        self.dropped.load(Ordering::Relaxed)
    }
}

pub(crate) fn sample_channel(
    sample_rate: u32,
    channels: usize,
) -> (SampleProducer, SampleConsumer) {
    let (producer, consumer) = RingBuffer::new(SOURCE_RING_FRAMES * channels);
    let dropped = Arc::new(AtomicU64::new(0));
    (
        SampleProducer {
            producer,
            dropped: dropped.clone(),
            sample_rate,
            channels,
        },
        SampleConsumer { consumer, dropped },
    )
}

#[cfg(test)]
#[path = "ring.test.rs"]
mod ring_test;
