use std::collections::BTreeMap;
use std::sync::{Arc, LazyLock};

use gpui_kit::{Image, ImageSource, SharedString};
use serde::{Deserialize, Serialize};

use super::covers;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TrackId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AlbumId(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct PlaylistId(pub u32);

#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct Track {
    pub id: TrackId,
    pub title: SharedString,
    pub artist: SharedString,
    pub duration_ms: u64,
}

#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct Album {
    pub id: AlbumId,
    pub title: SharedString,
    pub artist: SharedString,
    pub year: u16,
    pub track_ids: Vec<TrackId>,
}

#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct Playlist {
    pub id: PlaylistId,
    pub title: SharedString,
    pub track_ids: Vec<TrackId>,
}

/// Immutable demo catalog. `catalog()` returns the same instance on every call.
pub struct Library {
    tracks: Vec<Track>,
    albums: Vec<Album>,
    playlists: Vec<Playlist>,
    covers: BTreeMap<AlbumId, Arc<Image>>,
}

static CATALOG: LazyLock<Arc<Library>> = LazyLock::new(|| Arc::new(build_catalog()));

const ALBUM_FIXTURES: &[(&str, &str, u16, usize)] = &[
    ("Signal Drift", "Vector Field", 2019, 7),
    ("Room Tone", "Halo Form", 2021, 6),
    ("Low Meridian", "Basin", 2016, 7),
    ("Tape Bloom", "Night Cassette", 2023, 6),
    ("Iron Chorus", "Ferrous", 2018, 7),
    ("Static Hymn", "Sable Choir", 2015, 6),
    ("North Lines", "Meridian Wire", 2020, 6),
    ("Vacancy Loops", "Pale Motel", 2022, 7),
];

const TITLE_FIRST: &[&str] = &[
    "Copper", "Glass", "Night", "Slow", "Paper", "Neon", "Winter", "Salt", "Amber", "Quiet",
    "Velvet", "Iron", "Hollow", "Pale", "Signal", "Coastal",
];

const TITLE_SECOND: &[&str] = &[
    "Season",
    "Arcade",
    "Distance",
    "Weather",
    "Machines",
    "Harbor",
    "Static",
    "Lantern",
    "Fields",
    "Corridor",
    "Echoes",
    "Runway",
    "Cathedral",
    "Tide",
    "Postcard",
    "Almanac",
];

fn track_title(album_ix: usize, track_ix: usize) -> SharedString {
    let first = TITLE_FIRST[(album_ix * 5 + track_ix * 7) % TITLE_FIRST.len()];
    let second = TITLE_SECOND[(album_ix * 11 + track_ix * 3) % TITLE_SECOND.len()];
    format!("{first} {second}").into()
}

impl Library {
    pub fn catalog() -> Arc<Library> {
        CATALOG.clone()
    }

    pub fn track(&self, id: TrackId) -> Option<&Track> {
        self.tracks.iter().find(|track| track.id == id)
    }

    pub fn album(&self, id: AlbumId) -> Option<&Album> {
        self.albums.iter().find(|album| album.id == id)
    }

    /// Album owning this track, for cover resolution; always Some for
    /// catalog tracks.
    pub fn track_album(&self, id: TrackId) -> Option<AlbumId> {
        self.albums
            .iter()
            .find(|album| album.track_ids.contains(&id))
            .map(|album| album.id)
    }

    pub fn playlist(&self, id: PlaylistId) -> Option<&Playlist> {
        self.playlists.iter().find(|playlist| playlist.id == id)
    }

    pub fn albums(&self) -> &[Album] {
        &self.albums
    }

    pub fn playlists(&self) -> &[Playlist] {
        &self.playlists
    }

    /// Generated SVG cover as a cached image source; always Some for
    /// catalog albums.
    pub fn cover(&self, id: AlbumId) -> Option<ImageSource> {
        self.covers
            .get(&id)
            .map(|image| ImageSource::Image(image.clone()))
    }

    #[cfg(test)]
    pub(crate) fn from_parts(
        tracks: Vec<Track>,
        albums: Vec<Album>,
        playlists: Vec<Playlist>,
    ) -> Self {
        Self {
            tracks,
            albums,
            playlists,
            covers: BTreeMap::new(),
        }
    }
}

fn build_catalog() -> Library {
    let mut tracks = Vec::new();
    let mut albums = Vec::new();
    for (ix, (title, artist, year, track_count)) in ALBUM_FIXTURES.iter().enumerate() {
        let track_ids = (0..*track_count)
            .map(|track_ix| {
                let id = TrackId(tracks.len() as u32);
                tracks.push(Track {
                    id,
                    title: track_title(ix, track_ix),
                    artist: (*artist).into(),
                    duration_ms: 90_000 + (u64::from(id.0) * 37_777) % 210_001,
                });
                id
            })
            .collect();
        albums.push(Album {
            id: AlbumId(ix as u32),
            title: (*title).into(),
            artist: (*artist).into(),
            year: *year,
            track_ids,
        });
    }
    let track_total = tracks.len() as u32;
    let playlists = vec![
        Playlist {
            id: PlaylistId(0),
            title: "Late Night".into(),
            track_ids: (0..12)
                .map(|k| TrackId((k * 4 + 2) % track_total))
                .collect(),
        },
        Playlist {
            id: PlaylistId(1),
            title: "Long Play".into(),
            track_ids: (0..120).map(|k| TrackId(k % track_total)).collect(),
        },
    ];
    let covers = albums
        .iter()
        .map(|album| (album.id, covers::album_image(album.id)))
        .collect();
    Library {
        tracks,
        albums,
        playlists,
        covers,
    }
}

pub(crate) fn format_time(ms: u64) -> SharedString {
    let total_seconds = ms / 1000;
    format!("{}:{:02}", total_seconds / 60, total_seconds % 60).into()
}

pub(crate) fn move_within<T>(items: &mut [T], ix: usize, delta: isize) -> bool {
    if ix >= items.len() {
        return false;
    }
    let target = ix as isize + delta;
    if target < 0 || target >= items.len() as isize {
        return false;
    }
    items.swap(ix, target as usize);
    true
}

#[cfg(test)]
#[path = "model.test.rs"]
mod model_test;
