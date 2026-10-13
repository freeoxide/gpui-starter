use std::sync::Arc;
use std::time::Duration;

use gpui_kit::{TestAppContext, Window, prelude::*};

use super::NowPlayingBar;
use crate::features::pages::music::{Library, PlayerEngine, TransportStatus};

const T0: crate::features::pages::music::TrackId = crate::features::pages::music::TrackId(0);
const T1: crate::features::pages::music::TrackId = crate::features::pages::music::TrackId(1);

fn fixture_track(
    id: crate::features::pages::music::TrackId,
    duration_ms: u64,
) -> crate::features::pages::music::Track {
    crate::features::pages::music::Track {
        id,
        title: format!("Track {}", id.0).into(),
        artist: "Fixture".into(),
        duration_ms,
    }
}

fn two_track_engine(cx: &mut TestAppContext) -> gpui_kit::Entity<PlayerEngine> {
    let library = Arc::new(Library::from_parts(
        vec![fixture_track(T0, 200_000), fixture_track(T1, 100_000)],
        Vec::new(),
        Vec::new(),
    ));
    cx.update(|cx| cx.new(|cx| PlayerEngine::new(library, cx)))
}

struct BarHost {
    bar: gpui_kit::Entity<NowPlayingBar>,
}

impl Render for BarHost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.bar.clone()
    }
}

fn open_bar_window(
    cx: &mut TestAppContext,
    engine: gpui_kit::Entity<PlayerEngine>,
) -> gpui_kit::Entity<NowPlayingBar> {
    let mut bar = None;
    cx.add_window(|window, cx| {
        let entity = cx.new(|cx| NowPlayingBar::new(engine, window, cx));
        bar = Some(entity.clone());
        BarHost { bar: entity }
    });
    bar.unwrap()
}

#[gpui_kit::test]
fn bar_draws_idle_without_touching_the_engine(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let engine = two_track_engine(cx);
    let bar = open_bar_window(cx, engine.clone());
    cx.read(|cx| {
        assert_eq!(engine.read(cx).status(), TransportStatus::Stopped);
        assert_eq!(engine.read(cx).position_ms(), 0);
        assert!(bar.read(cx).scrub.is_none());
    });
}

#[gpui_kit::test]
fn bar_draws_playing_without_status_drift(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let engine = two_track_engine(cx);
    cx.update(|cx| engine.update(cx, |engine, cx| engine.play_queue(vec![T0, T1], 0, cx)));
    let bar = open_bar_window(cx, engine.clone());
    cx.read(|cx| {
        assert_eq!(engine.read(cx).status(), TransportStatus::Playing);
        assert_eq!(engine.read(cx).position_ms(), 0);
        assert_eq!(bar.read(cx).scrub, None);
    });
    cx.executor().advance_clock(Duration::from_millis(250));
    cx.run_until_parked();
    cx.read(|cx| assert_eq!(engine.read(cx).position_ms(), 250));
}

#[gpui_kit::test]
fn bar_draws_paused_and_ignores_ticks(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let engine = two_track_engine(cx);
    cx.update(|cx| {
        engine.update(cx, |engine, cx| {
            engine.play_queue(vec![T0, T1], 0, cx);
            engine.toggle_playback(cx);
        })
    });
    let bar = open_bar_window(cx, engine.clone());
    cx.read(|cx| {
        assert_eq!(engine.read(cx).status(), TransportStatus::Paused);
        assert_eq!(bar.read(cx).scrub, None);
    });
    cx.executor().advance_clock(Duration::from_millis(1000));
    cx.run_until_parked();
    cx.read(|cx| assert_eq!(engine.read(cx).position_ms(), 0));
}

#[gpui_kit::test]
fn seek_release_commits_against_the_track_duration(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let engine = two_track_engine(cx);
    let bar = open_bar_window(cx, engine.clone());
    cx.update(|cx| {
        engine.update(cx, |engine, cx| {
            engine.play_queue(vec![T0, T1], 0, cx);
        });
        bar.update(cx, |bar, cx| bar.on_seek_release(0.5, cx));
    });
    cx.read(|cx| assert_eq!(engine.read(cx).position_ms(), 100_000));
}

#[gpui_kit::test]
fn volume_change_forwards_to_the_engine(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let engine = two_track_engine(cx);
    let bar = open_bar_window(cx, engine.clone());
    cx.update(|cx| {
        bar.update(cx, |bar, cx| bar.on_volume_change(0.25, cx));
    });
    cx.read(|cx| assert_eq!(engine.read(cx).volume(), 0.25));
}

#[gpui_kit::test]
fn transport_handlers_drive_the_engine(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let engine = two_track_engine(cx);
    let bar = open_bar_window(cx, engine.clone());
    cx.update(|cx| {
        engine.update(cx, |engine, cx| {
            engine.play_queue(vec![T0, T1], 0, cx);
        });
        bar.update(cx, |bar, cx| bar.on_next(cx));
    });
    cx.read(|cx| assert_eq!(engine.read(cx).current_ix(), Some(1)));

    cx.update(|cx| {
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
