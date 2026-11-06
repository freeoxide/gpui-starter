use std::sync::Arc;
use std::time::Duration;

use gpui_kit::component::slider::SliderState;
use gpui_kit::prelude::*;
use gpui_kit::{Axis, Entity, TestAppContext, VisualTestContext, Window, point, px, size};

use super::NowPlayingBar;
use crate::features::pages::music::{Library, PlayerEngine, Track, TrackId, TransportStatus};

const T0: TrackId = TrackId(0);
const T1: TrackId = TrackId(1);

fn fixture_track(id: TrackId, duration_ms: u64) -> Track {
    Track {
        id,
        title: format!("Track {}", id.0).into(),
        artist: "Fixture".into(),
        duration_ms,
    }
}

fn two_track_engine(cx: &mut TestAppContext) -> Entity<PlayerEngine> {
    let library = Arc::new(Library::from_parts(
        vec![fixture_track(T0, 200_000), fixture_track(T1, 100_000)],
        Vec::new(),
        Vec::new(),
    ));
    cx.update(|cx| cx.new(|cx| PlayerEngine::new(library, cx)))
}

struct BarHost {
    bar: Entity<NowPlayingBar>,
}

impl Render for BarHost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.bar.clone()
    }
}

/// Mounts the bar in a drawable window so the sliders carry real bounds and
/// the bar's render-side seek writes are observable on the slider entity.
fn open_bar_window(
    cx: &mut TestAppContext,
    engine: Entity<PlayerEngine>,
) -> (Entity<NowPlayingBar>, VisualTestContext) {
    cx.update(gpui_kit::component::theme::init);
    let mut bar = None;
    let handle = cx.open_window(size(px(800.), px(600.)), |window, cx| {
        let entity = cx.new(|cx| NowPlayingBar::new(engine, window, cx));
        bar = Some(entity.clone());
        BarHost { bar: entity }
    });
    let bar = bar.expect("bar entity created");
    let mut cx = VisualTestContext::from_window(*handle, cx);
    cx.update(|window, cx| window.draw(cx).clear(cx));
    (bar, cx)
}

fn slider_point_at(
    cx: &TestAppContext,
    slider: &Entity<SliderState>,
    fraction: f32,
) -> gpui_kit::Point<gpui_kit::Pixels> {
    let bounds = cx.read(|cx| slider.read(cx).bounds());
    point(
        bounds.origin.x + bounds.size.width * fraction,
        bounds.origin.y + bounds.size.height * 0.5,
    )
}

#[gpui_kit::test]
fn bar_draws_idle_without_touching_the_engine(cx: &mut TestAppContext) {
    let engine = two_track_engine(cx);
    let (bar, cx) = open_bar_window(cx, engine);
    cx.read(|cx| {
        let bar = bar.read(cx);
        assert!(bar.scrub.is_none());
        assert_eq!(bar.seek.read(cx).value().start(), 0.);
        assert_eq!(bar.volume.read(cx).value().start(), 0.8);
    });
}

#[gpui_kit::test]
fn seek_slider_tracks_engine_progress_while_playing(cx: &mut TestAppContext) {
    let engine = two_track_engine(cx);
    cx.update(|cx| {
        engine.update(cx, |engine, cx| engine.play_queue(vec![T0, T1], 0, cx));
    });
    let (bar, mut cx) = open_bar_window(cx, engine.clone());
    cx.read(|cx| assert_eq!(bar.read(cx).seek.read(cx).value().start(), 0.));

    cx.executor().advance_clock(Duration::from_millis(250));
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));

    let (progress, seek_value) = cx.read(|cx| {
        (
            engine.read(cx).progress(),
            bar.read(cx).seek.read(cx).value().start(),
        )
    });
    assert!(
        (progress - 0.00125).abs() < 1e-6,
        "the engine must advance first, got {progress}"
    );
    assert!(
        (seek_value - progress).abs() < 1e-6,
        "render must feed engine progress into the seek slider"
    );
}

#[gpui_kit::test]
fn paused_ticks_leave_the_seek_slider_at_zero(cx: &mut TestAppContext) {
    let engine = two_track_engine(cx);
    cx.update(|cx| {
        engine.update(cx, |engine, cx| {
            engine.play_queue(vec![T0, T1], 0, cx);
            engine.toggle_playback(cx);
        })
    });
    let (bar, mut cx) = open_bar_window(cx, engine.clone());

    cx.executor().advance_clock(Duration::from_millis(1000));
    cx.run_until_parked();
    cx.update(|window, cx| window.draw(cx).clear(cx));

    cx.read(|cx| {
        assert_eq!(engine.read(cx).position_ms(), 0);
        assert_eq!(
            bar.read(cx).seek.read(cx).value().start(),
            0.,
            "a paused engine must not drag the seek slider"
        );
    });
}

/// Drives the real SliderState entities, so the bar's subscriptions — not the
/// forwarder methods — are the code under test.
#[gpui_kit::test]
fn seek_drag_scrubs_and_release_commits_the_position(cx: &mut TestAppContext) {
    let engine = two_track_engine(cx);
    cx.update(|cx| {
        engine.update(cx, |engine, cx| engine.play_queue(vec![T0, T1], 0, cx));
    });
    let (bar, mut cx) = open_bar_window(cx, engine.clone());

    let seek = cx.read(|cx| bar.read(cx).seek.clone());
    let at_half = slider_point_at(&cx, &seek, 0.5);
    cx.update(|window, cx| {
        seek.update(cx, |state, cx| {
            state.update_value_by_position(Axis::Horizontal, at_half, false, window, cx)
        });
    });
    cx.run_until_parked();
    cx.read(|cx| {
        let scrub = bar.read(cx).scrub;
        assert!(
            scrub.is_some_and(|fraction| (fraction - 0.5).abs() < 0.01),
            "a drag Change must set the scrub fraction, got {scrub:?}"
        );
    });

    cx.update(|_window, cx| seek.update(cx, |state, cx| state.handle_release(cx)));
    cx.run_until_parked();
    cx.read(|cx| {
        assert_eq!(bar.read(cx).scrub, None, "release must clear the scrub");
        let position = engine.read(cx).position_ms();
        assert!(
            (position as i64 - 100_000).abs() < 2_000,
            "release must seek to half the 200s track, got {position}"
        );
    });
}

#[gpui_kit::test]
fn volume_drag_updates_the_engine_through_the_subscription(cx: &mut TestAppContext) {
    let engine = two_track_engine(cx);
    let (bar, mut cx) = open_bar_window(cx, engine.clone());

    let volume = cx.read(|cx| bar.read(cx).volume.clone());
    let at_quarter = slider_point_at(&cx, &volume, 0.25);
    cx.update(|window, cx| {
        volume.update(cx, |state, cx| {
            state.update_value_by_position(Axis::Horizontal, at_quarter, false, window, cx)
        });
    });
    cx.run_until_parked();
    cx.read(|cx| {
        let set = engine.read(cx).volume();
        assert!(
            (set - 0.25).abs() < 0.01,
            "the volume Change event must reach the engine, got {set}"
        );
    });
}

#[gpui_kit::test]
fn transport_handlers_drive_the_engine(cx: &mut TestAppContext) {
    let engine = two_track_engine(cx);
    let (bar, mut cx) = open_bar_window(cx, engine.clone());
    cx.update(|_window, cx| {
        engine.update(cx, |engine, cx| {
            engine.play_queue(vec![T0, T1], 0, cx);
        });
        bar.update(cx, |bar, cx| bar.on_next(cx));
    });
    cx.read(|cx| assert_eq!(engine.read(cx).current_ix(), Some(1)));

    cx.update(|_window, cx| {
        bar.update(cx, |bar, cx| {
            bar.on_previous(cx);
            bar.on_toggle_play(cx);
        });
    });
    cx.read(|cx| {
        assert_eq!(engine.read(cx).current_ix(), Some(0));
        assert_eq!(engine.read(cx).status(), TransportStatus::Paused);
        assert_eq!(engine.read(cx).position_ms(), 0);
    });
}
