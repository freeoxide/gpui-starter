use super::*;

#[test]
fn catalog_ids_are_unique_per_collection() {
    let library = Library::catalog();
    let track_ids: Vec<_> = library.tracks.iter().map(|t| t.id).collect();
    let album_ids: Vec<_> = library.albums.iter().map(|a| a.id).collect();
    let playlist_ids: Vec<_> = library.playlists.iter().map(|p| p.id).collect();
    assert_unique(track_ids);
    assert_unique(album_ids);
    assert_unique(playlist_ids);
}

#[test]
fn every_album_and_playlist_track_resolves() {
    let library = Library::catalog();
    for album in library.albums() {
        assert!(!album.track_ids.is_empty());
        for &track_id in &album.track_ids {
            assert!(
                library.track(track_id).is_some(),
                "album {:?} track {track_id:?} missing",
                album.id
            );
        }
    }
    for playlist in library.playlists() {
        for &track_id in &playlist.track_ids {
            assert!(
                library.track(track_id).is_some(),
                "playlist {:?} track {track_id:?} missing",
                playlist.id
            );
        }
    }
}

#[test]
fn fixture_shape_matches_the_plan() {
    let library = Library::catalog();
    let track_count: usize = library.albums.iter().map(|a| a.track_ids.len()).sum();
    assert_eq!(library.albums().len(), 8);
    assert!((48..=56).contains(&track_count), "got {track_count}");
    assert_eq!(library.playlists().len(), 2);
    assert_eq!(library.playlist(PlaylistId(0)).unwrap().track_ids.len(), 12);
    assert_eq!(
        library.playlist(PlaylistId(1)).unwrap().track_ids.len(),
        120
    );
    for track in &library.tracks {
        assert!((90_000..=300_000).contains(&track.duration_ms));
    }
}

#[test]
fn track_album_resolves_for_every_track() {
    let library = Library::catalog();
    for track in &library.tracks {
        let album_id = library
            .track_album(track.id)
            .unwrap_or_else(|| panic!("track {:?} has no album", track.id));
        let album = library.album(album_id).unwrap();
        assert!(album.track_ids.contains(&track.id));
        assert!(library.cover(album_id).is_some());
    }
}

#[test]
fn catalog_returns_one_shared_instance() {
    let a = Library::catalog();
    let b = Library::catalog();
    assert!(Arc::ptr_eq(&a, &b));
}

#[test]
fn format_time_renders_minutes_and_zero_padded_seconds() {
    assert_eq!(format_time(0), "0:00");
    assert_eq!(format_time(59_999), "0:59");
    assert_eq!(format_time(60_000), "1:00");
    assert_eq!(format_time(3_723_000), "62:03");
}

#[test]
fn move_within_swaps_and_refuses_out_of_bounds() {
    let mut items = vec![10, 20, 30];
    assert!(move_within(&mut items, 1, -1));
    assert_eq!(items, vec![20, 10, 30]);
    assert!(move_within(&mut items, 1, 1));
    assert_eq!(items, vec![20, 30, 10]);

    assert!(!move_within(&mut items, 0, -1));
    assert!(!move_within(&mut items, 2, 1));
    assert!(!move_within(&mut items, 3, -1));
    assert_eq!(items, vec![20, 30, 10]);
}

fn assert_unique(ids: Vec<impl PartialEq + std::fmt::Debug>) {
    for (i, id) in ids.iter().enumerate() {
        for other in &ids[i + 1..] {
            assert_ne!(id, other, "duplicate id {id:?}");
        }
    }
}
