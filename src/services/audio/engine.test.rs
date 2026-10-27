use std::time::Duration;

use gpui_kit::TestAppContext;
use gpui_kit::prelude::*;

use super::super::demo;
use super::{AudioEngine, PlaybackStatus};
use crate::ui::waveform::{DEFAULT_RETENTION, WaveformState};

fn loaded_sweep_engine(cx: &mut TestAppContext) -> gpui_kit::Entity<AudioEngine> {
    let engine = cx.update(|cx| cx.new(|cx| AudioEngine::new(cx)));
    cx.update(|cx| {
        engine.update(cx, |engine, cx| {
            engine.load_bytes("sweep.wav", demo::sweep_wav(1), cx)
        })
    });
    cx.run_until_parked();
    engine
}

#[gpui_kit::test]
fn load_bytes_lands_stopped_with_peaks_and_meta(cx: &mut TestAppContext) {
    let engine = loaded_sweep_engine(cx);
    cx.update(|cx| {
        assert_eq!(engine.read(cx).status(), Some(PlaybackStatus::Stopped));
        assert_eq!(engine.read(cx).duration(), Some(Duration::from_secs(1)));
        assert!(engine.read(cx).peaks().is_some());
        let meta = engine.read(cx).meta().unwrap();
        assert_eq!(meta.sample_rate, 44_100);
        assert_eq!(meta.channels, 2);
        assert_eq!(meta.source_name, "sweep.wav");
    });
}

#[gpui_kit::test]
fn seek_and_volume_clamp(cx: &mut TestAppContext) {
    let engine = loaded_sweep_engine(cx);
    cx.update(|cx| {
        engine.update(cx, |engine, cx| engine.seek_fraction(0.5, cx));
        assert_eq!(engine.read(cx).position(), Duration::from_millis(500));
        engine.update(cx, |engine, cx| engine.seek_fraction(2., cx));
        assert_eq!(
            engine.read(cx).position(),
            engine.read(cx).duration().unwrap()
        );
        engine.update(cx, |engine, cx| engine.set_volume(1.5, cx));
        assert_eq!(engine.read(cx).volume(), 1.);
        engine.update(cx, |engine, cx| engine.set_volume(-1., cx));
        assert_eq!(engine.read(cx).volume(), 0.);
    });
}

#[gpui_kit::test]
fn unload_clears_every_reader(cx: &mut TestAppContext) {
    let engine = loaded_sweep_engine(cx);
    cx.update(|cx| engine.update(cx, |engine, cx| engine.unload(cx)));
    cx.update(|cx| {
        assert_eq!(engine.read(cx).status(), None);
        assert_eq!(engine.read(cx).duration(), None);
        assert!(engine.read(cx).peaks().is_none());
        assert!(engine.read(cx).meta().is_none());
        assert_eq!(engine.read(cx).position(), Duration::ZERO);
        assert_eq!(engine.read(cx).progress(), 0.);
        assert_eq!(engine.read(cx).last_error(), None);
    });
}

#[gpui_kit::test]
fn failed_load_keeps_the_previous_load(cx: &mut TestAppContext) {
    let engine = loaded_sweep_engine(cx);
    cx.update(|cx| {
        engine.update(cx, |engine, cx| {
            engine.load_bytes("garbage.wav", b"not audio".to_vec(), cx)
        })
    });
    cx.run_until_parked();
    cx.update(|cx| {
        assert_eq!(engine.read(cx).duration(), Some(Duration::from_secs(1)));
        assert!(engine.read(cx).last_error().is_some());
    });
}

#[gpui_kit::test]
fn failed_reload_keeps_the_previous_load_playable(cx: &mut TestAppContext) {
    let waveform = cx.update(|cx| cx.new(|_| WaveformState::new()));
    let engine = cx.update(|cx| cx.new(|cx| AudioEngine::new(cx)));
    cx.update(|cx| {
        engine.update(cx, |engine, cx| engine.attach_meter(&waveform, cx));
        engine.update(cx, |engine, cx| {
            engine.load_bytes("sweep.wav", demo::sweep_wav(1), cx)
        });
        engine.update(cx, |engine, cx| engine.toggle_play(cx));
    });
    cx.run_until_parked();
    cx.update(|cx| {
        engine.update(cx, |engine, cx| {
            engine.load_bytes("garbage.wav", b"not audio".to_vec(), cx)
        })
    });
    cx.run_until_parked();
    cx.update(|cx| {
        let engine_ref = engine.read(cx);
        assert_eq!(
            engine_ref.status(),
            Some(PlaybackStatus::Stopped),
            "a failed reload must restore a playable status"
        );
        assert_eq!(engine_ref.duration(), Some(Duration::from_secs(1)));
        assert!(engine_ref.last_error().is_some());
        #[cfg(not(target_family = "wasm"))]
        {
            let has_session = engine_ref.session.is_some();
            let has_producer = engine_ref
                .load
                .as_ref()
                .is_some_and(|load| load.producer.is_some());
            assert!(
                has_session || has_producer,
                "neither a session nor a producer path survived the failed reload"
            );
        }
    });
}

#[gpui_kit::test]
fn newer_load_wins_the_generation_race(cx: &mut TestAppContext) {
    let engine = cx.update(|cx| cx.new(|cx| AudioEngine::new(cx)));
    cx.update(|cx| {
        engine.update(cx, |engine, cx| {
            engine.load_bytes("bad.wav", b"not audio".to_vec(), cx)
        });
        engine.update(cx, |engine, cx| {
            engine.load_bytes("sweep.wav", demo::sweep_wav(1), cx)
        });
    });
    cx.run_until_parked();
    cx.update(|cx| {
        assert_eq!(engine.read(cx).status(), Some(PlaybackStatus::Stopped));
    });
}

#[gpui_kit::test]
fn attach_meter_feeds_and_detaches_the_widget(cx: &mut TestAppContext) {
    let waveform = cx.update(|cx| cx.new(|_| WaveformState::new()));
    let engine = cx.update(|cx| cx.new(|cx| AudioEngine::new(cx)));
    cx.update(|cx| {
        engine.update(cx, |engine, cx| engine.attach_meter(&waveform, cx));
        engine.update(cx, |engine, cx| {
            engine.load_bytes("sweep.wav", demo::sweep_wav(1), cx)
        });
    });
    cx.run_until_parked();
    cx.update(|cx| {
        assert!(waveform.read(cx).has_source());
        assert!(waveform.read(cx).sample_rate().is_some());
        assert_eq!(engine.read(cx).duration(), Some(Duration::from_secs(1)));
    });
    cx.update(|cx| engine.update(cx, |engine, cx| engine.unload(cx)));
    cx.update(|cx| assert!(!waveform.read(cx).has_source()));
}

#[gpui_kit::test]
fn refresh_devices_and_unknown_select_are_safe(cx: &mut TestAppContext) {
    let engine = cx.update(|cx| cx.new(|cx| AudioEngine::new(cx)));
    cx.update(|cx| {
        engine.update(cx, |engine, cx| engine.refresh_devices(cx));
        engine.update(cx, |engine, cx| engine.select_device("no such device", cx));
        assert_eq!(engine.read(cx).selected_device(), None);
    });
}

// The cpal host scan is synchronous and slow: it must never run on the
// caller's turn, only inside the task that delivers the result (RCA 5.2.3).
#[cfg(not(target_family = "wasm"))]
#[gpui_kit::test]
fn refresh_devices_scans_off_the_caller_and_commits_from_a_task(cx: &mut TestAppContext) {
    let expected = super::super::output::enumerate();
    let engine = cx.update(|cx| cx.new(|cx| AudioEngine::new(cx)));
    cx.update(|cx| {
        engine.update(cx, |engine, cx| engine.refresh_devices(cx));
        assert!(
            engine.read(cx).devices().is_empty(),
            "the device scan must not run inline on the caller's turn"
        );
    });
    cx.run_until_parked();
    cx.update(|cx| assert_eq!(engine.read(cx).devices(), expected));
}

#[gpui_kit::test]
fn meter_pushes_from_a_foreign_thread_only_land_via_the_timer_drain(cx: &mut TestAppContext) {
    let waveform = cx.update(|cx| cx.new(|_| WaveformState::new()));
    let mut producer = cx.update(|cx| {
        waveform.update(cx, |state, cx| {
            state.attach_source(48_000, 2, DEFAULT_RETENTION, cx)
        })
    });
    let samples: Vec<f32> = (0..2048)
        .flat_map(|i| {
            let s = (440. * i as f32 / 48_000. * std::f32::consts::TAU).sin() * 0.5;
            [s, s]
        })
        .collect();
    // The cpal data callback runs off the GPUI thread; pushing is its only
    // legal effect. Nothing may observe the samples until the drain tick.
    std::thread::spawn(move || producer.push_interleaved(&samples))
        .join()
        .unwrap();
    cx.run_until_parked();
    cx.update(|cx| {
        assert!(
            waveform.read(cx).spectrum().is_none(),
            "a push must never advance the meter; only the timer drain does"
        );
    });
    cx.executor().advance_clock(Duration::from_millis(34));
    cx.run_until_parked();
    cx.update(|cx| assert!(waveform.read(cx).spectrum().is_some()));
}

#[cfg(not(target_family = "wasm"))]
fn usable_output_device() -> bool {
    use cpal::traits::HostTrait as _;
    let Some(device) = cpal::default_host().default_output_device() else {
        return false;
    };
    // A default device object can exist on a headless host while the device
    // itself cannot be opened; planning is the cheap probe that catches it.
    super::super::output::plan_stream(&device, 44_100).is_ok()
}

#[cfg(not(target_family = "wasm"))]
#[gpui_kit::test]
fn native_playback_advances_pauses_and_stops(cx: &mut TestAppContext) {
    use gpui_kit::Entity;
    if !usable_output_device() {
        return;
    }
    let engine = loaded_sweep_engine(cx);
    cx.update(|cx| engine.update(cx, |engine, cx| engine.toggle_play(cx)));
    cx.run_until_parked();
    cx.update(|cx| assert_eq!(engine.read(cx).status(), Some(PlaybackStatus::Playing)));
    let mut advanced = false;
    for _ in 0..50 {
        cx.executor().advance_clock(Duration::from_millis(33));
        cx.run_until_parked();
        if cx.update(|cx| engine.read(cx).position()) > Duration::ZERO {
            advanced = true;
            break;
        }
    }
    assert!(advanced, "the playhead never advanced on a live device");
    let position = cx.update(|cx| engine.read(cx).position());
    cx.update(|cx| {
        engine.update(cx, |engine, cx| engine.toggle_play(cx));
        assert_eq!(engine.read(cx).status(), Some(PlaybackStatus::Paused));
    });
    for _ in 0..5 {
        cx.executor().advance_clock(Duration::from_millis(33));
        cx.run_until_parked();
    }
    let frozen = cx.update(|cx| engine.read(cx).position());
    assert!(frozen >= position);
    cx.update(|cx| {
        engine.update(cx, |engine, cx| engine.seek_fraction(0.75, cx));
        assert!(engine.read(cx).position() >= Duration::from_millis(700));
        engine.update(cx, |engine, cx| engine.stop(cx));
        assert_eq!(engine.read(cx).status(), Some(PlaybackStatus::Stopped));
        assert_eq!(engine.read(cx).position(), Duration::ZERO);
    });
    let _engine: Entity<AudioEngine> = engine;
}

/// Runs only where the output device cannot open: the resume path fails and
/// the error must stay visible instead of being cleared by toggle_play.
#[cfg(not(target_family = "wasm"))]
#[gpui_kit::test]
fn failed_play_keeps_the_error_visible(cx: &mut TestAppContext) {
    if usable_output_device() {
        return;
    }
    let engine = loaded_sweep_engine(cx);
    cx.update(|cx| {
        assert!(
            engine.read(cx).last_error().is_some(),
            "a plan-failing device must leave an error after load"
        );
    });
    cx.update(|cx| engine.update(cx, |engine, cx| engine.toggle_play(cx)));
    cx.run_until_parked();
    cx.update(|cx| {
        assert_eq!(engine.read(cx).status(), Some(PlaybackStatus::Stopped));
        assert!(
            engine.read(cx).last_error().is_some(),
            "toggle_play must not clear the error of a failed resume"
        );
    });
    cx.update(|cx| engine.update(cx, |engine, cx| engine.toggle_play(cx)));
    cx.run_until_parked();
    cx.update(|cx| {
        assert!(
            engine.read(cx).last_error().is_some(),
            "a second failed resume must still surface its error"
        );
    });
}
