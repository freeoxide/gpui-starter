use std::sync::Arc;
use std::time::Duration;

use gpui_kit::TestAppContext;
use gpui_kit::prelude::*;

use super::{PlayerEngine, TICK_MS, TransportStatus};
use crate::features::pages::music::{
    Album, AlbumId, Library, Playlist, PlaylistId, Track, TrackId,
};

const T0: TrackId = TrackId(0);
const T1: TrackId = TrackId(1);
const T2: TrackId = TrackId(2);

fn track(id: TrackId, duration_ms: u64) -> Track {
    Track {
        id,
        title: format!("Track {}", id.0).into(),
        artist: "Fixture".into(),
        duration_ms,
    }
}

fn test_library() -> Arc<Library> {
    let tracks = vec![track(T0, 1_000), track(T1, 500), track(T2, 200_000)];
    let albums = vec![Album {
        id: AlbumId(0),
        title: "Fixture Album".into(),
        artist: "Fixture".into(),
        year: 2026,
        track_ids: vec![T0, T1, T2],
    }];
    let playlists = vec![Playlist {
        id: PlaylistId(0),
        title: "Fixture List".into(),
        track_ids: vec![T0, T1],
    }];
    Arc::new(Library::from_parts(tracks, albums, playlists))
}

fn engine(cx: &mut TestAppContext) -> gpui_kit::Entity<PlayerEngine> {
    let library = test_library();
    cx.update(|cx| cx.new(|cx| PlayerEngine::new(library, cx)))
}

#[gpui_kit::test]
fn fresh_engine_is_stopped_and_silent(cx: &mut TestAppContext) {
    let engine = engine(cx);
    cx.read(|cx| {
        assert_eq!(engine.read(cx).status(), TransportStatus::Stopped);
        assert!(engine.read(cx).current_track().is_none());
        assert_eq!(engine.read(cx).position_ms(), 0);
        assert_eq!(engine.read(cx).duration_ms(), 0);
        assert_eq!(engine.read(cx).progress(), 0.);
        assert_eq!(engine.read(cx).volume(), 0.8);
        assert!(engine.read(cx).queue().is_empty());
        assert_eq!(engine.read(cx).current_ix(), None);
    });
}

#[gpui_kit::test]
fn play_queue_starts_at_the_requested_index(cx: &mut TestAppContext) {
    let engine = engine(cx);
    cx.update(|cx| engine.update(cx, |engine, cx| engine.play_queue(vec![T0, T1, T2], 1, cx)));
    cx.read(|cx| {
        assert_eq!(engine.read(cx).current_track().unwrap().id, T1);
        assert_eq!(engine.read(cx).status(), TransportStatus::Playing);
        assert_eq!(engine.read(cx).position_ms(), 0);
        assert_eq!(engine.read(cx).current_ix(), Some(1));
    });
}

#[gpui_kit::test]
fn play_queue_out_of_bounds_is_a_noop(cx: &mut TestAppContext) {
    let engine = engine(cx);
    cx.update(|cx| engine.update(cx, |engine, cx| engine.play_queue(vec![T0], 5, cx)));
    cx.read(|cx| {
        assert_eq!(engine.read(cx).status(), TransportStatus::Stopped);
        assert!(engine.read(cx).queue().is_empty());
    });
}

#[gpui_kit::test]
fn ticks_advance_position_while_playing(cx: &mut TestAppContext) {
    let engine = engine(cx);
    cx.update(|cx| engine.update(cx, |engine, cx| engine.play_queue(vec![T0, T1, T2], 0, cx)));
    cx.executor()
        .advance_clock(Duration::from_millis(3 * TICK_MS));
    cx.run_until_parked();
    cx.read(|cx| assert_eq!(engine.read(cx).position_ms(), 750));
}

// PlayerEngine::new spawns the tick; its entity updates are legal only from
// that task once the timer elapses, never at spawn or construction time.
#[gpui_kit::test]
fn engine_updates_only_when_the_tick_timer_fires(cx: &mut TestAppContext) {
    let engine = engine(cx);
    cx.update(|cx| engine.update(cx, |engine, cx| engine.play_queue(vec![T0, T1, T2], 0, cx)));
    cx.run_until_parked();
    cx.read(|cx| {
        assert_eq!(
            engine.read(cx).position_ms(),
            0,
            "the construction spawn must not update the entity before its timer"
        );
    });
    cx.executor().advance_clock(Duration::from_millis(TICK_MS));
    cx.run_until_parked();
    cx.read(|cx| assert_eq!(engine.read(cx).position_ms(), 250));
}

#[gpui_kit::test]
fn end_of_track_auto_advances_into_the_next_track(cx: &mut TestAppContext) {
    let engine = engine(cx);
    cx.update(|cx| engine.update(cx, |engine, cx| engine.play_queue(vec![T0, T1, T2], 1, cx)));
    cx.executor()
        .advance_clock(Duration::from_millis(3 * TICK_MS));
    cx.run_until_parked();
    cx.read(|cx| {
        assert_eq!(engine.read(cx).current_ix(), Some(2));
        assert_eq!(engine.read(cx).position_ms(), 250);
        assert_eq!(engine.read(cx).status(), TransportStatus::Playing);
    });
}

#[gpui_kit::test]
fn end_of_queue_stops_on_the_last_index(cx: &mut TestAppContext) {
    let engine = engine(cx);
    cx.update(|cx| engine.update(cx, |engine, cx| engine.play_queue(vec![T1], 0, cx)));
    cx.executor()
        .advance_clock(Duration::from_millis(4 * TICK_MS));
    cx.run_until_parked();
    cx.read(|cx| {
        assert_eq!(engine.read(cx).status(), TransportStatus::Stopped);
        assert_eq!(engine.read(cx).position_ms(), 0);
        assert_eq!(engine.read(cx).current_ix(), Some(0));
    });
}

#[gpui_kit::test]
fn paused_engine_consumes_ticks_without_moving(cx: &mut TestAppContext) {
    let engine = engine(cx);
    cx.update(|cx| {
        engine.update(cx, |engine, cx| {
            engine.play_queue(vec![T0, T1, T2], 0, cx);
            engine.toggle_playback(cx);
        })
    });
    cx.executor()
        .advance_clock(Duration::from_millis(4 * TICK_MS));
    cx.run_until_parked();
    cx.read(|cx| {
        assert_eq!(engine.read(cx).status(), TransportStatus::Paused);
        assert_eq!(engine.read(cx).position_ms(), 0);
    });
}

#[gpui_kit::test]
fn next_at_the_last_index_stops_in_place(cx: &mut TestAppContext) {
    let engine = engine(cx);
    cx.update(|cx| {
        engine.update(cx, |engine, cx| {
            engine.play_queue(vec![T0, T1], 1, cx);
            engine.next(cx);
        })
    });
    cx.read(|cx| {
        assert_eq!(engine.read(cx).status(), TransportStatus::Stopped);
        assert_eq!(engine.read(cx).position_ms(), 0);
        assert_eq!(engine.read(cx).current_ix(), Some(1));
    });
}

#[gpui_kit::test]
fn previous_at_zero_only_rewinds_position(cx: &mut TestAppContext) {
    let engine = engine(cx);
    cx.update(|cx| {
        engine.update(cx, |engine, cx| {
            engine.play_queue(vec![T0, T1, T2], 0, cx);
            engine.seek_ms(400, cx);
            engine.previous(cx);
        })
    });
    cx.read(|cx| {
        assert_eq!(engine.read(cx).current_ix(), Some(0));
        assert_eq!(engine.read(cx).position_ms(), 0);
        assert_eq!(engine.read(cx).status(), TransportStatus::Playing);
    });
}

#[gpui_kit::test]
fn track_changes_while_paused_stay_paused(cx: &mut TestAppContext) {
    let engine = engine(cx);
    cx.update(|cx| {
        engine.update(cx, |engine, cx| {
            engine.play_queue(vec![T0, T1, T2], 1, cx);
            engine.toggle_playback(cx);
            engine.next(cx);
        })
    });
    cx.read(|cx| {
        assert_eq!(engine.read(cx).status(), TransportStatus::Paused);
        assert_eq!(engine.read(cx).current_ix(), Some(2));
    });
    cx.update(|cx| engine.update(cx, |engine, cx| engine.previous(cx)));
    cx.read(|cx| {
        assert_eq!(engine.read(cx).status(), TransportStatus::Paused);
        assert_eq!(engine.read(cx).current_ix(), Some(1));
    });
}

#[gpui_kit::test]
fn toggle_playback_requires_a_current_track(cx: &mut TestAppContext) {
    let engine = engine(cx);
    cx.update(|cx| engine.update(cx, |engine, cx| engine.toggle_playback(cx)));
    cx.read(|cx| assert_eq!(engine.read(cx).status(), TransportStatus::Stopped));

    cx.update(|cx| {
        engine.update(cx, |engine, cx| {
            engine.play_queue(vec![T0, T1, T2], 0, cx);
            engine.toggle_playback(cx);
        })
    });
    cx.read(|cx| assert_eq!(engine.read(cx).status(), TransportStatus::Paused));
    cx.update(|cx| engine.update(cx, |engine, cx| engine.toggle_playback(cx)));
    cx.read(|cx| assert_eq!(engine.read(cx).status(), TransportStatus::Playing));
}

#[gpui_kit::test]
fn seek_and_volume_clamp(cx: &mut TestAppContext) {
    let engine = engine(cx);
    cx.update(|cx| {
        engine.update(cx, |engine, cx| {
            engine.play_queue(vec![T0, T1, T2], 2, cx);
            engine.seek_ms(600_000, cx);
            engine.set_volume(2., cx);
        })
    });
    cx.read(|cx| {
        assert_eq!(engine.read(cx).position_ms(), 200_000);
        assert_eq!(engine.read(cx).volume(), 1.);
    });
    cx.update(|cx| engine.update(cx, |engine, cx| engine.set_volume(-1., cx)));
    cx.read(|cx| assert_eq!(engine.read(cx).volume(), 0.));
}

#[gpui_kit::test]
fn seek_without_a_current_track_is_a_noop(cx: &mut TestAppContext) {
    let engine = engine(cx);
    cx.update(|cx| engine.update(cx, |engine, cx| engine.seek_ms(1_000, cx)));
    cx.read(|cx| assert_eq!(engine.read(cx).position_ms(), 0));
}

#[gpui_kit::test]
fn enqueue_selects_the_first_track_but_stays_stopped(cx: &mut TestAppContext) {
    let engine = engine(cx);
    cx.update(|cx| engine.update(cx, |engine, cx| engine.enqueue(T1, cx)));
    cx.read(|cx| {
        assert_eq!(engine.read(cx).status(), TransportStatus::Stopped);
        assert_eq!(engine.read(cx).current_ix(), Some(0));
        assert_eq!(engine.read(cx).queue(), &[T1]);
    });
    cx.update(|cx| {
        engine.update(cx, |engine, cx| {
            engine.enqueue(T0, cx);
            engine.toggle_playback(cx);
        })
    });
    cx.read(|cx| {
        assert_eq!(engine.read(cx).status(), TransportStatus::Playing);
        assert_eq!(engine.read(cx).current_ix(), Some(0));
        assert_eq!(engine.read(cx).queue(), &[T1, T0]);
    });
}

#[gpui_kit::test]
fn clear_queue_resets_everything(cx: &mut TestAppContext) {
    let engine = engine(cx);
    cx.update(|cx| {
        engine.update(cx, |engine, cx| {
            engine.play_queue(vec![T0, T1, T2], 2, cx);
            engine.clear_queue(cx);
        })
    });
    cx.read(|cx| {
        assert_eq!(engine.read(cx).status(), TransportStatus::Stopped);
        assert_eq!(engine.read(cx).position_ms(), 0);
        assert_eq!(engine.read(cx).current_ix(), None);
        assert!(engine.read(cx).queue().is_empty());
    });
}

#[gpui_kit::test]
fn duplicate_track_ids_play_in_queue_order(cx: &mut TestAppContext) {
    let engine = engine(cx);
    cx.update(|cx| engine.update(cx, |engine, cx| engine.play_queue(vec![T0, T0, T0], 2, cx)));
    cx.read(|cx| assert_eq!(engine.read(cx).current_ix(), Some(2)));
}

#[gpui_kit::test]
fn global_handle_round_trips_the_entity(cx: &mut TestAppContext) {
    cx.read(|cx| assert!(crate::features::pages::music::player(cx).is_none()));
    let engine = engine(cx);
    cx.update(|cx| {
        cx.set_global(super::super::MusicPlayerGlobal(engine.clone()));
    });
    cx.read(|cx| {
        let stored = crate::features::pages::music::player(cx).unwrap();
        assert!(stored == engine);
    });
}
