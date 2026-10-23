use std::sync::Arc;

use gpui_kit::{AppContext as _, TestAppContext};

use super::super::demos::DemoId;
use super::*;
use crate::features::wgpu::bgra_to_render_image;
use crate::platform::clock::{Duration, Instant};

fn image(seed: u8) -> Arc<gpui_kit::RenderImage> {
    bgra_to_render_image(1, 1, vec![seed, seed, seed, 255])
}

#[gpui_kit::test]
async fn set_demo_resets_playback_without_touching_playing(cx: &mut TestAppContext) {
    let state = cx.new(|cx| ShaderState::new(DemoId::Gradient, cx));
    let playing = cx.update(|cx| state.read(cx).is_playing());

    cx.update(|cx| {
        state.update(cx, |s, cx| {
            s.frame_done(Instant::now(), image(1), cx);
            s.set_slider(2.0, cx);
            s.set_demo(DemoId::Raymarch, cx);
        })
    });

    cx.update(|cx| {
        let read = state.read(cx);
        assert_eq!(read.demo(), DemoId::Raymarch);
        assert_eq!(read.slider(), demo(DemoId::Raymarch).slider.default);
        assert_eq!(
            read.params_snapshot(demo(DemoId::Raymarch)).mouse,
            (0.5, 0.5)
        );
        assert_eq!(read.time, 0.0);
        assert_eq!(read.frames_rendered(), 0);
        assert_eq!(read.is_playing(), playing);
    });
}

#[gpui_kit::test]
async fn set_slider_clamps_to_the_active_spec(cx: &mut TestAppContext) {
    let state = cx.new(|cx| ShaderState::new(DemoId::Gradient, cx));

    cx.update(|cx| {
        state.update(cx, |s, cx| {
            s.set_slider(9.0, cx);
        })
    });
    assert_eq!(
        cx.update(|cx| state.read(cx).slider()),
        demo(DemoId::Gradient).slider.max
    );

    cx.update(|cx| {
        state.update(cx, |s, cx| {
            s.set_slider(-4.0, cx);
        })
    });
    assert_eq!(
        cx.update(|cx| state.read(cx).slider()),
        demo(DemoId::Gradient).slider.min
    );
}

#[gpui_kit::test]
async fn pausing_while_in_flight_clears_the_flag(cx: &mut TestAppContext) {
    let state = cx.new(|cx| ShaderState::new(DemoId::Gradient, cx));

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
    let state = cx.new(|cx| ShaderState::new(DemoId::Gradient, cx));
    let now = Instant::now();
    let first = image(1);

    cx.update(|cx| {
        state.update(cx, |s, cx| {
            s.frame_done(now, first.clone(), cx);
            s.mark_frame_started(now, cx);
            s.toggle_playing(cx);
        })
    });

    cx.update(|cx| {
        state.update(cx, |s, cx| {
            s.frame_done(now, image(2), cx);
            assert!(!s.is_frame_in_flight());
            assert_eq!(s.frames_rendered(), 1);
        })
    });

    assert!(
        cx.update(|cx| state.update(cx, |s, _| s.take_stale_frames()))
            .is_empty()
    );
    let displayed = cx.update(|cx| state.read(cx).displayed.clone());
    assert!(displayed.is_some_and(|current| Arc::ptr_eq(&current, &first)));
}

#[gpui_kit::test]
async fn frame_done_notifies_once_and_retires_the_old_image(cx: &mut TestAppContext) {
    let state = cx.new(|cx| ShaderState::new(DemoId::Gradient, cx));
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
            s.mark_frame_started(now, cx);
            s.frame_done(now + Duration::from_millis(16), image(1), cx);
        })
    });
    cx.update(|cx| {
        state.update(cx, |s, cx| {
            s.mark_frame_started(now + Duration::from_millis(16), cx);
            s.frame_done(now + Duration::from_millis(32), image(2), cx);
        })
    });

    assert_eq!(notifications.get(), 2);
    cx.update(|cx| {
        let read = state.read(cx);
        assert_eq!(read.frames_rendered(), 2);
        assert!(!read.is_frame_in_flight());
        assert!((read.time - Duration::from_millis(32).as_secs_f32()).abs() < 1e-6);
    });
    assert_eq!(
        cx.update(|cx| state.update(cx, |s, _| s.take_stale_frames().len())),
        1
    );
    assert!(
        cx.update(|cx| state.update(cx, |s, _| s.take_stale_frames()))
            .is_empty()
    );
}

#[gpui_kit::test]
async fn frame_failed_stops_the_in_flight_flag_and_records_the_message(cx: &mut TestAppContext) {
    let state = cx.new(|cx| ShaderState::new(DemoId::Gradient, cx));

    cx.update(|cx| {
        state.update(cx, |s, cx| {
            s.mark_frame_started(Instant::now(), cx);
            s.frame_failed("adapter lost", cx);
        })
    });

    cx.update(|cx| {
        let read = state.read(cx);
        assert!(!read.is_frame_in_flight());
        assert_eq!(read.runtime_error(), Some("adapter lost"));
    });

    cx.update(|cx| {
        state.update(cx, |s, cx| s.clear_error(cx));
    });
    assert_eq!(
        cx.update(|cx| state.read(cx).runtime_error().map(str::to_string)),
        None
    );
}

#[gpui_kit::test]
async fn pacing_accumulates_from_the_previous_deadline(cx: &mut TestAppContext) {
    let state = cx.new(|cx| ShaderState::new(DemoId::Gradient, cx));
    let start = Instant::now();
    let interval = Duration::from_millis(16);

    cx.update(|cx| {
        state.update(cx, |s, cx| {
            assert_eq!(s.pacing_wait(start), Duration::ZERO);
            s.mark_frame_started(start, cx);
            s.frame_done(start, image(1), cx);
        })
    });

    cx.update(|cx| {
        state.update(cx, |s, cx| {
            assert_eq!(s.pacing_wait(start), interval);
            let second_start = start + interval;
            s.mark_frame_started(second_start, cx);
            s.frame_done(second_start + Duration::from_millis(5), image(2), cx);
            assert_eq!(
                s.pacing_wait(second_start + Duration::from_millis(5)),
                Duration::from_millis(11)
            );
            let third_start = start + interval * 2;
            s.mark_frame_started(third_start, cx);
            s.frame_done(third_start + Duration::from_millis(30), image(3), cx);
            assert_eq!(
                s.pacing_wait(third_start + Duration::from_millis(30)),
                Duration::ZERO
            );
        })
    });
}

#[gpui_kit::test]
async fn resuming_playback_resets_the_deadline(cx: &mut TestAppContext) {
    let state = cx.new(|cx| ShaderState::new(DemoId::Gradient, cx));
    let now = Instant::now();

    cx.update(|cx| {
        state.update(cx, |s, cx| {
            s.frame_done(now, image(1), cx);
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
async fn set_mouse_clamps_without_notifying(cx: &mut TestAppContext) {
    let state = cx.new(|cx| ShaderState::new(DemoId::Ripple, cx));
    let notifications = std::rc::Rc::new(std::cell::Cell::new(0usize));
    let counter = notifications.clone();
    cx.update(|cx| {
        cx.observe(&state, move |_, _| {
            counter.set(counter.get() + 1);
        })
        .detach();
    });

    cx.update(|cx| {
        state.update(cx, |s, cx| {
            s.set_mouse((0.25, 0.75), cx);
            s.set_mouse((-2.0, 4.0), cx);
        })
    });

    assert_eq!(notifications.get(), 0);
    assert_eq!(
        cx.update(|cx| state.read(cx).params_snapshot(demo(DemoId::Ripple)).mouse),
        (0.0, 1.0)
    );
}

#[gpui_kit::test]
async fn stale_frames_stay_capped(cx: &mut TestAppContext) {
    let state = cx.new(|cx| ShaderState::new(DemoId::Gradient, cx));
    let now = Instant::now();

    cx.update(|cx| {
        state.update(cx, |s, cx| {
            for seed in 0..8u8 {
                s.mark_frame_started(now, cx);
                s.frame_done(now, image(seed), cx);
            }
        })
    });

    assert_eq!(
        cx.update(|cx| state.update(cx, |s, _| s.take_stale_frames().len())),
        4
    );
}

#[gpui_kit::test]
async fn params_snapshot_carries_the_spec_resolution(cx: &mut TestAppContext) {
    let state = cx.new(|cx| ShaderState::new(DemoId::Warp, cx));

    let params = cx.update(|cx| {
        state.update(cx, |s, cx| {
            s.set_mouse((0.25, 0.75), cx);
            s.params_snapshot(demo(DemoId::Warp))
        })
    });

    assert_eq!(params.resolution, (512.0, 512.0));
    assert_eq!(params.mouse, (0.25, 0.75));
    assert_eq!(params.slider, demo(DemoId::Warp).slider.default);
}

#[gpui_kit::test]
async fn fit_is_taken_once_per_demo_install(cx: &mut TestAppContext) {
    let state = cx.new(|cx| ShaderState::new(DemoId::Gradient, cx));

    assert!(cx.update(|cx| state.update(cx, |s, _| s.take_fit())));
    assert!(!cx.update(|cx| state.update(cx, |s, _| s.take_fit())));

    cx.update(|cx| {
        state.update(cx, |s, cx| s.set_demo(DemoId::Warp, cx));
    });
    assert!(cx.update(|cx| state.update(cx, |s, _| s.take_fit())));
}
