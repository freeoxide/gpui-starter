use std::sync::Arc;

use gpui_kit::{AppContext as _, TestAppContext};

use super::super::frame::{FpsTracker, bgra_to_render_image};
use super::*;
use crate::platform::clock::{Duration, Instant};

fn image(seed: u8) -> Arc<gpui_kit::RenderImage> {
    bgra_to_render_image(1, 1, vec![seed, seed, seed, 255])
}

#[gpui_kit::test]
async fn viewport_sizes_quantize_to_sixteen_pixel_steps(cx: &mut TestAppContext) {
    let state = cx.new(|cx| WgpuPageState::new(cx));

    cx.update(|cx| state.update(cx, |s, cx| s.viewport_changed((100, 100), cx)));
    assert_eq!(cx.update(|cx| state.read(cx).pending_size()), (112, 112));

    cx.update(|cx| state.update(cx, |s, cx| s.viewport_changed((4095, 20), cx)));
    assert_eq!(cx.update(|cx| state.read(cx).pending_size()), (4096, 64));

    cx.update(|cx| state.update(cx, |s, cx| s.viewport_changed((8, 8), cx)));
    assert_eq!(cx.update(|cx| state.read(cx).pending_size()), (64, 64));
}

#[gpui_kit::test]
async fn pausing_while_in_flight_clears_the_flag(cx: &mut TestAppContext) {
    let state = cx.new(|cx| WgpuPageState::new(cx));

    cx.update(|cx| {
        state.update(cx, |s, cx| {
            s.mark_frame_started(Instant::now(), cx);
            s.toggle_playing(cx);
        })
    });

    assert!(!cx.update(|cx| state.read(cx).is_playing()));
    assert!(!cx.update(|cx| state.read(cx).is_frame_in_flight()));
}

#[gpui_kit::test]
async fn completing_a_paused_frame_drops_the_image_without_landing(cx: &mut TestAppContext) {
    let state = cx.new(|cx| WgpuPageState::new(cx));
    let now = Instant::now();
    let first = image(1);

    cx.update(|cx| {
        state.update(cx, |s, cx| {
            s.frame_done(now, first.clone(), 10.0, cx);
            s.mark_frame_started(now, cx);
            s.toggle_playing(cx);
        })
    });

    cx.update(|cx| {
        state.update(cx, |s, cx| {
            s.frame_done(now, image(2), 10.0, cx);
            assert!(!s.is_frame_in_flight());
            assert_eq!(s.frames_rendered(), 1);
        })
    });

    assert!(
        cx.update(|cx| state.update(cx, |s, _| s.take_stale_frames()))
            .is_empty()
    );
    let displayed = cx.update(|cx| state.read(cx).displayed().cloned());
    assert!(displayed.is_some_and(|current| Arc::ptr_eq(&current, &first)));
}

#[gpui_kit::test]
async fn frame_done_notifies_once_and_retires_the_old_image(cx: &mut TestAppContext) {
    let state = cx.new(|cx| WgpuPageState::new(cx));
    let notifications = std::rc::Rc::new(std::cell::Cell::new(0usize));
    let counter = notifications.clone();
    cx.update(|cx| {
        cx.observe(&state, move |_, _| {
            counter.set(counter.get() + 1);
        })
        .detach();
    });

    let now = Instant::now();
    cx.update(|cx| {
        state.update(cx, |s, cx| {
            s.frame_done(now, image(1), 10.0, cx);
        })
    });
    cx.update(|cx| {
        state.update(cx, |s, cx| {
            s.frame_done(now, image(2), 20.0, cx);
        })
    });

    assert_eq!(notifications.get(), 2);
    cx.update(|cx| {
        let read = state.read(cx);
        assert_eq!(read.frames_rendered(), 2);
        assert!(!read.is_frame_in_flight());
    });
    assert_eq!(
        cx.update(|cx| state.update(cx, |s, _| s.take_stale_frames().len())),
        1
    );
    let ema = cx.update(|cx| state.read(cx).engine_ms());
    assert!((ema - (2.0 + 0.2 * (20.0 - 2.0))).abs() < 1e-4);
}

#[gpui_kit::test]
async fn errors_set_and_clear(cx: &mut TestAppContext) {
    let state = cx.new(|cx| WgpuPageState::new(cx));

    cx.update(|cx| state.update(cx, |s, cx| s.frame_failed("adapter lost", cx)));
    assert_eq!(
        cx.update(|cx| state.read(cx).runtime_error().map(str::to_string)),
        Some("adapter lost".to_string())
    );

    cx.update(|cx| {
        state.update(cx, |s, cx| s.clear_error(cx));
    });
    assert_eq!(
        cx.update(|cx| state.read(cx).runtime_error().map(str::to_string)),
        None
    );

    cx.update(|cx| {
        state.update(cx, |s, cx| {
            s.frame_failed("device lost", cx);
            s.set_demo_ix(1, cx);
        })
    });
    assert_eq!(
        cx.update(|cx| state.read(cx).runtime_error().map(str::to_string)),
        None
    );
}

#[gpui_kit::test]
async fn pacing_accumulates_from_the_previous_deadline(cx: &mut TestAppContext) {
    let state = cx.new(|cx| WgpuPageState::new(cx));
    let start = Instant::now();
    let interval = Duration::from_millis(16);

    cx.update(|cx| {
        state.update(cx, |s, cx| {
            assert_eq!(s.pacing_wait(start), Duration::ZERO);
            s.mark_frame_started(start, cx);
            s.frame_done(start, image(1), 5.0, cx);
        })
    });

    cx.update(|cx| {
        state.update(cx, |s, cx| {
            assert_eq!(s.pacing_wait(start), interval);
            // Frame two starts on the deadline, renders 5 ms, lands 5 ms
            // before the next one: the sleep shrinks by the render time.
            let second_start = start + interval;
            s.mark_frame_started(second_start, cx);
            s.frame_done(second_start + Duration::from_millis(5), image(2), 5.0, cx);
            assert_eq!(
                s.pacing_wait(second_start + Duration::from_millis(5)),
                Duration::from_millis(11)
            );
            // A frame that overruns its slot clamps the deadline to now
            // instead of accruing debt.
            let third_start = start + interval * 2;
            s.mark_frame_started(third_start, cx);
            s.frame_done(third_start + Duration::from_millis(30), image(3), 30.0, cx);
            assert_eq!(
                s.pacing_wait(third_start + Duration::from_millis(30)),
                Duration::ZERO
            );
        })
    });
}

#[gpui_kit::test]
async fn frame_dt_spans_the_paced_period_not_the_kick_gap(cx: &mut TestAppContext) {
    let state = cx.new(|cx| WgpuPageState::new(cx));
    let kick = Instant::now();

    cx.update(|cx| state.update(cx, |s, cx| s.mark_frame_started(kick, cx)));

    let after_wait = kick + Duration::from_millis(16);
    let dt = cx.update(|cx| state.read(cx).frame_dt(after_wait));
    assert!((dt - Duration::from_millis(16).as_secs_f32()).abs() < 1e-6);

    let overrun = cx.update(|cx| state.read(cx).frame_dt(kick + Duration::from_millis(300)));
    assert_eq!(overrun, 0.25);
}

#[gpui_kit::test]
async fn resuming_playback_resets_the_deadline(cx: &mut TestAppContext) {
    let state = cx.new(|cx| WgpuPageState::new(cx));
    let now = Instant::now();

    cx.update(|cx| {
        state.update(cx, |s, cx| {
            s.frame_done(now, image(1), 5.0, cx);
            s.toggle_playing(cx);
            s.toggle_playing(cx);
        })
    });

    assert_eq!(
        cx.update(|cx| state.read(cx).pacing_wait(now + Duration::from_millis(60))),
        Duration::ZERO
    );
}

#[gpui_kit::test]
async fn particle_count_only_accepts_the_supported_tiers(cx: &mut TestAppContext) {
    let state = cx.new(|cx| WgpuPageState::new(cx));

    cx.update(|cx| state.update(cx, |s, cx| s.set_particle_count(123, cx)));
    assert_eq!(cx.update(|cx| state.read(cx).particle_count()), 1_000_000);

    cx.update(|cx| state.update(cx, |s, cx| s.set_particle_count(500_000, cx)));
    assert_eq!(cx.update(|cx| state.read(cx).particle_count()), 500_000);
}

#[test]
fn fps_tracker_window_from_state_perspective() {
    let now = Instant::now();
    let mut tracker = FpsTracker::new();
    tracker.push(now);
    tracker.push(now + Duration::from_millis(16));
    assert_eq!(tracker.fps(), 3);
}
