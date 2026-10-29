use std::time::Duration;

use gpui_kit::prelude::*;
use gpui_kit::{Entity, Modifiers, TestAppContext, VisualTestContext, point, px, size};

use crate::ui::floating::{FloatAnchor, FloatLevel, FloatingHandle, floating_snapshot, init};

use super::{
    DICTATION_ID, DictationHud, SpeechSynth, dictation_spec, format_elapsed, spawn_dictation,
    speech_sample,
};

#[gpui_kit::test]
fn speech_sample_stays_normalized_and_varies_over_time() {
    for i in 0..2000 {
        let t = i as f32 * 0.001;
        assert!(
            (-0.99..=0.99).contains(&speech_sample(t)),
            "t={t} out of range"
        );
    }
    let samples: Vec<f32> = (0..6).map(|i| speech_sample(i as f32 * 0.25)).collect();
    assert!(
        samples.windows(2).any(|pair| pair[0] != pair[1]),
        "the synthetic signal must move over time"
    );
}

#[gpui_kit::test]
fn frozen_synth_repeats_one_chunk_while_live_synth_advances() {
    let mut frozen = SpeechSynth::new(true);
    let first = frozen.chunk();
    for _ in 0..5 {
        assert_eq!(frozen.chunk(), first, "reduced motion must not animate");
    }
    let mut live = SpeechSynth::new(false);
    let first = live.chunk();
    let second = live.chunk();
    assert_ne!(first, second);
}

#[gpui_kit::test]
fn format_elapsed_renders_minutes_and_seconds() {
    assert_eq!(format_elapsed(Duration::ZERO), "0:00");
    assert_eq!(format_elapsed(Duration::from_millis(4_500)), "0:04");
    assert_eq!(format_elapsed(Duration::from_secs(65)), "1:05");
}

#[gpui_kit::test]
fn dictation_spec_is_chromeless_screen_anchored_and_focusing() {
    let spec = dictation_spec();
    assert!(
        spec.is_chromeless(),
        "the speech-to-text look drops the header"
    );
    assert!(
        spec.is_focus(),
        "shortcut-triggered floats take focus for Esc"
    );
    assert!(matches!(spec.float_anchor(), FloatAnchor::Screen { .. }));
    assert_eq!(spec.float_level(), FloatLevel::AlwaysOnTop);
}

#[gpui_kit::test]
fn hud_start_feeds_the_waveform_and_stop_clears_the_source(cx: &mut TestAppContext) {
    let hud = cx.new(DictationHud::new);
    cx.update(|cx| {
        assert!(!hud.read(cx).is_listening());
        assert!(!hud.read(cx).waveform.read(cx).has_source());
    });

    hud.update(cx, |hud, cx| hud.set_listening(true, cx));
    cx.executor().advance_clock(Duration::from_millis(200));
    cx.run_until_parked();
    cx.update(|cx| {
        assert!(hud.read(cx).is_listening());
        let waveform = hud.read(cx).waveform.clone();
        assert!(waveform.read(cx).has_source());
        assert!(
            waveform.read(cx).spectrum().is_some(),
            "synthetic samples must reach the display path"
        );
    });

    hud.update(cx, |hud, cx| hud.set_listening(false, cx));
    cx.update(|cx| {
        let waveform = hud.read(cx).waveform.clone();
        assert!(!waveform.read(cx).has_source());
        assert!(!hud.read(cx).is_listening());
    });
}

#[gpui_kit::test]
fn reduced_motion_freezes_the_displayed_spectrum(cx: &mut TestAppContext) {
    fn bars_at(hud: &Entity<DictationHud>, cx: &TestAppContext) -> Vec<f32> {
        cx.read(|cx| {
            hud.read(cx)
                .waveform
                .read(cx)
                .spectrum()
                .expect("samples must have arrived")
                .bars()
                .to_vec()
        })
    }

    let live = cx.new(DictationHud::new);
    live.update(cx, |hud, cx| hud.set_listening(true, cx));
    cx.executor().advance_clock(Duration::from_millis(200));
    cx.run_until_parked();
    let live_first = bars_at(&live, cx);
    cx.executor().advance_clock(Duration::from_millis(400));
    cx.run_until_parked();
    let live_second = bars_at(&live, cx);
    assert_ne!(
        live_first, live_second,
        "the default display must keep animating"
    );

    cx.update(|cx| cx.set_reduce_motion(true));
    let frozen = cx.new(DictationHud::new);
    frozen.update(cx, |hud, cx| hud.set_listening(true, cx));
    cx.executor().advance_clock(Duration::from_millis(200));
    cx.run_until_parked();
    let frozen_first = bars_at(&frozen, cx);
    cx.executor().advance_clock(Duration::from_millis(400));
    cx.run_until_parked();
    let frozen_second = bars_at(&frozen, cx);
    assert_eq!(
        frozen_first, frozen_second,
        "reduced motion must freeze the displayed waveform"
    );
}

#[gpui_kit::test]
fn spawn_dictation_opens_an_idempotent_screen_float_without_a_main_window(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init);
    let spawned = cx.update(|cx| spawn_dictation(cx));
    assert!(matches!(spawned, Ok(FloatingHandle::Native(_))));
    cx.update(|cx| {
        let entry = floating_snapshot(cx)
            .into_iter()
            .find(|entry| entry.id.0 == DICTATION_ID)
            .expect("the dictation float registers");
        assert_eq!(entry.bounds.size, size(px(320.), px(140.)));
    });

    let _again = cx.update(|cx| spawn_dictation(cx));
    let count = cx.update(|cx| {
        floating_snapshot(cx)
            .iter()
            .filter(|entry| entry.id.0 == DICTATION_ID)
            .count()
    });
    assert_eq!(count, 1, "a second spawn must reuse the open float");
}

#[gpui_kit::test]
async fn the_toggle_button_flips_listening_inside_the_hud(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let window = cx.open_window(size(px(320.), px(140.)), |_, cx| DictationHud::new(cx));
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.update(|window, cx| window.draw(cx).clear(cx));

    let mut toggled = false;
    for y in (96..136usize).step_by(4) {
        cx.simulate_click(point(px(160.), px(y as f32)), Modifiers::none());
        if cx.read(|cx| window.read(cx).unwrap().is_listening()) {
            toggled = true;
            break;
        }
    }
    assert!(toggled, "the primary button must start listening");
    cx.executor()
        .advance_clock(core::time::Duration::from_millis(200));
    cx.run_until_parked();
    let (listening, listened, has_source) = cx.read(|cx| {
        let hud = window.read(cx).unwrap();
        (
            hud.is_listening(),
            hud.listened(),
            hud.waveform.read(cx).has_source(),
        )
    });
    assert!(listening);
    assert!(listened > Duration::ZERO);
    assert!(has_source);
}
