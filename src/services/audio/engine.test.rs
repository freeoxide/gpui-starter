#[cfg(not(target_family = "wasm"))]
use std::sync::Arc;
#[cfg(not(target_family = "wasm"))]
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use gpui_kit::TestAppContext;
use gpui_kit::prelude::*;

use super::super::demo;
use super::{AudioEngine, PlaybackStatus};
use crate::ui::waveform::{DEFAULT_RETENTION, WaveformState};

fn loaded_sweep_engine(cx: &mut TestAppContext) -> gpui_kit::Entity<AudioEngine> {
    let engine = cx.update(|cx| cx.new(AudioEngine::new));
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
    let engine = cx.update(|cx| cx.new(AudioEngine::new));
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
    let engine = cx.update(|cx| cx.new(AudioEngine::new));
    cx.update(|cx| {
        engine.update(cx, |engine, cx| {
            engine.load_bytes("bad.wav", b"not audio".to_vec(), cx)
        });
        engine.update(cx, |engine, cx| {
            engine.load_bytes("sweep.wav", demo::sweep_wav(1), cx)
        });
    });
    cx.run_until_parked();
    // Deliver the stale decode after the newer load already committed.
    cx.update(|cx| {
        engine.update(cx, |engine, cx| {
            engine.finish_load(1, Err(super::AudioError::Decode("stale".into())), cx);
        });
        let engine_ref = engine.read(cx);
        assert_eq!(engine_ref.status(), Some(PlaybackStatus::Stopped));
        assert_eq!(engine_ref.duration(), Some(Duration::from_secs(1)));
        assert_eq!(
            engine_ref.meta().map(|meta| meta.source_name.as_str()),
            Some("sweep.wav")
        );
    });
    // The unload path bumps the generation too: a late decode error must not
    // surface on an engine with nothing loaded.
    cx.update(|cx| engine.update(cx, |engine, cx| engine.unload(cx)));
    cx.update(|cx| {
        engine.update(cx, |engine, cx| {
            engine.finish_load(2, Err(super::AudioError::Decode("stale".into())), cx);
        });
        assert_eq!(engine.read(cx).status(), None);
        assert_eq!(engine.read(cx).last_error(), None);
    });
}

#[gpui_kit::test]
fn attach_meter_feeds_and_detaches_the_widget(cx: &mut TestAppContext) {
    let waveform = cx.update(|cx| cx.new(|_| WaveformState::new()));
    let engine = cx.update(|cx| cx.new(AudioEngine::new));
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
    let engine = cx.update(|cx| cx.new(AudioEngine::new));
    cx.update(|cx| {
        engine.update(cx, |engine, cx| engine.refresh_devices(cx));
        engine.update(cx, |engine, cx| engine.select_device("no such device", cx));
        assert_eq!(engine.read(cx).selected_device(), None);
    });
}

// The cpal host scan is synchronous and slow: it must never run on the
// caller's turn, only inside the task that delivers the result (RCA §5.2.3).
#[cfg(not(target_family = "wasm"))]
#[gpui_kit::test]
fn refresh_devices_scans_off_the_caller_and_commits_from_a_task(cx: &mut TestAppContext) {
    let expected = super::super::output::enumerate();
    let engine = cx.update(|cx| cx.new(AudioEngine::new));
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

// In-process stand-in for the cpal stream: counts play/pause calls so the
// lifecycle tests assert what actually reached the output device.
#[cfg(not(target_family = "wasm"))]
struct FakeSession {
    plays: AtomicUsize,
    pauses: AtomicUsize,
}

#[cfg(not(target_family = "wasm"))]
impl FakeSession {
    fn new() -> Self {
        Self {
            plays: AtomicUsize::new(0),
            pauses: AtomicUsize::new(0),
        }
    }

    fn plays(&self) -> usize {
        self.plays.load(Ordering::Relaxed)
    }

    fn pauses(&self) -> usize {
        self.pauses.load(Ordering::Relaxed)
    }
}

#[cfg(not(target_family = "wasm"))]
impl super::super::output::SessionSink for FakeSession {
    fn play(&self) -> Result<(), super::AudioError> {
        self.plays.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    fn pause(&self) -> Result<(), super::AudioError> {
        self.pauses.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

#[cfg(not(target_family = "wasm"))]
fn fake_session_builder(fake: Arc<FakeSession>) -> super::SessionBuilder {
    Arc::new(move |_request: super::SessionRequest| {
        Ok(fake.clone() as Arc<dyn super::super::output::SessionSink>)
    })
}

#[cfg(not(target_family = "wasm"))]
fn fake_loaded_engine(
    cx: &mut TestAppContext,
    fake: Arc<FakeSession>,
) -> gpui_kit::Entity<AudioEngine> {
    let engine = cx.update(|cx| cx.new(AudioEngine::new));
    cx.update(|cx| {
        engine.update(cx, |engine, _| {
            engine.session_builder = fake_session_builder(fake);
        });
        engine.update(cx, |engine, cx| {
            engine.load_bytes("sweep.wav", demo::sweep_wav(1), cx)
        });
    });
    cx.run_until_parked();
    engine
}

// The session seam replaces cpal and fill_buffer replaces the audio
// callback, so the lifecycle runs the same on every host.
#[cfg(not(target_family = "wasm"))]
#[gpui_kit::test]
fn playback_runs_without_a_meter_and_pauses_and_stops(cx: &mut TestAppContext) {
    let fake = Arc::new(FakeSession::new());
    let engine = fake_loaded_engine(cx, fake.clone());
    cx.update(|cx| {
        assert!(
            engine.read(cx).session.is_some(),
            "playback must not depend on an attached meter widget"
        );
        assert_eq!(engine.read(cx).status(), Some(PlaybackStatus::Stopped));
    });

    cx.update(|cx| engine.update(cx, |engine, cx| engine.toggle_play(cx)));
    cx.update(|cx| assert_eq!(engine.read(cx).status(), Some(PlaybackStatus::Playing)));
    assert_eq!(fake.plays(), 1);

    let position = cx.update(|cx| {
        engine.update(cx, |engine, _| {
            let load = engine.load.as_ref().expect("load landed");
            let mut out = [0f32; 64];
            let mut tap = Vec::new();
            super::super::output::fill_buffer(
                &load.render,
                load.render_channels,
                &load.transport,
                &mut out,
                &mut tap,
            );
            engine.position()
        })
    });
    assert!(
        position > Duration::ZERO,
        "one callback must advance the cursor"
    );

    cx.update(|cx| engine.update(cx, |engine, cx| engine.toggle_play(cx)));
    cx.update(|cx| assert_eq!(engine.read(cx).status(), Some(PlaybackStatus::Paused)));
    assert_eq!(fake.pauses(), 1);
    // Paused streams stop calling back: the cursor freezes exactly.
    assert_eq!(cx.update(|cx| engine.read(cx).position()), position);

    cx.update(|cx| {
        engine.update(cx, |engine, cx| engine.stop(cx));
        assert_eq!(engine.read(cx).status(), Some(PlaybackStatus::Stopped));
        assert_eq!(engine.read(cx).position(), Duration::ZERO);
    });
    assert_eq!(fake.pauses(), 2);
}

#[cfg(not(target_family = "wasm"))]
#[gpui_kit::test]
fn stop_cancels_a_session_build_queued_by_toggle(cx: &mut TestAppContext) {
    let fake = Arc::new(FakeSession::new());
    let first_build_fails = Arc::new(AtomicBool::new(true));
    let engine = cx.update(|cx| cx.new(AudioEngine::new));
    cx.update(|cx| {
        engine.update(cx, |engine, _| {
            let fake = fake.clone();
            let fail_first = first_build_fails.clone();
            engine.session_builder = Arc::new(move |_request: super::SessionRequest| {
                if fail_first.swap(false, Ordering::SeqCst) {
                    Err(super::AudioError::NoOutputDevice)
                } else {
                    Ok(fake.clone() as Arc<dyn super::super::output::SessionSink>)
                }
            });
        });
        engine.update(cx, |engine, cx| {
            engine.load_bytes("sweep.wav", demo::sweep_wav(1), cx)
        });
    });
    cx.run_until_parked();
    cx.update(|cx| assert!(engine.read(cx).session.is_none()));

    // toggle_play queues a build; stop() must retire it before it lands.
    cx.update(|cx| {
        engine.update(cx, |engine, cx| engine.toggle_play(cx));
        engine.update(cx, |engine, cx| engine.stop(cx));
    });
    cx.run_until_parked();
    cx.update(|cx| {
        let engine_ref = engine.read(cx);
        assert_eq!(engine_ref.status(), Some(PlaybackStatus::Stopped));
        assert!(!engine_ref.pending_play);
        assert!(engine_ref.session.is_none());
    });
    assert_eq!(fake.plays(), 0, "a session built after stop must not play");
}

#[cfg(not(target_family = "wasm"))]
#[gpui_kit::test]
fn eof_pauses_the_session_and_stops(cx: &mut TestAppContext) {
    let fake = Arc::new(FakeSession::new());
    let engine = fake_loaded_engine(cx, fake.clone());
    cx.update(|cx| engine.update(cx, |engine, cx| engine.toggle_play(cx)));
    cx.update(|cx| {
        engine.update(cx, |engine, cx| {
            // Mirror the real callback: it clears playing and arms eof together.
            let load = engine.load.as_ref().expect("load landed");
            load.transport.set_playing(false);
            load.transport.set_eof();
            engine.refresh_from_transport(cx);
        });
        assert_eq!(engine.read(cx).status(), Some(PlaybackStatus::Stopped));
        assert!(
            !engine
                .read(cx)
                .load
                .as_ref()
                .unwrap()
                .transport
                .is_playing(),
            "a seek while Stopped must not restart audio"
        );
    });
    assert_eq!(fake.pauses(), 1, "EOF must pause the still-running stream");
}

#[cfg(not(target_family = "wasm"))]
#[gpui_kit::test]
fn transport_stopping_under_a_playing_engine_pauses(cx: &mut TestAppContext) {
    let fake = Arc::new(FakeSession::new());
    let engine = fake_loaded_engine(cx, fake);
    cx.update(|cx| engine.update(cx, |engine, cx| engine.toggle_play(cx)));
    cx.update(|cx| {
        engine.update(cx, |engine, cx| {
            let load = engine.load.as_ref().expect("load landed");
            load.transport.set_playing(false);
            engine.refresh_from_transport(cx);
        });
        assert_eq!(engine.read(cx).status(), Some(PlaybackStatus::Paused));
    });
}

#[cfg(not(target_family = "wasm"))]
#[gpui_kit::test]
fn stopped_at_eof_restarts_from_zero(cx: &mut TestAppContext) {
    let fake = Arc::new(FakeSession::new());
    let engine = fake_loaded_engine(cx, fake);
    cx.update(|cx| {
        engine.update(cx, |engine, cx| engine.seek_fraction(1., cx));
        engine.update(cx, |engine, cx| engine.toggle_play(cx));
        assert_eq!(engine.read(cx).position(), Duration::ZERO);
        assert_eq!(engine.read(cx).status(), Some(PlaybackStatus::Playing));
    });
}

#[cfg(not(target_family = "wasm"))]
fn wait_for(deadline: Duration, mut check: impl FnMut() -> bool) -> bool {
    let end = std::time::Instant::now() + deadline;
    loop {
        if check() {
            return true;
        }
        if std::time::Instant::now() >= end {
            return check();
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Live-hardware variant; the seam tests above cover the logic everywhere.
#[cfg(not(target_family = "wasm"))]
#[gpui_kit::test]
#[ignore = "needs an openable output device; run explicitly with --ignored"]
fn live_playback_advances_pauses_and_stops(cx: &mut TestAppContext) {
    if !usable_output_device() {
        return;
    }
    let engine = loaded_sweep_engine(cx);
    cx.update(|cx| engine.update(cx, |engine, cx| engine.toggle_play(cx)));
    cx.run_until_parked();
    cx.update(|cx| assert_eq!(engine.read(cx).status(), Some(PlaybackStatus::Playing)));
    // The cpal callback thread runs on wall clock; the fake test clock cannot
    // drive it, so poll the playhead against a real deadline.
    let advanced = wait_for(Duration::from_secs(2), || {
        cx.update(|cx| engine.read(cx).position()) > Duration::ZERO
    });
    assert!(advanced, "the playhead never advanced on a live device");
    cx.update(|cx| engine.update(cx, |engine, cx| engine.toggle_play(cx)));
    cx.update(|cx| assert_eq!(engine.read(cx).status(), Some(PlaybackStatus::Paused)));
    std::thread::sleep(Duration::from_millis(120));
    let frozen = cx.update(|cx| engine.read(cx).position());
    std::thread::sleep(Duration::from_millis(300));
    assert_eq!(cx.update(|cx| engine.read(cx).position()), frozen);
    cx.update(|cx| {
        engine.update(cx, |engine, cx| engine.seek_fraction(0.75, cx));
        assert!(engine.read(cx).position() >= Duration::from_millis(700));
        engine.update(cx, |engine, cx| engine.stop(cx));
        assert_eq!(engine.read(cx).status(), Some(PlaybackStatus::Stopped));
        assert_eq!(engine.read(cx).position(), Duration::ZERO);
    });
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
