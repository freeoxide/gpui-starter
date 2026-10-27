use gpui_kit::AppContext as _;

use super::*;

const GIB: f64 = 1024. * 1024. * 1024.;

#[test]
fn fixture_first_sample_matches_the_formula() {
    let mut sampler = Sampler::fixture();
    let first = sampler.sample();
    assert_eq!(first.seq, 0);
    assert_eq!(first.mem_total_bytes, 16 * 1024 * 1024 * 1024);
    assert_eq!(first.mem_used_bytes, (16. * GIB * 0.55) as u64);
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
}
