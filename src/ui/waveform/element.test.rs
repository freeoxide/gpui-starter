// No glob `use gpui_kit::*` here: it would import gpui's `test` proc-macro and
// shadow this file's `#[gpui_kit::test]` attribute.
use gpui_kit::prelude::*;
use gpui_kit::{AppContext, Entity, Quad, TestAppContext, VisualTestContext, Window, px, size};

use crate::ui::waveform::{
    DEFAULT_RETENTION, MIN_WINDOW, SPECTRUM_BARS, Waveform, WaveformMode, WaveformState,
};

const SAMPLE_RATE: u32 = 48_000;
const WINDOW_HEIGHT: f32 = 400.;

struct WaveformHost {
    state: Entity<WaveformState>,
}

impl WaveformHost {
    fn new(state: Entity<WaveformState>, cx: &mut Context<Self>) -> Self {
        cx.observe(&state, |_, _, cx| cx.notify()).detach();
        Self { state }
    }
}

impl Render for WaveformHost {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        Waveform::new("test-waveform", self.state.clone())
    }
}

fn sine_interleaved(frames: usize, freq: f32) -> Vec<f32> {
    (0..frames)
        .flat_map(|i| {
            let s = (2. * std::f32::consts::PI * freq * i as f32 / SAMPLE_RATE as f32).sin();
            [s, s]
        })
        .collect()
}

fn mounted_host(
    cx: &mut TestAppContext,
    width: f32,
    state: &Entity<WaveformState>,
) -> VisualTestContext {
    cx.update(gpui_kit::component::theme::init);
    let handle = cx.open_window(size(px(width), px(WINDOW_HEIGHT)), {
        let state = state.clone();
        |_, cx| WaveformHost::new(state, cx)
    });
    let mut cx = VisualTestContext::from_window(handle.into(), cx);
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx
}

fn draw_quads(cx: &mut VisualTestContext) -> Vec<Quad> {
    cx.update(|window, cx| {
        let clear = window.draw(cx);
        let quads = window.painted_quads();
        clear.clear(cx);
        quads
    })
}

fn quad_geom(quad: &Quad) -> (f32, f32, f32, f32) {
    let b = quad.bounds;
    (
        b.origin.x.as_f32(),
        b.origin.y.as_f32(),
        b.size.width.as_f32(),
        b.size.height.as_f32(),
    )
}

/// Y centers of full-width horizontal hairlines (grid centers, dividers).
fn hairline_mids(quads: &[Quad]) -> Vec<f32> {
    quads
        .iter()
        .filter(|q| {
            let (_, _, w, h) = quad_geom(q);
            h <= 1.01 && w >= 590.
        })
        .map(|q| quad_geom(q).1 + 0.5)
        .collect()
}

fn has_vertical_gridlines(quads: &[Quad]) -> bool {
    quads.iter().any(|q| {
        let (_, _, w, h) = quad_geom(q);
        w <= 1.01 && h >= 100.
    })
}

/// Spectrum bar heights: narrow data quads between the hairlines.
fn bar_heights(quads: &[Quad]) -> Vec<f32> {
    quads
        .iter()
        .filter(|q| {
            let (_, _, w, h) = quad_geom(q);
            (4. ..=12.).contains(&w) && h > 2.
        })
        .map(|q| quad_geom(q).3)
        .collect()
}

#[gpui_kit::test]
fn detached_paints_background_and_grid_only(cx: &mut TestAppContext) {
    let state = cx.new(|_| WaveformState::new());
    let mut cx = mounted_host(cx, 600., &state);
    let quads = draw_quads(&mut cx);
    assert!(!quads.is_empty(), "the idle frame paints its background");
    assert!(
        quads.len() <= 12,
        "no source means background plus hairlines only: {} quads",
        quads.len()
    );
    assert!(has_vertical_gridlines(&quads));
}

#[gpui_kit::test]
fn mono_attached_paints_envelope_columns(cx: &mut TestAppContext) {
    let state = cx.new(|_| WaveformState::new());
    let mut producer = state.update(cx, |s, cx| {
        s.attach_source(SAMPLE_RATE, 1, DEFAULT_RETENTION, cx)
    });
    producer.push_interleaved(&sine_interleaved(2048, 440.));
    state.update(cx, |s, cx| s.refresh(cx));
    let mut cx = mounted_host(cx, 600., &state);
    let quads = draw_quads(&mut cx);
    assert!(
        quads.len() >= 2 * 600,
        "one 600-column lane paints peak and rms quads: {}",
        quads.len()
    );
}

#[gpui_kit::test]
fn stereo_attached_paints_envelope_lanes_and_divider(cx: &mut TestAppContext) {
    let state = cx.new(|_| WaveformState::new());
    let mut producer = state.update(cx, |s, cx| {
        s.attach_source(SAMPLE_RATE, 2, DEFAULT_RETENTION, cx)
    });
    producer.push_interleaved(&sine_interleaved(2048, 440.));
    state.update(cx, |s, cx| s.refresh(cx));
    let mut cx = mounted_host(cx, 600., &state);
    let quads = draw_quads(&mut cx);
    assert!(
        quads.len() >= 2 * 1200,
        "both lanes paint their columns: {}",
        quads.len()
    );
    let mids = hairline_mids(&quads);
    assert_eq!(
        mids.iter()
            .filter(|y| (190f32..=210f32).contains(*y))
            .count(),
        1,
        "exactly the divider hairline sits at the lane split: {mids:?}"
    );
}

#[gpui_kit::test]
fn spectrum_paints_bar_quads_once_data_arrives(cx: &mut TestAppContext) {
    let state = cx.new(|_| WaveformState::new());
    let mut producer = state.update(cx, |s, cx| {
        s.attach_source(SAMPLE_RATE, 1, DEFAULT_RETENTION, cx)
    });
    state.update(cx, |s, cx| s.set_mode(WaveformMode::Spectrum, cx));
    let mut cx = mounted_host(cx, 600., &state);
    assert!(
        bar_heights(&draw_quads(&mut cx)).is_empty(),
        "no refresh, no bars"
    );

    producer.push_interleaved(&sine_interleaved(2048, 440.));
    state.update(&mut cx, |s, cx| s.refresh(cx));
    let quads = draw_quads(&mut cx);
    let bars = bar_heights(&quads);
    assert!(!bars.is_empty(), "the 440 Hz frame must paint bars");
    assert!(bars.len() <= SPECTRUM_BARS);
    assert!(
        bars.iter().any(|&h| h > 100.),
        "the peak bar dominates the frame: {bars:?}"
    );
    assert!(
        has_vertical_gridlines(&quads),
        "gridlines stay painted under the bars"
    );
}

#[gpui_kit::test]
fn paused_display_freezes_across_draws(cx: &mut TestAppContext) {
    let state = cx.new(|_| WaveformState::new());
    let mut producer = state.update(cx, |s, cx| {
        s.attach_source(SAMPLE_RATE, 2, DEFAULT_RETENTION, cx)
    });
    producer.push_interleaved(&sine_interleaved(2048, 440.));
    state.update(cx, |s, cx| {
        s.refresh(cx);
        s.set_mode(WaveformMode::Spectrum, cx);
    });
    let mut cx = mounted_host(cx, 600., &state);
    let before_quads = draw_quads(&mut cx);
    assert!(
        !bar_heights(&before_quads).is_empty(),
        "the unpaused frame paints bars to freeze"
    );
    let before: Vec<_> = before_quads.iter().map(quad_geom).collect();

    state.update(&mut cx, |s, cx| s.set_paused(true, cx));
    producer.push_interleaved(&sine_interleaved(2048, 60.));
    state.update(&mut cx, |s, cx| s.refresh(cx));
    let after: Vec<_> = draw_quads(&mut cx).iter().map(quad_geom).collect();
    assert_eq!(
        before, after,
        "a paused display must keep painting the frozen frame"
    );
}

#[gpui_kit::test]
fn short_window_line_dispatch_paints_paths_not_columns(cx: &mut TestAppContext) {
    let state = cx.new(|_| WaveformState::new());
    let mut producer = state.update(cx, |s, cx| {
        s.attach_source(SAMPLE_RATE, 2, DEFAULT_RETENTION, cx)
    });
    producer.push_interleaved(&sine_interleaved(2048, 440.));
    state.update(cx, |s, cx| {
        s.refresh(cx);
        s.set_window(MIN_WINDOW, cx);
    });
    super::test_probe::reset();
    let mut cx = mounted_host(cx, 3000., &state);
    let quads = draw_quads(&mut cx);
    assert!(
        super::test_probe::line_path_builds() >= 1,
        "the line lanes must tessellate stroke paths"
    );
    assert!(
        quads.len() < 100,
        "line dispatch must not paint envelope columns: {}",
        quads.len()
    );
}
