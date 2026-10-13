//! Removable music-player feature. `MusicPage` and `NowPlayingBar` are the
//! rendered surfaces; `PlayerEngine` is shared state behind the
//! `MusicPlayerGlobal` global. Removal = delete this directory and revert the
//! seam touches: the `pub mod music;`/`pub use` lines in features/pages/mod.rs,
//! the Page::Music arms and all() entry in shell/sidebar.rs, the VALID_HOSTS
//! entry in shell/route.rs, the AppRoot wiring and bar child in
//! shell/root/app_root/{state,render}.rs, the page count in
//! tests/e2e_navigation.rs, the music_* and nav_music keys in both locale
//! files, and the "music_playback" capability line in app/init.rs.

mod bar;
mod covers;
mod engine;
mod model;
mod page;

pub use bar::NowPlayingBar;
pub use engine::{PlayerEngine, TransportStatus};
pub use model::{Album, AlbumId, Library, Playlist, PlaylistId, Track, TrackId};
pub use page::MusicPage;

use gpui_kit::{App, AppContext as _, Context, Entity, Global, Window};

struct MusicPlayerGlobal(Entity<PlayerEngine>);

impl Global for MusicPlayerGlobal {}

/// The app-wide engine handle set by [`init`]; None before `init` ran.
pub fn player(cx: &App) -> Option<Entity<PlayerEngine>> {
    cx.try_global::<MusicPlayerGlobal>()
        .map(|global| global.0.clone())
}

/// Shell wiring: creates the engine, sets the global, registers the
/// capability, and returns the page and bar entities for AppRoot to hold.
pub fn init(
    window: &mut Window,
    cx: &mut Context<crate::root::AppRoot>,
) -> (Entity<MusicPage>, Entity<NowPlayingBar>) {
    let engine = cx.new(|cx| PlayerEngine::new(Library::catalog(), cx));
    cx.set_global(MusicPlayerGlobal(engine.clone()));
    crate::capabilities::set(
        "music_playback",
        crate::capabilities::CapabilityStatus::supported_enabled(),
        cx,
    );
    let page = cx.new(|cx| MusicPage::new(engine.clone(), window, cx));
    let bar = cx.new(|cx| NowPlayingBar::new(engine, window, cx));
    (page, bar)
}
