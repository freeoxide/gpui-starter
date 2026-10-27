use std::time::Duration;

use gpui_kit::TestAppContext;
use gpui_kit::prelude::*;
use gpui_kit::{Bounds, Pixels, point, px, size};

use super::{AudioPage, format_time, fraction_at, transport_idle};
use crate::audio::PlaybackStatus;
use crate::ui::waveform::WaveformMode;

#[test]
fn format_time_renders_minutes_and_seconds() {
    assert_eq!(format_time(Duration::ZERO), "0:00");
    assert_eq!(format_time(Duration::from_secs(65)), "1:05");
    assert_eq!(format_time(Duration::from_secs(600)), "10:00");
}

#[test]
fn fraction_at_clamps_outside_bounds() {
    let bounds = Bounds {
        origin: point(px(100.), px(0.)),
        size: size(px(200.), px(96.)),
    };
    assert_eq!(fraction_at(px(50.), bounds), 0.);
    assert_eq!(fraction_at(px(400.), bounds), 1.);
    assert_eq!(fraction_at(px(200.), bounds), 0.5);
}

#[test]
fn transport_controls_are_disabled_until_a_load_lands() {
    assert!(transport_idle(None));
    assert!(transport_idle(Some(PlaybackStatus::Loading)));
    assert!(!transport_idle(Some(PlaybackStatus::Stopped)));
    assert!(!transport_idle(Some(PlaybackStatus::Playing)));
    assert!(!transport_idle(Some(PlaybackStatus::Paused)));
}

#[gpui_kit::test]
fn page_renders_unloaded_without_a_capability_entry(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let mut page = None;
    cx.add_window(|window, cx| {
        let view = AudioPage::new(window, cx);
        page = Some(cx.entity());
        view
    });
    let page = page.unwrap();
    cx.read(|cx| {
        let status = page.read(cx).engine.read(cx).status();
        assert_eq!(status, None);
        assert!(
            transport_idle(status),
            "the play button must render disabled while unloaded"
        );
        assert!(!page.read(cx).waveform.read(cx).has_source());
        assert_eq!(
            page.read(cx).device_names,
            page.read(cx)
                .engine
                .read(cx)
                .devices()
                .iter()
                .map(|d| d.name.clone())
                .collect::<Vec<_>>()
        );
    });
}

#[gpui_kit::test]
fn loaded_page_attaches_the_live_monitor(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let mut page = None;
    cx.add_window(|window, cx| {
        let view = AudioPage::new(window, cx);
        page = Some(cx.entity());
        view
    });
    let page = page.unwrap();
    cx.update(|cx| {
        page.update(cx, |page, cx| {
            page.engine.update(cx, |engine, cx| {
                engine.load_bytes("sweep.wav", crate::audio::sweep_wav(1), cx)
            });
        });
    });
    cx.run_until_parked();
    cx.read(|cx| {
        let page = page.read(cx);
        assert!(page.waveform.read(cx).has_source());
        assert_eq!(
            page.engine.read(cx).status(),
            Some(crate::audio::PlaybackStatus::Stopped)
        );
        assert_eq!(
            page.engine.read(cx).duration(),
            Some(Duration::from_secs(1))
        );
    });
}

#[gpui_kit::test]
fn spectrum_switch_flips_the_monitor_mode(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let mut page = None;
    cx.add_window(|window, cx| {
        let view = AudioPage::new(window, cx);
        page = Some(cx.entity());
        view
    });
    let page = page.unwrap();
    cx.update(|cx| {
        page.update(cx, |page, cx| page.set_spectrum(true, cx));
    });
    cx.read(|cx| {
        assert_eq!(
            page.read(cx).waveform.read(cx).mode(),
            WaveformMode::Spectrum
        );
    });
    cx.update(|cx| {
        page.update(cx, |page, cx| page.set_spectrum(false, cx));
    });
    cx.read(|cx| {
        assert_eq!(
            page.read(cx).waveform.read(cx).mode(),
            WaveformMode::Oscilloscope
        );
    });
}

// RCA 5.2.4: eager demo loads must defer their work off the invoking turn;
// the click itself may not decode or touch the engine synchronously.
#[gpui_kit::test]
fn load_demo_touches_the_engine_only_after_the_click_turn(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let mut page = None;
    cx.add_window(|window, cx| {
        let view = AudioPage::new(window, cx);
        page = Some(cx.entity());
        view
    });
    let page = page.unwrap();
    cx.update(|cx| {
        page.update(cx, |page, cx| {
            page.load_demo("sine-sweep.wav", 1, crate::audio::sweep_wav, cx);
        });
        assert!(
            page.read(cx).engine.read(cx).status().is_none(),
            "the click turn must not decode or update the engine inline"
        );
    });
    cx.run_until_parked();
    cx.read(|cx| {
        assert_eq!(
            page.read(cx).engine.read(cx).status(),
            Some(PlaybackStatus::Stopped)
        );
        assert_eq!(
            page.read(cx).engine.read(cx).duration(),
            Some(Duration::from_secs(1))
        );
    });
}
