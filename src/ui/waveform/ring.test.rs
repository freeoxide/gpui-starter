use super::*;

#[test]
fn _assert_send() {
    fn assert_send<T: Send>() {}
    assert_send::<SampleProducer>();
}

#[test]
fn push_pop_roundtrip_preserves_order_and_length() {
    let (mut producer, mut consumer) = sample_channel(48_000, 2);
    let samples: Vec<f32> = (0..24).map(|i| i as f32 * 0.5).collect();
    assert_eq!(producer.push_interleaved(&samples), 12);
    assert_eq!(producer.dropped_samples(), 0);

    let mut drained = Vec::new();
    consumer.drain_into(&mut drained);
    assert_eq!(drained, samples);
}

#[test]
fn partial_trailing_frame_is_rejected_and_counted() {
    let (mut producer, _consumer) = sample_channel(48_000, 2);
    let pushed = producer.push_interleaved(&[0., 1., 2., 3., 4.]);
    assert_eq!(pushed, 2);
    assert_eq!(producer.dropped_samples(), 1);
}

#[test]
fn overflow_counts_exact_uncopied_and_keeps_earlier_data() {
    let (mut producer, mut consumer) = sample_channel(48_000, 2);
    producer.push_interleaved(&[100., 101., 102., 103.]);

    let burst: Vec<f32> = (0..(20_000 * 2)).map(|i| i as f32).collect();
    let pushed_frames = producer.push_interleaved(&burst);
    assert_eq!(pushed_frames, producer.capacity_frames() - 2);
    let dropped_samples = 20_000 * 2 - (producer.capacity_frames() - 2) * 2;
    assert_eq!(producer.dropped_samples(), dropped_samples as u64);

    let mut drained = Vec::new();
    consumer.drain_into(&mut drained);
    assert_eq!(
        drained,
        [100., 101., 102., 103.]
            .into_iter()
            .chain(
                burst[..(producer.capacity_frames() - 2) * 2]
                    .iter()
                    .copied()
            )
            .collect::<Vec<f32>>()
    );
}

#[test]
fn is_attached_flips_false_when_consumer_drops() {
    let (producer, consumer) = sample_channel(48_000, 1);
    assert!(producer.is_attached());
    drop(consumer);
    assert!(!producer.is_attached());
}
