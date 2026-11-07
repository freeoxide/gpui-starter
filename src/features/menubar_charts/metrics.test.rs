use gpui_kit::AppContext as _;

use super::*;

#[test]
fn fixture_first_sample_matches_the_formula() {
    let mut sampler = Sampler::fixture();
    let first = sampler.sample();
    assert_eq!(first.seq, 0);
    assert_eq!(first.mem_total_bytes, FIXTURE_TOTAL_BYTES);
    assert_eq!(
        first.mem_used_bytes,
        (FIXTURE_TOTAL_BYTES as f64 * 0.55) as u64
    );
    assert!((first.cpu_percent - 25.).abs() < 1e-3);
    assert!((0_f32..=100.).contains(&first.cpu_percent));
}

#[test]
fn fixture_second_sample_advances_phase_and_seq() {
    let mut sampler = Sampler::fixture();
    let first = sampler.sample();
    let second = sampler.sample();
    assert_eq!(second.seq, first.seq + 1);
    let expected_cpu = (25. + 20. * 0.35_f64.sin()).clamp(0., 100.) as f32;
    assert!((second.cpu_percent - expected_cpu).abs() < 1e-3);
    assert!(second.mem_used_bytes < second.mem_total_bytes);
}

#[gpui_kit::test]
async fn ring_advances_on_tick_freezes_while_paused_and_caps(cx: &mut gpui_kit::TestAppContext) {
    let state = cx.new(|_| MetricsState::new());
    for _ in 0..4 {
        state.update(cx, |this, cx| this.tick(cx));
    }
    assert_eq!(cx.read(|cx| state.read(cx).samples().len()), 4);
    let seqs: Vec<u64> = cx.read(|cx| state.read(cx).samples().iter().map(|s| s.seq).collect());
    assert_eq!(seqs, vec![0, 1, 2, 3]);

    state.update(cx, |this, cx| this.set_paused(true, cx));
    assert!(cx.read(|cx| state.read(cx).is_paused()));
    state.update(cx, |this, cx| this.tick(cx));
    assert_eq!(
        cx.read(|cx| state.read(cx).samples().len()),
        4,
        "paused ticks must not sample"
    );

    state.update(cx, |this, cx| this.set_paused(false, cx));
    for _ in 4..(MAX_SAMPLES + 5) {
        state.update(cx, |this, cx| this.tick(cx));
    }
    assert_eq!(cx.read(|cx| state.read(cx).samples().len()), MAX_SAMPLES);
    let (front, back) = cx.read(|cx| {
        let samples = state.read(cx).samples();
        (
            samples.front().map(|sample| sample.seq),
            samples.back().map(|sample| sample.seq),
        )
    });
    assert_eq!(
        front,
        Some(5),
        "the cap keeps the newest window: front is the 5th sample"
    );
    assert_eq!(
        back,
        Some(124),
        "the cap drops the oldest samples: back is the newest"
    );
}

#[gpui_kit::test]
async fn sampling_loop_samples_while_alive_and_exits_on_release(cx: &mut gpui_kit::TestAppContext) {
    let state = cx.new(|_| MetricsState::new());
    cx.update(|cx| MetricsState::start(&state, cx));

    cx.executor().advance_clock(INTERVAL);
    cx.run_until_parked();
    assert_eq!(
        cx.read(|cx| state.read(cx).samples().len()),
        1,
        "the loop lands one sample per interval while the entity lives"
    );

    let weak = state.downgrade();
    drop(state);
    cx.executor().advance_clock(INTERVAL * 3);
    cx.run_until_parked();
    assert!(
        weak.upgrade().is_none(),
        "the loop must not keep the released entity alive"
    );
    assert!(
        !cx.executor()
            .dispatcher()
            .as_test()
            .expect("test dispatcher")
            .scheduler()
            .has_pending_tasks(),
        "the detached loop must exit once the entity is released"
    );
}
