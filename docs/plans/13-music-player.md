# 13 Music Player Plan

## Goal

A Music page in the sidebar with an albums grid (procedurally generated SVG cover art, no bundled assets), a playlist detail view (virtualized track list with explicit move up/down reorder), and a session queue panel. A persistent now-playing bar sits above the status bar on every page with transport controls, seek, and volume, driven by a simulated playback engine (entity + tick timer, fake progress, auto-advance). Optional real audio output via rodio sits behind a native-only `audio` cargo feature, documented as a separate phase.

## Dependencies

No plan dependencies. `ls docs/plans/` this session shows 01 through 20 (12, 14, 15, 17, 18, 19, 20 landed after the survey was written); none of their Interface Contract items are consumed. Acknowledged peer: `docs/plans/19-audio-playback.md` builds the real decode + cpal output stack in `src/services/audio/` and explicitly treats this plan as a peer, not a dependency (`19-audio-playback.md` Dependencies section). If 19 lands first, Phase 6 of this plan should be dropped in favor of routing the engine's audio through `services::audio` rather than adding rodio; nothing in phases 1–5 depends on that choice. Inputs used:

- `docs/plans/_survey/seams.md` §1 (page seam, 15-row table), §2 (virtual list widget), §4 (events/capabilities), §5 (i18n), §6 (wasm), §7 (tests), §8 (gates), §9 (rules); every load-bearing row re-verified below where cited.
- `docs/plans/_survey/ecosystem.md` §e (images), §f (dnd verdict), §h (rodio 0.23 row); re-verified against crate sources and docs.rs this session.
- `docs/plans/16-waveform-foundation.md` is NOT consumed: its `SampleProducer` needs real sample streams; this plan's simulated engine has none. If both land, a later contract edit could feed plan 16's `WaveformState::attach_source` from plan 19's stack; noted as a follow-up, not a dependency.

## Research Digest

Path key: `GPUI` = `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-pre-0.3.8/src`, `COMP` = `.../gpui-component-0.7.1/src`, `BASE` = `.../gpui-base-0.7.1/src`, `KIT` = `.../gpui-kit-0.7.1/src`, `ASSETS` = `.../gpui-kit-assets-0.7.1`, `REPO` = `/home/thragg/fo/gpui-starter`.

Dnd verdict (re-verified): no dnd/reorder component in gpui-kit 0.7.1 or gpui-component 0.7.1. `find $COMP $BASE -iname "*dnd*"` → nothing; `grep -rln "on_drag|draggable" $COMP` hits only touch_selection, list_item, table state, dock internals. Only raw primitives exist: `div.on_drag<T,W>` (`GPUI/elements/div.rs:615`), `on_drag_move` (`:360`), `on_drop` (`:568`). Per the spec's rule, reorder uses explicit move up/down buttons; the primitives are left unused.

Images (verified chain for runtime-generated SVG):

- `img(source: impl Into<ImageSource>) -> Img` (`GPUI/elements/img.rs:201`); `ImageSource` variants incl. `Image(Arc<Image>)` (`img.rs:42-51`); `impl Styled for Img` (`img.rs:513`) and `StyledImage` with `object_fit` (`img.rs:159`), `ObjectFit::{Fill, Contain, Cover, ScaleDown, None}` (`GPUI/style.rs:29-38`).
- `Image::from_bytes(format: ImageFormat, bytes: Vec<u8>) -> Self` (`GPUI/platform.rs:3217-3223`), `ImageFormat::Svg` (`platform.rs:3091-3104`); id = hash of bytes (`platform.rs:3221`), so identical SVG bytes hit the image cache once.
- SVG decode path: `to_image_data` routes `ImageFormat::Svg` to `svg_renderer.render_single_frame(&self.bytes, 1.0)` (`platform.rs:3296-3298`): the SVG must carry explicit `width`/`height` attributes; gpui's own test rasterizes a 1×1 SVG with explicit attrs (`platform.rs:3405-3420`). Decode errors surface as `ImageCacheError::Usvg` (`img.rs:774-792`).
- In-repo precedent: `REPO/src/gallery/sections/media/demo.rs:16-21` builds `ImageSource::Image(Arc::new(Image::from_bytes(ImageFormat::Svg, include_bytes!(...).to_vec())))`: the exact construction this plan uses, minus `include_bytes!`.

Slider (verified):

- Styled element: `Slider::new(state: &Entity<SliderState>)` with `.horizontal()`, `.disabled(bool)` (`COMP/slider.rs:86-133`); re-exports `SliderEvent, SliderScale, SliderState, SliderValue` from gpui-base (`COMP/slider.rs:9`).
- State: `SliderState::new()` builder `.min/.max/.step/.default_value(impl Into<SliderValue>)`, runtime `set_value(value, &mut Window, &mut Context<Self>)`, reader `value() -> SliderValue` where `Single(f32).start()` returns the value (`BASE/slider.rs:196-299`, `:91-105`).
- Events: `SliderEvent::Change(SliderValue)` continuous during drag, `Release(SliderValue)` once on release; `impl EventEmitter<SliderEvent> for SliderState` (`BASE/slider.rs:15-20`, `:867`); repo subscription pattern `REPO/src/gallery/sections/controls/slider.rs:122-130`.

Buttons (verified): `Button::new(id)` with `.label` (`COMP/button/button.rs:371`), `.icon` (`:397`), `.tooltip` (`:403`), `.on_click(impl Fn(&ClickEvent, &mut Window, &mut App))` (`:447-452`), `.accessibility_label` (`:391`), `.outline()` (`:332`), variant traits primary/danger/ghost (`:52,62,82`), `disabled(bool)` (`:525`); `Sizable::{xsmall, small, with_size}` (`COMP/sizing.rs:190-220`); `Icon::new(...).small()` on icons (`REPO/src/shell/root/app_root/render.rs:330`).

Icon enums (verified on disk; this decides which import each file needs):

- `gpui_kit::component::IconName` is a 106-variant compatibility enum generated by `gpui_kit_assets::__component_icon_names!` (`COMP/icon.rs:12-38`) from `ASSETS/default-icons.txt` (106 lines; the filter is `ASSETS/build.rs:79-107`). It contains `Play` (`default-icons.txt:78`), `Pause` (`:77`), `ArrowLeft` (`:3`), `ArrowUp` (`:5`), `ArrowDown` (`:2`), `CircleX` (`:30`), `ChevronLeft`, `ChevronRight`, `Menu`, `Settings2`, `Close`. It does NOT contain `Music`, `Volume2`, `SkipBack`, `SkipForward`, or `ListMusic` (grep of default-icons.txt for music|volume|skip|list-music returns no hits, run this session).
- `gpui_kit::assets::IconName` is the full 1830-icon catalog enum and DOES contain those variants — but the app's native asset source (`CombinedAssets` wrapping `gpui_kit::assets::Assets`, `REPO/src/app/assets.rs:79-88` + `ASSETS/native_assets.rs:5` — `default_assets.rs` only) embeds just the default 106 bytes; `AllAssets` is the separate full source and the app does not register it. An assets-enum variant therefore compiles but renders a BLANK icon on native (only wasm's CDN source serves the full set). The gallery full-catalog precedents (`tokens.rs:11`, `toolbar.rs:3`) share this hazard; do not copy them.
- `Icon::new(impl Into<Icon>)` (`COMP/icon.rs:105-107`) accepts either enum: `impl<T: IconNamed> From<T> for Icon` (`COMP/icon.rs:59-64`) and assets `IconName: IconNamed` (`ASSETS/src/icon.rs:10-15`).

Consequence: this feature stays ENTIRELY on the component enum — no import swaps, and the sidebar keeps its existing import. Default-subset stand-ins for the audio-only glyphs: `Music` → `Play` (sidebar arm and cover-slot fallback), `SkipBack`/`SkipForward` → `ChevronLeft`/`ChevronRight` (transport), `Volume2` → `Settings2` (mixer row), `ListMusic` → `Menu` (queue panel). Each is a verified default; if richer glyphs are wanted later, the module can ship section-local SVGs the way gallery sections do — an in-module change, no asset-source seam.

Virtual list (verified): repo wrapper `render_virtual_list(cx, id, item_sizes, list_height, gap, &VirtualListScrollHandle, show_scrollbar, render_items)` (`REPO/src/ui/widgets/virtual_list.rs:29-69`), `uniform_item_sizes(count, height)` (`:9-11`), `bounded_list_height(sizes, gap, max)` (`:18-22`), `virtual_list_item(id, label, index, total)` with ListItem role + set metadata (`:74-88`). Usage precedent with capped height + wheel-stop: `REPO/src/features/pages/query_playground/render_sections/activity_log.rs:66-90`.

Timer/entity patterns (verified): tick loop `cx.spawn(async move |this, cx| loop { cx.background_executor().timer(INTERVAL).await; if this.update(cx, |this, cx| ...).is_err() { break; } })` (`REPO/src/gallery/sections/charts/system_monitor.rs:217-230`); `#[gpui_kit::test]` with `cx.executor().advance_clock(Duration)` + `run_until_parked()` (`REPO/src/state/config_store.test.rs:280-295`); `TestAppContext::add_window` (`GPUI/app/test_context.rs:279-296`, cited by plan 16).

Shell seams (verified): AppRoot render ends `v_flex().size_full().child(title_bar).child(content_area).child(crate::status_bar::render(...))` (`REPO/src/shell/root/app_root/render.rs:285-290`): the now-playing bar inserts between `content_area` and the status bar. Page construction in `AppRoot::new` (`REPO/src/shell/root/app_root/state.rs:46-56`), `unchecked_active_page_view` arm (`state.rs:211-223`), `cx.set_global` inside `Context<AppRoot>` works (`state.rs:59-61`). Page seam table: seams §1 rows 1-15 against `REPO/src/shell/sidebar.rs:4-105`, `REPO/src/shell/route.rs:10-21`, `REPO/src/features/pages/mod.rs:1-25`, `REPO/tests/e2e_navigation.rs:14-19` (count assert 10).

A11y helpers (verified): `div().a11y(Role, label)` via `crate::accessibility::A11yExt` (`REPO/src/shell/status_bar.rs:7,85`), `on_a11y_action(AccessibleAction::Click, ...)` on stateful divs (`REPO/src/shell/root/app_root/render.rs:342`; `AccessibleAction = accesskit::Action` re-export at `GPUI/gpui.rs:98`; trait method `GPUI/elements/div.rs:1503`), `a11y_live(accesskit::Live::Polite)` (`REPO/src/features/pages/notifications.rs:53`).

rodio 0.23.0 (docs.rs, fetched 2026-10-12; API reworked: old `OutputStream::try_default` is gone): `DeviceSinkBuilder::open_default_sink() -> Result<MixerDeviceSink, DeviceSinkError>`; `MixerDeviceSink::mixer(&self) -> &Mixer` (docs.rs `stream/struct.MixerDeviceSink.html`); `Player::connect_new(&Mixer) -> Player`; `Player::{append<S: Source + Send + 'static>, set_volume(Float=f32), pause, play, stop, empty}` (docs.rs `struct.Player.html`). Tone source: `rodio::source::SineWave::new(freq)` + `Source::take_duration(Duration)` (crate root docs). Features: default set pulls symphonia decoders (flac/mp3/mp4/vorbis/wav, MPL-2.0 via symphonia); `default-features = false, features = ["playback"]` keeps cpal only and avoids MPL code entirely (docs.rs features page). rodio is not in `REPO/Cargo.lock` (`grep '^name = "rodio"' Cargo.lock` → no hits) and adds a second cpal consumer, acceptable: cpal is a leaf here, no gpui-family conflict (`tests/qa_migration.rs:131-146` only guards gpui-pre/gpui-component/gpui-kit-assets copies).

No new crate for phases 1–5: simulated engine needs only std + gpui-kit.

## Architecture

Entities and ownership (kit coding guides: state in the narrowest owner; shared service via a global handle):

- `PlayerEngine` (entity), the only retained playback state: `Arc<Library>`, `queue: Vec<TrackId>`, `current: Option<usize>` (index into queue), `status: TransportStatus`, `position_ms: u64`, `volume: f32`, tick `Task<()>`, and (Phase 6 only) `Option<AudioBackend>`. Created once per app in `music::init`, registered as the `MusicPlayerGlobal` global. Every mutation ends in one `cx.notify()`; consumers re-render via `cx.observe(&engine, |_, _, cx| cx.notify())`; no custom events (nothing an owner must hear that observation cannot express).
- `MusicPage` (entity), view state only: `Entity<PlayerEngine>`, `detail: Option<DetailSource>`, `orders: HashMap<PlaylistId, Vec<TrackId>>` (session-local playlist order copies; absent = catalog order), `queue_panel_open: bool`, virtual-list scroll handle. Owns no playback state.
- `NowPlayingBar` (entity), shell chrome: `Entity<PlayerEngine>`, `Entity<SliderState>` for seek (normalized 0..1; max never changes, so no runtime max mutation; `SliderState` has none, `BASE/slider.rs:213-243` are build-time only), `Entity<SliderState>` for volume, `scrub: Option<f32>` set on `Change`, cleared on `Release`, and `last_seek_write: f32` as the render-sync loop guard.
- `Library` (plain shared data, `Arc`), immutable catalog: tracks, albums, playlists, prebuilt `Arc<Image>` covers. `Library::catalog()` internally caches via `std::sync::LazyLock`, so `music::init`, the page, and tests all share one instance (stable, no wasm issue; the repo toolchain is rustc 1.98 per survey).

Data flow: UI actions call `PlayerEngine` methods through `Entity::update`; the tick loop advances `position_ms` while `Playing`, auto-advances at track end, and notifies; observers (page, bar) re-render. Playlist reorder mutates the page's order copy only; pressing play snapshots that copy into the engine queue. The engine never reads view state.

Module tree:

```text
src/features/pages/music/
├── mod.rs        seam header + init() + re-exports + MusicPlayerGlobal
├── model.rs      ids, Track/Album/Playlist, Library, format_time, move_within
├── covers.rs     deterministic SVG cover generator (bytes -> Arc<Image>)
├── engine.rs     PlayerEngine entity: transport, queue, tick, volume
├── audio.rs      Phase 6 only (cfg: audio + native): rodio AudioBackend
├── page.rs       MusicPage: library grid, detail view, queue panel
├── bar.rs        NowPlayingBar (transport, seek, volume, meta)
└── *.test.rs     sibling test modules per repo convention
```

## Interface Contract

Exact pub surface, re-exported from `src/features/pages/music/mod.rs` (crate path `gpui_starter::features::pages::music`). Behavioral structs keep private fields; the record types below are data. Everything that may grow is `#[non_exhaustive]`.

```rust
use std::sync::Arc;
use gpui_kit::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TrackId(pub u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AlbumId(pub u32);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PlaylistId(pub u32);

#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct Track { pub id: TrackId, pub title: SharedString, pub artist: SharedString, pub duration_ms: u64 }

#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct Album { pub id: AlbumId, pub title: SharedString, pub artist: SharedString, pub year: u16, pub track_ids: Vec<TrackId> }

#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct Playlist { pub id: PlaylistId, pub title: SharedString, pub track_ids: Vec<TrackId> }

/// Immutable demo catalog. `catalog()` returns the same instance on every call.
pub struct Library { /* private */ }
impl Library {
    pub fn catalog() -> Arc<Library>;
    pub fn track(&self, id: TrackId) -> Option<&Track>;
    pub fn album(&self, id: AlbumId) -> Option<&Album>;
    /// Album owning this track, for cover resolution; always Some for
    /// catalog tracks.
    pub fn track_album(&self, id: TrackId) -> Option<AlbumId>;
    pub fn playlist(&self, id: PlaylistId) -> Option<&Playlist>;
    pub fn albums(&self) -> &[Album];
    pub fn playlists(&self) -> &[Playlist];
    /// Generated SVG cover as a cached image source; one per album, always Some
    /// for catalog albums.
    pub fn cover(&self, id: AlbumId) -> Option<ImageSource>;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TransportStatus { #[default] Stopped, Playing, Paused }

/// Simulated playback engine. Simulated clock is the source of truth for
/// position even with the `audio` feature on. Every state-changing mutator
/// ends with exactly one `cx.notify()`; the documented no-ops return without
/// notifying. Consumers observe the entity.
pub struct PlayerEngine { /* private */ }
impl PlayerEngine {
    pub fn new(library: Arc<Library>, cx: &mut Context<Self>) -> Self;

    /// Replaces the queue and starts playback at `start_ix`. No-op when
    /// `start_ix >= tracks.len()` or the queue would be empty.
    pub fn play_queue(&mut self, tracks: Vec<TrackId>, start_ix: usize, cx: &mut Context<Self>);
    /// Jumps to an existing queue index and plays.
    pub fn play_queue_index(&mut self, ix: usize, cx: &mut Context<Self>);
    /// Playing -> Paused; Paused -> Playing; Stopped -> Playing when a
    /// current track exists, otherwise no-op.
    pub fn toggle_playback(&mut self, cx: &mut Context<Self>);
    /// Advances; at the end of the queue the engine stops (position 0,
    /// `Stopped`, current stays on the last index). `next`/`previous` keep a
    /// Paused status Paused (track changes, playback does not resume).
    pub fn next(&mut self, cx: &mut Context<Self>);
    /// Rewinds; at index 0 it only resets position to 0.
    pub fn previous(&mut self, cx: &mut Context<Self>);
    /// Clamped to the current track's duration; no-op with no current track.
    pub fn seek_ms(&mut self, ms: u64, cx: &mut Context<Self>);
    /// Clamped to 0.0..=1.0.
    pub fn set_volume(&mut self, volume: f32, cx: &mut Context<Self>);
    /// Appends; if nothing was ever selected the new track becomes current
    /// (status stays Stopped until play).
    pub fn enqueue(&mut self, track: TrackId, cx: &mut Context<Self>);
    pub fn clear_queue(&mut self, cx: &mut Context<Self>);

    pub fn status(&self) -> TransportStatus;
    pub fn current_track(&self) -> Option<Track>;
    pub fn position_ms(&self) -> u64;      // 0 with no current track
    pub fn duration_ms(&self) -> u64;      // 0 with no current track
    /// position / duration clamped to 0..=1; 0 when duration is 0.
    pub fn progress(&self) -> f32;
    pub fn volume(&self) -> f32;
    pub fn queue(&self) -> &[TrackId];
    pub fn current_ix(&self) -> Option<usize>;
}

pub struct MusicPage { /* private */ }
impl MusicPage {
    /// `player` is observed for re-render.
    pub fn new(player: Entity<PlayerEngine>, window: &mut Window, cx: &mut Context<Self>) -> Self;
}

pub struct NowPlayingBar { /* private */ }
impl NowPlayingBar {
    /// `player` is observed for re-render. Mount once at the shell level.
    pub fn new(player: Entity<PlayerEngine>, window: &mut Window, cx: &mut Context<Self>) -> Self;
}

/// The app-wide engine handle set by `init`; None before `init` ran.
pub fn player(cx: &App) -> Option<Entity<PlayerEngine>>;

/// Shell wiring: creates the engine, sets the global, registers capabilities,
/// returns (page, bar) for AppRoot to hold. Call once from `AppRoot::new`.
pub fn init(window: &mut Window, cx: &mut Context<crate::root::AppRoot>)
    -> (Entity<MusicPage>, Entity<NowPlayingBar>);
```

Crate-internal (not contract): `model::format_time(ms: u64) -> SharedString`, `model::move_within<T>(items: &mut [T], ix: usize, delta: isize) -> bool`, `covers::album_svg(id: AlbumId) -> String`, `audio::AudioBackend` (Phase 6). The `audio` cargo feature changes no pub signature; it only makes the engine also drive rodio on native.

Consumption notes that are part of the contract:

- Re-render recipe: keep `Entity<PlayerEngine>`, wire `cx.observe(&player, |_, _, cx| cx.notify())` once in the consumer's constructor (pattern: `REPO/src/features/pages/notifications.rs:13-22`).
- The tick task lives inside the engine and dies with the entity; consumers never manage it. Tests drive time with `cx.executor().advance_clock` (`config_store.test.rs:288-292`).
- Queue identity is positional (`current_ix`); `TrackId` values may repeat within a queue (the long playlist repeats tracks). Do not key per-row UI state on `TrackId` alone; use `(source id, row position)`.
- Dependent plans needing real samples (e.g. plan 16) extend this contract later; nothing here blocks them.

## Module Layout

| File | Purpose |
|---|---|
| `src/features/pages/music/mod.rs` | Seam: removal-recipe header, `mod` decls, pub re-exports, `MusicPlayerGlobal`, `init()` |
| `src/features/pages/music/model.rs` | Id types, `Track`/`Album`/`Playlist`, `Library` + fixture catalog, `format_time`, `move_within` |
| `src/features/pages/music/covers.rs` | Deterministic SVG cover bytes per album, `Arc<Image>` construction |
| `src/features/pages/music/engine.rs` | `PlayerEngine`, `TransportStatus`, tick loop, queue/transport/volume/seek logic |
| `src/features/pages/music/audio.rs` | Phase 6: `AudioBackend` over rodio 0.23 (`cfg(all(feature = "audio", not(target_family = "wasm")))`) |
| `src/features/pages/music/page.rs` | `MusicPage`: albums grid, playlists list, detail view (virtual list + move up/down), queue panel |
| `src/features/pages/music/bar.rs` | `NowPlayingBar`: transport buttons, seek slider, volume slider, track meta |
| `src/features/pages/music/model.test.rs`, `engine.test.rs`, `covers.test.rs`, `page.test.rs`, `bar.test.rs` | Sibling tests (`#[cfg(test)] #[path = ...]` per seams §7) |

## Seam Touches

Every edit outside `src/features/pages/music/` (page seam rows from seams §1, all re-verified this session):

| # | File | Change |
|---|---|---|
| 1 | `src/features/pages/mod.rs:10` | `pub mod music;` (public module so `crate::views::music::init` resolves; `ai_chat` precedent at `:2-3`; Step 4.4 adds `pub use music::MusicPage;` next to `pub use notifications::NotificationsPage;` at `:19`) |
| 2 | `src/shell/sidebar.rs:5-16` | `Music` variant (after `Gallery`; appended at the end so existing digit bindings `secondary-1..9` do not shift; seam gotcha seams §1 row 8, binding code `state.rs:144-166`) |
| 3 | `src/shell/sidebar.rs:19-31` | `title()` arm `"Music"` |
| 4 | `src/shell/sidebar.rs:36-48` | `title_key()` arm `"nav_music"` |
| 5 | `src/shell/sidebar.rs:57-69` | `host()` arm `"music"` |
| 6 | `src/shell/sidebar.rs:77-89` | `icon()` arm `IconName::Play` — component enum, NO import or return-type swap (the full-catalog `Music` renders blank on native: the app embeds only the default 106 — Research Digest icons bullet) |
| 7 | `src/shell/sidebar.rs:92-105` | `all()` entry appended after `Page::Gallery` |
| 8 | `src/shell/route.rs:10-21` | `Page::Music.host(),` in `VALID_HOSTS` |
| 9 | `src/shell/root/app_root/state.rs:17-38` | Fields `music_page: Entity<MusicPage>`, `now_playing_bar: Entity<NowPlayingBar>` |
| 10 | `src/shell/root/app_root/state.rs:46-56` | `let (music_page, now_playing_bar) = crate::views::music::init(window, cx);` + import |
| 11 | `src/shell/root/app_root/state.rs:211-223` | `unchecked_active_page_view` arm `Page::Music => self.music_page.clone().into()` |
| 12 | `src/shell/root/app_root/render.rs:285-290` | `.child(self.now_playing_bar.clone())` between `content_area` and `crate::status_bar::render(...)` (entity-as-child precedent: `title_bar` at `render.rs:287`) |
| 13 | `tests/e2e_navigation.rs:14-19` | Count 10 → 11; add `Page::Music` to the `expected` array (`:25`) |
| 14 | `locales/en.yml` + `locales/zh-CN.yml` | All `music_*` + `nav_music` keys (table below), in both files in lockstep (`tests/qa_localization.rs:188-198`) |
| 15 | `src/app/init.rs:95-108` | Add `"music_playback"` to `DEFAULT_ENABLED_CAPABILITIES` |
| 16 | Phase 6 only: `Cargo.toml` `[features]` `audio = ["dep:rodio"]` + not-wasm table `rodio = { version = "0.23", default-features = false, features = ["playback"], optional = true }`; regenerate + commit `Cargo.lock` (`tests/qa_migration.rs:167-186`) |

Deliberate non-touches: no `AppConfig` field (nothing persists; volume and queue are session state; schema stays pinned per `tests/snapshot_tests.rs:26,57`); no commands (launcher entry is future work); no `src/ui/` widget (the page reuses `ui::widgets::render_virtual_list`); no sub-route (`detail` is page-internal state, so deep links keep working unchanged); `src/lib.rs` untouched (`pub use features::pages as views` covers it, seams §1 row 15).

### Removal Recipe

`mod.rs` opens with (mirroring `src/gallery/mod.rs:1-7`):

```rust
//! Removable music-player feature. `MusicPage` and `NowPlayingBar` are the
//! rendered surfaces; `PlayerEngine` is shared state behind the
//! `MusicPlayerGlobal` global. Removal = delete this directory and revert the
//! seam touches: the `mod music;`/`pub use` lines in features/pages/mod.rs,
//! the Page::Music arms and all() entry in shell/sidebar.rs, the VALID_HOSTS
//! entry in shell/route.rs, the AppRoot wiring and bar child in
//! shell/root/app_root/{state,render}.rs, the page count in
//! tests/e2e_navigation.rs, the music_* and nav_music keys in both locale
//! files, the "music_playback" capability line in app/init.rs, and (if Phase
//! 6 landed) the audio feature and rodio entry in Cargo.toml plus the
//! regenerated Cargo.lock.
```

Ordered removal steps:

1. Revert seam rows 13, 15, 14, 12, 11, 10, 9, 8, 7-2, 1 (reverse order of the table above), each to its current content.
2. `rm -r src/features/pages/music/`.
3. If Phase 6 landed: remove the `audio` feature and the rodio line from `Cargo.toml`, run `cargo check --workspace`, commit the regenerated `Cargo.lock`.
4. `grep -rn "music" src/ tests/ locales/`: remaining hits are dependents consuming the Interface Contract; removal is only complete when they are removed or stubbed first.
5. Run the gates in Verification; they must pass with zero references left.

## Implementation Phases

## Phase 1: Catalog model and covers

### Step 1.1: Module scaffold and id types

- Files: `src/features/pages/music/mod.rs`, `src/features/pages/music/model.rs`, `src/features/pages/mod.rs`.
- Build: `model.rs` with the three id newtypes (`Serialize, Deserialize` derives so dependents can persist references); `music/mod.rs` with the removal-recipe header, `mod model;` plus `pub use model::{TrackId, AlbumId, PlaylistId};` (final re-export list frozen in Step 4.4); `pages/mod.rs` gains `pub mod music;` (ai-chat precedent `mod.rs:2-3`).
- Tests: none.
- Done-check: `cargo check -p gpui-starter` passes; `grep -n "mod music" src/features/pages/mod.rs` shows the new line.

### Step 1.2: Catalog fixture

- Files: `model.rs`, `model.test.rs` (wired `#[cfg(test)] #[path = "model.test.rs"] mod model_test;`).
- Build: `Track`/`Album`/`Playlist` records (`#[non_exhaustive]`, pub fields); `Library { tracks: Vec<Track>, albums: Vec<Album>, playlists: Vec<Playlist>, covers: BTreeMap<AlbumId, Arc<Image>> }` with private fields and the contract's reader methods (`track_album` included; the bar needs it for cover resolution); `Library::catalog()` via `std::sync::LazyLock` building the fixture: 8 albums × 6–7 tracks (~50 tracks, invented titles/artists, e.g. artist "Vector Field", album "Signal Drift", durations 90_000–300_000 ms), 2 playlists: "Late Night" (12 entries) and "Long Play" (120 entries drawing tracks cyclically, duplicates intended so the virtual list has real volume). `format_time(ms) -> SharedString` rendering `M:SS` (minutes unpadded, seconds zero-padded; 3_723_000 → "62:03"). `move_within<T>(items: &mut [T], ix: usize, delta: isize) -> bool`: swaps toward `delta` direction, false (no-op) when the target is out of bounds.
- Tests: ids unique across each collection; every `album.track_ids`/`playlist.track_ids` entry resolves via `track()`; `format_time` spot checks (0 → "0:00", 59_999 → "0:59", 60_000 → "1:00", 3_723_000 → "62:03"); `move_within` moves inside bounds and refuses `[0, -1]` and `[len-1, +1]`.
- Done-check: `cargo test -p gpui-starter --lib pages::music::model` passes.

### Step 1.3: Cover generator

- Files: `covers.rs`, `covers.test.rs`, extend `mod.rs` decls.
- Build: `covers::album_svg(id: AlbumId) -> String`: deterministic, no RNG: hue `h = (id.0 as i32 * 137) % 360`, background `hsl(h, 45%, 22%)`, foreground `hsl((h + 40) % 360, 70%, 62%)`; pattern variant `id.0 % 4` (0 diagonal split + circle, 1 three concentric rings, 2 seven vertical stripes, 3 four quarter-circles); root tag `<svg xmlns="http://www.w3.org/2000/svg" width="512" height="512" viewBox="0 0 512 512">` (explicit size is required by the rasterizer, `GPUI/platform.rs:3296-3298`). No text elements (avoids i18n and font dependence). `covers::album_image(id) -> Arc<Image>` = `Arc::new(Image::from_bytes(ImageFormat::Svg, album_svg(id).into_bytes()))` (`GPUI/platform.rs:3217`); bytes contain only the fixed template plus numbers, so no XML escaping is needed, asserted by the test. `Library::catalog()` prebuilds `covers` for every album; `Library::cover(id)` returns `Some(ImageSource::Image(arc.clone()))`.
- Tests: output starts with the svg root tag and contains `width="512"` and `height="512"`; same id → identical string; the 8 catalog ids produce 8 distinct strings; every char is ASCII (no injected markup).
- Done-check: `cargo test -p gpui-starter --lib pages::music::covers` passes.

## Phase 2: PlayerEngine

### Step 2.1: Transport core and tick loop

- Files: `engine.rs`, `engine.test.rs`.
- Build: `TransportStatus` (derive `Default = Stopped`, `Clone, Copy, Debug, PartialEq, Eq`); `pub const TICK_MS: u64 = 250;` (pub(crate)); `PlayerEngine { library: Arc<Library>, queue: Vec<TrackId>, current: Option<usize>, status: TransportStatus, position_ms: u64, volume: f32, tick: Option<Task<()>> }` (audio field added in Phase 6). `new()` sets `volume = 0.8`, spawns the tick task stored (not detached; dropping the engine cancels it; plan 16 Step 4.2 precedent): `cx.spawn(async move |this, cx| loop { cx.background_executor().timer(Duration::from_millis(TICK_MS)).await; if this.update(cx, |engine, cx| engine.tick(cx)).is_err() { break; } })` (`system_monitor.rs:217-230` shape). `tick(&mut self, cx)`: return without notify when `status != Playing`; `position_ms += TICK_MS`; when `position_ms >= duration_ms()` run the end-of-track transition (advance index or stop, exactly as `next` does at the boundary) and set `position_ms = 0` (advance) or `0` (stop); one `cx.notify()` per call. Readers per contract (`duration_ms` = `current_track().map_or(0, |t| t.duration_ms)`, `progress` clamps and guards division).
- Tests (`#[gpui_kit::test]`, engine built with a small inline `Library`: add `pub(crate) fn from_parts` test constructor or build via `Library` fields through a `pub(crate) builder`; choose `pub(crate) fn from_parts(tracks, albums, playlists)` reusing `Album/Playlist` records, covers empty):
  - Fresh engine: `status() == Stopped`, `current_track().is_none()`, `progress() == 0.`, `volume() == 0.8`.
  - `play_queue([t0, t1, t2], 1)`: current is t1, status Playing, position 0.
  - Tick advance: track duration 1000 ms → `advance_clock(750ms)` + `run_until_parked()` → `position_ms() == 750`.
  - Auto-advance: duration 500 ms, `advance_clock(750ms)` → next track at position 250 (500 ticked over into track 2).
  - End of queue stop: single 500 ms track, `advance_clock(1000ms)` → `Stopped`, `position_ms == 0`, `current_ix` still `Some(0)`.
  - Paused engine consumes ticks without moving: pause, `advance_clock(1000ms)`, position unchanged.
- Done-check: `cargo test -p gpui-starter --lib pages::music::engine` passes.

### Step 2.2: Queue operations, seek, volume

- Files: same.
- Build: `play_queue`, `play_queue_index`, `toggle_playback`, `next`, `previous`, `seek_ms` (clamp `0..=duration_ms`), `set_volume` (clamp), `enqueue` (append; when `current.is_none()` set `current = Some(new_last)`), `clear_queue` (clear, `current = None`, position 0, Stopped), `queue`, `current_ix`, `current_track`: semantics exactly as the Interface Contract states. Each mutator notifies once; no-ops (empty queue toggle, out-of-bounds jump) do not notify.
- Tests: `next` at last index → Stopped + position 0 + current unchanged; `previous` at 0 → position 0 only; `next`/`previous` while Paused stay Paused with the new track; `toggle_playback` cycles Stopped(no current) → no-op, Playing → Paused → Playing; `seek_ms(10 min)` clamps to duration; `set_volume(2.0) == 1.0`, `set_volume(-1.) == 0.`; `enqueue` on an empty engine selects the track but stays Stopped, then `toggle_playback` plays it; `clear_queue` resets everything; duplicate `TrackId`s in a queue play in order (`play_queue([t0, t0, t0], 2)` → current_ix 2).
- Done-check: `cargo test -p gpui-starter --lib pages::music::engine` passes.

### Step 2.3: Global handle

- Files: `mod.rs`, `engine.test.rs`.
- Build: `struct MusicPlayerGlobal(Entity<PlayerEngine>); impl Global for MusicPlayerGlobal;` (private); `pub fn player(cx: &App) -> Option<Entity<PlayerEngine>>` reading `cx.try_global`. `init()` is not added yet: it needs both view types and lands in Step 4.4.
- Tests: `player(cx)` is None before any global, Some after `cx.update(|cx| cx.set_global(MusicPlayerGlobal(entity)))` and the entity reads back equal (`entity == returned`, `Entity` PartialEq via `entity_id`, same handle).
- Done-check: `cargo test -p gpui-starter --lib pages::music` passes.

## Phase 3: Now-playing bar

### Step 3.1: Bar entity and layout

- Files: `bar.rs`, `bar.test.rs`, `mod.rs` (decl + re-export).
- Build: `bar.rs` uses the component enum (`gpui_kit::component::IconName` via the usual glob; stand-ins per Research Digest — no `Music`/`SkipBack`/`SkipForward`/`Volume2` exist in the default 106 the app registers). `NowPlayingBar { player: Entity<PlayerEngine>, seek: Entity<SliderState>, volume: Entity<SliderState>, scrub: Option<f32>, last_seek_write: f32, _subscriptions: Vec<Subscription> }`. `new`: `SliderState::new().min(0.).max(1.).step(0.001).default_value(0.)` (seek; normalized so the max never changes, `SliderState` max is build-time only, `BASE/slider.rs:230-243`), `SliderState::new().min(0.).max(1.).step(0.01).default_value(0.8)` (volume); subscriptions: `cx.observe(&player, |_, _, cx| cx.notify())`, `cx.subscribe(&seek, |this, _, event: &SliderEvent, cx| match event { Change(v) => this.scrub = Some(v.start()), Release(v) => { let d = this.player.read(cx).duration_ms() as f32; this.scrub = None; this.player.update(cx, |p, cx| p.seek_ms((v.start() * d) as u64, cx)); } })` (`slider.rs:122-130` pattern; `SliderValue::Single.start()` returns the value, `BASE/slider.rs:91-96`), volume subscription applying `set_volume(v.start())` on Change (immediate). Render: one compact row `div().id("now-playing-bar").a11y(Role::Region, localize("music_bar_region")).w_full().h_10().flex().items_center().gap_3().px_3().border_t_1().border_color(cx.theme().status_bar_border).bg(cx.theme().tokens.status_bar)`: same surface tokens as the status bar (`REPO/src/shell/status_bar.rs:92-93`). Children: cover slot (`img(Library cover).size_8().rounded_lg().object_fit(ObjectFit::Cover)` when the current track's album resolves, else `div().size_8().rounded_lg().bg(cx.theme().muted)` with `Icon::new(IconName::Play).xsmall()`; album resolution uses the `Library::track_album(track_id) -> Option<AlbumId>` reader added in Step 1.2); meta column (`v_flex().min_w_0().w(rems(24.))` with `.truncate()` title `text_sm` and muted `text_xs` artist, or `music_nothing_playing` muted when idle); transport `Button::new("npb-prev").ghost().xsmall().icon(IconName::ChevronLeft).accessibility_label(localize("music_previous")).on_click(...)` plus play/pause (icon swaps `Play`/`Pause` by status) and next `IconName::ChevronRight` (`button.rs:391,397,447`); time labels `format_time(display_ms)` + " / " + `format_time(duration)` in `.flex_shrink_0().text_xs().text_color(cx.theme().muted_foreground)` where `display_ms` prefers `scrub` (`scrub * duration`); seek `Slider::new(&self.seek).horizontal().flex_1().min_w_0()`; volume group `Icon::new(IconName::Settings2).xsmall()` (no speaker glyph in the default 106; mixer stand-in per Research Digest) + `Slider::new(&self.volume).w(rems(8.))`.

Seek-slider sync (locked): the observe callback has no `&mut Window`, and `set_value` requires one (`BASE/slider.rs:273-284`), so the slider value is synced at the top of `render`: `if self.scrub.is_none() && engine.progress() != self.last_seek_write { self.seek.update(cx, |s, cx| s.set_value(p, window, cx)); self.last_seek_write = p; }`. This cannot loop: `set_value` notifies `Context<SliderState>` only, never the bar, and the `last_seek_write` guard makes the write conditional. One comment in the code states exactly that. The bar struct therefore carries `last_seek_write: f32`.
- Locale keys (both files): `music_bar_region`, `music_nothing_playing`, `music_play`, `music_pause`, `music_next`, `music_previous`, `music_volume`.
- Tests (`#[gpui_kit::test]`, `cx.add_window` per plan 16 Step 6.2, `GPUI/app/test_context.rs:279-296`): construct bar with an engine in each of Stopped/Playing/Paused, open a window rendering a host view containing the bar, assert no panic and `engine.status()` unchanged after first draw.
- Done-check: `cargo test -p gpui-starter --lib pages::music::bar` passes.

### Step 3.2: Bar interaction logic

- Files: same.
- Build: none beyond 3.1 (handlers already wired); this step adds the interaction tests.
- Tests: through a window-hosted bar, call the slider subscriptions directly: `seek_entity.update(cx, |s, cx| { s.set_value(0.5, window, cx); cx.emit? })` is synthetic; instead test at the seam the contract guarantees: simulate `Release` by invoking the bar's pub(crate) handler `fn on_seek_release(&mut self, value: f32, cx: &mut Context<Self>)` extracted as a named method (used by the subscription and by tests), likewise `on_volume_change`. Assert `on_seek_release(0.5)` on a 200 s track → `player.position_ms() == 100_000`; `on_volume_change(0.25)` → `player.volume() == 0.25`; transport buttons call engine methods: assert via engine state after `bar.update(cx, |b, cx| b.on_next(cx))` (pub(crate) handler mirroring the click closures). Named-handler extraction keeps tests off closure internals.
- Done-check: `cargo test -p gpui-starter --lib pages::music::bar` passes.

## Phase 4: Music page and shell wiring

### Step 4.1: Library view

- Files: `page.rs`, `page.test.rs`, `mod.rs`.
- Build: `page.rs` uses the component enum throughout (default-subset stand-ins per Research Digest — Play/Menu/ArrowLeft/ArrowUp/ArrowDown are all verified defaults; no assets-enum import, whose variants render blank on native). `enum DetailSource { Album(AlbumId), Playlist(PlaylistId) }` (pub(crate)); `MusicPage { player: Entity<PlayerEngine>, library: Arc<Library>, detail: Option<DetailSource>, orders: HashMap<PlaylistId, Vec<TrackId>>, queue_panel_open: bool, detail_scroll: VirtualListScrollHandle, _subscriptions: Vec<Subscription> }`; `new` observes the player. `render`: root `h_flex().items_stretch().size_full().min_h_full().p_6().gap_4()` with the content column `flex_1().min_w_0()` and (when open) the queue panel (`h_flex` centers its cross axis, so full-height columns need `items_stretch`, per the coding guides layout section). Content column in library mode: section heading Playlists (`.a11y(Role::Heading, ...).aria_level(2).text_lg().font_weight(FontWeight::BOLD)`: level 2 because the app header already owns the page's level 1, `render.rs:149-158`); playlist rows are `div().id(("playlist", id)).a11y(Role::Button, format!("{title}. {count}")).rounded_lg().px_3().py_2().hover(|s| s.bg(cx.theme().muted)).on_click(...).on_a11y_action(AccessibleAction::Click, ...)` (`Role::Button` precedent `REPO/src/features/pages/query_devtools_v2/registry.rs:385`; hover on `muted`, a verified token). Then heading Albums, grid `div().flex().flex_wrap().gap_4()`: card `v_flex().w(rems(10.)).gap_2().a11y(Role::Button, format!("{title}. {artist}")).on_click(...).on_a11y_action(AccessibleAction::Click, ...)` with `img(cover).w_full().h(rems(10.)).rounded_lg().object_fit(ObjectFit::Cover)`, title `.truncate().text_sm().font_weight(FontWeight::MEDIUM)`, artist muted `text_xs`; activation opens detail via `pub(crate) fn open_detail(&mut self, source, cx)`.
- Locale keys added here (both files): `music_library_playlists`, `music_library_albums`, `music_album_count` (uses `%{count}` + `localize_with_args`, `REPO/src/services/i18n.rs:7-13`).
- Tests: `add_window` smoke: page renders with `detail = None`; `page.update(open_detail(Album(first)))` then next frame renders without panic (drive by re-`add_window` after mutation, plan 16 Step 6.2 idiom).
- Done-check: `cargo test -p gpui-starter --lib pages::music::page` passes.

### Step 4.2: Detail view with virtual list and reorder

- Files: same.
- Build: detail mode content: back button `Button::new("music-back").ghost().xsmall().icon(IconName::ArrowLeft).accessibility_label(localize("music_back"))` (`arrow-left.svg` verified in both the assets catalog and `default-icons.txt:3`); header row with source title `text_xl` + meta line (`music_album_count` via `localize_with_args`, count = track count) + `Button::new("music-play-all").outline().xsmall().icon(IconName::Play).label(localize("music_play_all"))` calling `play_queue(self.detail_order(), 0)`. Track list: rows `px(40.)` via `uniform_item_sizes(n, px(40.))`, height `bounded_list_height(&sizes, px(0.), px(480.))` (the max is `Pixels` — no `From<Rems> for Pixels` exists in the crate, `geometry.rs` grep-verified; every repo call site passes `px(...)`, `virtual_list.test.rs:48-80`; `px(480.)` = 30 rem at the default scale) inside `div().on_scroll_wheel(|_, _, cx| cx.stop_propagation())` (`activity_log.rs:69-90` idiom), `render_virtual_list(cx, ("music-tracks", source_key), sizes, list_h, px(0.), &self.detail_scroll, true, render_items)`. Row: `virtual_list_item(("music-track", ix), label, ix, total)` with label `"{ix+1}. {title}. {artist} ({duration})"`; inner `h_flex().h_full().items_center().gap_2()`: index+title block `flex_1().min_w_0().truncate()` (click row → `play_queue(order, ix)`), duration `text_xs` muted `flex_shrink_0`, and for playlists only a trailing lane `h_flex().flex_shrink_0().gap_1()` with `Button::new(("music-up", ix)).ghost().xsmall().icon(IconName::ArrowUp).accessibility_label(localize("music_move_up")).disabled(ix == 0).on_click(move up)` and the mirrored down button: `move_within(self.orders.entry(playlist_id).or_insert_with(|| catalog order clone), ix, -1)` then `cx.notify()`. Order resolution: `fn detail_order(&self) -> Vec<TrackId>`: Album → `album.track_ids.clone()`, Playlist → the `orders` copy if present else catalog order. Row ids are `(name, ix)` positional on purpose: order is the mutable data, so identity is positional (contract note).
- Locale keys: `music_back`, `music_play_all`, `music_move_up`, `music_move_down`.
- Tests: `detail_order` returns catalog order before any move and the moved order after `move_within` through the page's handler (`page.update(|p, cx| p.move_track(ix, -1, cx))` pub(crate)); after a move, first two entries swapped; `move_track(0, -1)` is a no-op; album detail ignores move handlers (button not rendered; assert via `orders` staying empty for albums).
- Done-check: `cargo test -p gpui-starter --lib pages::music::page` passes.

### Step 4.3: Queue panel

- Files: same.
- Build: panel `v_flex().w(rems(20.)).flex_shrink_0().border_l_1().border_color(cx.theme().border).p_3().gap_2()`: heading row `music_queue` (level 2) + count + `Button::new("music-queue-clear").ghost().xsmall().label(localize("music_queue_clear")).disabled(queue.is_empty()).on_click(clear_queue)`; body `div().id("music-queue-list").a11y(Role::List, localize("music_queue")).aria_size_of_set(len).flex_1().min_h_0().overflow_y_scroll().v_flex().gap_1()`; empty state `music_queue_empty` muted. Item: `div().id(("music-queue-item", ix)).a11y(Role::ListItem, "{title}. {artist}").aria_position_in_set(ix).aria_size_of_set(len).rounded_lg().px_2().py_1()`, current index highlighted `.bg(cx.theme().accent)` (`pub accent: Hsla` at `BASE/theme_tokens.rs:31`; repo usage `REPO/src/gallery/sections/layout/collapsible.rs:94`), click → `play_queue_index(ix)`. Page header gains the toggle: `Button::new("music-queue-toggle").ghost().xsmall().icon(IconName::Menu).accessibility_label(localize("music_queue")).on_click(toggle queue_panel_open)` placed in the library heading row; the panel's presence is its visible state (documented in Design Compliance).
- Locale keys: `music_queue`, `music_queue_empty`, `music_queue_clear`. (Bar keys landed in Step 3.1, library/detail keys in 4.1/4.2.)
- Tests: with an engine holding a 3-track queue, panel renders all rows without panic (window smoke); clicking semantics tested at engine level (`play_queue_index` covered in 2.2); `queue_panel_open` toggles via the pub(crate) handler.
- Done-check: `cargo test -p gpui-starter --lib pages::music::page` passes.

### Step 4.4: Shell seam and feature freeze

- Files: `mod.rs`, `src/features/pages/mod.rs`, `src/shell/sidebar.rs`, `src/shell/route.rs`, `src/shell/root/app_root/state.rs`, `src/shell/root/app_root/render.rs`, `src/app/init.rs`, `tests/e2e_navigation.rs`, `locales/en.yml`, `locales/zh-CN.yml`.
- Build: every remaining seam row from the table (1 `pub use music::MusicPage;` next to the existing re-exports, 2-8 sidebar + route, 9-11 AppRoot state, 12 bar child in render, 13 e2e count, 15 capability). `music::init(window, cx)` final form: create engine (`Library::catalog()`), `cx.set_global(MusicPlayerGlobal(engine.clone()))`, `crate::capabilities::set("music_playback", CapabilityStatus::supported_enabled(), cx)` (belt-and-braces with row 15), construct page + bar, return them. Final `mod.rs` re-export list exactly `MusicPage, NowPlayingBar, PlayerEngine, TransportStatus, Library, Track, Album, Playlist, TrackId, AlbumId, PlaylistId, player, init`. Doc comments on public items per the contract's ownership notes.
- Tests: `tests/e2e_navigation.rs` updated count passes; `cargo test --workspace --all-features` green (localization lockstep included).
- Done-check: `cargo test -p gpui-starter --lib pages::music && cargo test --test e2e_navigation` pass; app boots to the Music page (manual).

## Phase 5: Platform, wasm, and gates

### Step 5.1: Wasm pass and final gates

- Files: none new; verification only.
- Build: nothing; confirm no `cfg` exists yet in the music module (simulated engine is platform-neutral: entities, timers, SVG rasterization all work on wasm; tick timer precedent `system_monitor.rs:217-230` compiles into the wasm lib per seams §6).
- Tests: full gate set.
- Done-check: `cargo fmt --all -- --check && cargo clippy --workspace --all-features && cargo test --workspace --all-features && just wasm-check` all pass, in that order.

## Phase 6 (optional, native-only): real audio via rodio

Tradeoffs, stated up front: (a) rodio 0.23 is a recent API rework with no in-repo usage, docs.rs-verified only, so this phase is gated behind a cargo feature and can be dropped without touching phases 1–5; (b) `default-features = false, features = ["playback"]` avoids symphonia entirely (symphonia is MPL-2.0, flagged in the ecosystem survey); consequence: no file decoding; playback is synthesized tones derived from the track id, which matches the no-bundled-assets constraint; adding mp3/flac decode later means accepting MPL-2.0 files in the tree; (c) the simulated clock stays authoritative, so audio and UI progress can drift by up to a tick interval, acceptable for a demo transport, documented in the module header; (d) no wasm: the feature compiles to a no-op on wasm32-unknown-unknown (same convention as `notifications-portal`, feature at `Cargo.toml:144`, no-op comment at `:250-256`, not-wasm table at `:310`).

### Step 6.1: Dependency and backend

- Files: `Cargo.toml`, `src/features/pages/music/audio.rs`, `engine.rs`, `engine.test.rs`.
- Build: `[features] audio = ["dep:rodio"]`; not-wasm table gains `rodio = { version = "0.23", default-features = false, features = ["playback"], optional = true }` (seams §6 table rules). Run `cargo check --workspace`, commit the regenerated `Cargo.lock` with the manifest change (`tests/qa_migration.rs:167-186`). `audio.rs` gated `#![cfg(all(feature = "audio", not(target_family = "wasm")))]`-style module attr in `mod.rs`: `#[cfg(all(feature = "audio", not(target_family = "wasm")))] mod audio;`. `AudioBackend { sink: MixerDeviceSink, player: Player }`: `open() -> Result<Self, String>` = `DeviceSinkBuilder::open_default_sink()` then `Player::connect_new(sink.mixer())` (drop order: player first, sink last; dropping the sink disposes the OS stream, docs.rs `MixerDeviceSink` Drop note); `start_tone(track_id, remaining: Duration)` = `player.stop(); append(SineWave::new(freq).take_duration(remaining))` with `freq = 110.0 * 2f32.powf((track_id.0 % 24) as f32 / 12.0)`; `set_volume(v)`, `pause()`, `play()`, `stop()`. Engine integration behind the same cfg: field `audio: Option<AudioBackend>`; first `play_*`/`toggle` to Playing calls `AudioBackend::open()`: on error, `crate::capabilities::set("music_audio_output", CapabilityStatus::degraded(err), cx)` and continue silent; on success `set("music_audio_output", supported_enabled())` once. Forward volume/pause/play/track-change/seek (seek = `start_tone(current, duration - position)`); engine clock semantics unchanged.
- Tests: engine tests remain green with the feature off (default). With `--features audio` on native: `cargo test -p gpui-starter --lib pages::music` still green (audio open is lazy; never triggered in tests without a playing status change through a real window; assert by keeping all existing tests' engine mutations on the main path, i.e. tests do call `play_queue`, so guard: tests construct the engine with audio disabled via `pub(crate) fn disable_audio(&mut self)` test hook; locked: `AudioBackend::open` is only invoked when `cfg` compiled AND `Self::audio_enabled` flag is true; tests set it false; document the flag).
- Done-check: `cargo clippy --workspace --all-features` and `cargo test --workspace --all-features` pass; `cargo check --target wasm32-unknown-unknown -p gpui-starter --lib` (plain `just wasm-check`) still passes with the feature off; `git diff Cargo.lock` in the commit contains rodio + cpal additions.

### Step 6.2: Capability wiring and docs

- Files: `src/features/pages/music/mod.rs` (header note), this plan's follow-through.
- Build: register `music_audio_output` (supported_enabled at first successful open, degraded on failure) per Step 6.1; module header documents the drift + no-decode tradeoffs verbatim from the phase intro.
- Tests: none beyond 6.1.
- Done-check: diagnostics page shows `music_playback` always and `music_audio_output` only in native+feature builds (manual).

## Platform & Wasm Gating

- Phases 1–5: zero `cfg` in the music module. Every API used is platform-neutral (entities, `background_executor().timer`, `img`/SVG rasterization, kit components). The SVG cover path rasterizes through the usvg renderer on both native and wasm (`GPUI/platform.rs:3296-3298`; the wasm build ships the same renderer as the gallery's embedded-SVG story, `REPO/src/gallery/sections/media/demo.rs:13-21`).
- `just wasm-check` (`cargo +nightly check --target wasm32-unknown-unknown -p gpui-starter --lib`, `justfile:24-25`) is a gate for phases 1–5: the page and bar must compile on wasm; behavior is identical (simulated playback, no audio).
- Phase 6: rodio lives only under `[target.'cfg(not(target_family = "wasm"))'.dependencies]` as an optional dep behind feature `audio`; all consuming code sits behind `#[cfg(all(feature = "audio", not(target_family = "wasm")))]`. `--features audio` on a wasm target compiles to a no-op (feature resolves, module cfg-fenced out), same convention as `notifications-portal` (`notifications-portal = ["dep:ashpd"]` at `Cargo.toml:144`, no-op comment at `:250-256`, not-wasm table at `:310`). `just wasm-check` runs with default features, so the audio path never enters the wasm check; CI has no wasm job (seams §6), making it a local gate.
- Capability registry: `music_playback`: always supported+enabled (simulated engine works everywhere), added to `DEFAULT_ENABLED_CAPABILITIES` (`REPO/src/app/init.rs:95-108`) plus registered in `music::init`. `music_audio_output`: registered only inside the Phase 6 cfg at runtime: `supported_enabled` after a successful sink open, `CapabilityStatus::degraded(err)` on open failure (`REPO/src/runtime/capabilities.rs:37-50`). Absent on wasm and on native-without-feature; absence is the honest "not compiled" signal; the diagnostics page renders whatever is registered.

## i18n

Keys land with the step that first references them (grammar: top-level quoted scalars, names `[a-z0-9_]`, both catalogs in lockstep; seams §5, `tests/qa_localization.rs:188-198`). Facade: `crate::i18n::localize` / `localize_with_args` (`REPO/src/services/i18n.rs:3-13`). `nav_music` is consumed indirectly via `title_key()` and invisible to the scanner (seams §5 blind-spot note): both files must carry it.

| key | en | zh-CN | added in |
|---|---|---|---|
| `nav_music` | "Music" | "音乐" | Step 4.4 |
| `music_bar_region` | "Now playing" | "正在播放" | Step 3.1 |
| `music_nothing_playing` | "Nothing playing" | "未在播放" | Step 3.1 |
| `music_play` | "Play" | "播放" | Step 3.1 |
| `music_pause` | "Pause" | "暂停" | Step 3.1 |
| `music_next` | "Next track" | "下一曲" | Step 3.1 |
| `music_previous` | "Previous track" | "上一曲" | Step 3.1 |
| `music_volume` | "Volume" | "音量" | Step 3.1 |
| `music_library_playlists` | "Playlists" | "播放列表" | Step 4.1 |
| `music_library_albums` | "Albums" | "专辑" | Step 4.1 |
| `music_album_count` | "%{count} tracks" | "%{count} 首曲目" | Step 4.1 |
| `music_back` | "Back to library" | "返回音乐库" | Step 4.2 |
| `music_play_all` | "Play all" | "播放全部" | Step 4.2 |
| `music_move_up` | "Move up" | "上移" | Step 4.2 |
| `music_move_down` | "Move down" | "下移" | Step 4.2 |
| `music_queue` | "Queue" | "播放队列" | Step 4.3 |
| `music_queue_empty` | "Queue is empty" | "队列为空" | Step 4.3 |
| `music_queue_clear` | "Clear" | "清空" | Step 4.3 |

Row labels (track title/artist/duration) are composed from catalog data in code, not catalog keys; the fixture is English-only data, matching the gallery's fixture convention.

## Design Compliance

- Shell/layout: the page is content inside the existing sidebar workspace shell; the now-playing bar is a persistent bottom chrome row using the status bar's surface tokens (`status_bar_border`, `tokens.status_bar`, `REPO/src/shell/status_bar.rs:92-93`) so the two bottom bands read as one system (design guides, Surfaces and elevation; Alignment details "align major regions"). Queue panel is a resizable-candidate but ships fixed `rems(20.)` with a visible toggle; inspectors "should not cover the primary object" and are dismissible (Proportion and layer hierarchy).
- Hierarchy: one level-1 heading per page is owned by the app header (`render.rs:149-158`); the page's section titles are level 2; muted `text_xs` carries artist/meta (Hierarchy; Typography). Emphasis budget: no badges, no colored panels; the only accent fills are the queue panel's current-track row and the theme's own primary on the play/pause icon button state.
- Density: compact tier for the bar (h_10 row, `xsmall` icon buttons, `text_xs` meta) per Density tiers "compact for toolbars"; the library grid is medium density (`gap_4`, `rems(10)` cards ~160 px at default rem) (Radius, spacing, and density).
- Tokens: colors from `cx.theme()` only: `background/foreground/muted_foreground/border/status_bar_border/tokens.status_bar/primary` and the accent token for selection. Raw hsl values exist only inside generated SVG cover bytes, which is raster/data color ("Raw colors belong only inside theme definitions or in audited data/raster content whose color is itself the data"). Radii via `rounded_lg()`; spacing via rem helpers (`gap_2/3/4`, `p_3/6`, `w(rems(n))`); the one px-typed value is the virtual-list row height `px(40.)`, which the `render_virtual_list` contract requires as a measured-row pin (`REPO/src/ui/widgets/virtual_list.rs:9-11` precedent, activity_log `px(20.)`).
- Interaction states: transport buttons are kit `Button`s (hover/pressed/focus/disabled come from the component); move up/down disable at bounds; play-all is the single visible primary-path action (`outline` per "default Button for ordinary visible actions"); rows/cards expose `Role::Button` + `on_a11y_action(Click)` so keyboard/AT activation exists without hover (Pointer conventions; Prefer desktop command surfaces). The queue toggle's visible state is the panel itself; its button carries an accessibility label.
- Motion: none added; the seek slider and time labels update on the 250 ms tick; no decorative animation, so reduced-motion needs no handling (Motion).
- A11y checklist mapping: bar container `Role::Region` labeled `music_bar_region`; icon-only buttons carry `accessibility_label` (button.rs:391); sliders are kit components with their own contracts, flanked by visible numeric time text so value is readable without the slider node; lists use `Role::List` + `aria_size_of_set` ancestors with `Role::ListItem` + `aria_position_in_set` rows (repo pattern `virtual_list.rs:54-88`, `notifications.rs:84-124`); the page's live region need is covered by the app header's title live region; the bar's status text (track title) is a plain paragraph: new-track announcements are intentionally not live (would chatter every auto-advance); documented as a v1 scope line.
- Interface language: nouns for destinations (Music, Albums, Playlists, Queue), verbs for commands (Play all, Move up, Clear) (Interface language); sentence case; no terminal punctuation on labels; zh-CN uses full-width punctuation only in sentences (none required here).

## Verification

Gates (AGENTS.md order; all four mandatory, wasm included because the lib gains a page):

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-features
cargo test --workspace --all-features
just wasm-check
```

Feature-specific automated coverage (wired in the phases):

```sh
cargo test -p gpui-starter --lib pages::music
cargo test --test e2e_navigation
cargo test --test qa_localization
```

Mechanical checks: `grep -rn "use gpui::" src/features/pages/music/` returns nothing (seams §9 kit-facade rule); `git diff Cargo.lock` in the Phase 6 commit contains rodio/cpal additions (qa_migration lock rule); `grep -c "music" locales/en.yml` equals the same count in `zh-CN.yml`.

Manual QA (native run, then `--features audio` run, then wasm build via `wasm/build.sh`):

1. Sidebar shows Music (icon + localized title in both locales via the settings language switch); `secondary-1..9` still land on the original nine pages (no rebinding from the append-at-end placement).
2. Library: albums grid renders 8 distinct generated covers; switching light/dark themes keeps cards legible; clicking a card opens the detail view; back returns.
3. Playlist detail: the 120-entry playlist scrolls smoothly (only visible rows render; spot-check via a debug frame-time readout on the status bar), move up/down reorder the list and persist for the session, play-all starts playback at track 1, clicking any row starts at that row.
4. Now-playing bar: present on every page; play/pause toggles, next/previous follow the contract semantics (end-of-queue stops; previous at 0 rewinds); seek drag shows scrub time live and commits on release; volume slider changes the icon-less value readout; time labels advance every 250 ms while Playing; idle state shows the muted placeholder on a fresh boot.
5. Queue panel: toggle reveals/hides; current track highlighted; click jumps; Clear empties and disables; empty state copy shows.
6. Keyboard/AT: Tab reaches transport, seek, volume, cards, rows, move buttons in visual order; Enter activates the focused card/row; focus rings visible at the bar's compact size.
7. With `--features audio`: playing a track emits a tone whose pitch varies by track; volume slider changes loudness; pause silences; end-of-queue stops audio with the UI; on a machine with no audio device the diagnostics page shows `music_audio_output` degraded and playback continues simulated. Wasm build: bar and page work, no audio, `music_audio_output` absent from diagnostics.

Acceptance criteria: all four gates pass; the three targeted test invocations pass; every manual item is confirmed or waived with a note naming what was checked instead.

## Risks & Open Questions

Ranked:

1. **rodio 0.23 is docs-verified only.** No compile was run against it (planning scope). `MixerDeviceSink::mixer() -> &Mixer`, `Player::connect_new(&Mixer)`, `Player::{append, set_volume, pause, play, stop, empty}`, `SineWave::new(freq)`, `Source::take_duration` are quoted from docs.rs pages fetched 2026-10-12. If a signature differs at compile time, the fix is contained in `audio.rs` behind the `audio` feature; phases 1–5 are unaffected. This is the top reason the phase is optional and last, and plan 19 (`docs/plans/19-audio-playback.md`) already builds a cpal-based output stack in `services::audio` that could replace this phase entirely.
2. **Seek-slider sync mutates SliderState during bar render.** `set_value` notifies the SliderState, not the bar (`BASE/slider.rs:273-284`), so no render loop; but a future kit change making state updates propagate to parents would turn this into a notify cycle. Guard: skip sync while `scrub.is_some()` and only when the value actually changed (compare to last written progress, cached on the bar). Test 3.2 plus the window smoke test cover the no-loop path (a loop would hang `run_until_parked`).
3. **Auto-advance boundary uses `position_ms >= duration` with 250 ms granularity.** A seek to exactly the duration stops playback by design (contract-consistent). Tracks shorter than TICK_MS would advance multiple times in one tick; the catalog has none (min 90 s), and the loop advances at most one track per tick by construction (single comparison per tick).
4. **Virtual list inside a page-scrolled container.** Bounded height + `on_scroll_wheel(stop_propagation)` follows the activity_log precedent exactly (`activity_log.rs:66-90`); if the wheel capture feels wrong on a trackpad, the alternative (list owns the full page height) is a layout-only change inside `page.rs`.
5. **Page count and digit bindings.** Appending `Page::Music` at the end of `all()` leaves `secondary-1..9` untouched (binding loop binds only `i < 9`, `state.rs:144-166`); if a future page needs a digit key, pages must be reordered consciously (seams §1 gotcha). `Page::Music` in persisted `AppConfig.active_route` deserializes from old configs fine (old variants unchanged; new variant only produced by new builds).
6. **No persisted music state.** Volume/queue/order reset per session, deliberate (config schema is pinned by snapshot tests, `tests/snapshot_tests.rs:26,57`); adding persistence later is a config-field addition with `#[serde(default)]`, not a contract break.

Open questions (each with what was tried):

- **Whether `Slider` renders an accessibility node with name/value on its own.** Tried: grepped `COMP/slider.rs` for role/aria: none found in the styled wrapper; the base `SliderTrack/Thumb` in `BASE/slider.rs` may add nodes. Mitigation in design: visible numeric time text flanks the seek slider, and the volume icon carries `music_volume` context. Not further verifiable without running a screen reader (manual QA item 6).
- **Exact diagnostics-page rendering of a capability registered at window-construction time rather than boot.** `capabilities::set` uses `default_global` (`capabilities.rs:64-79`) so ordering is safe; the diagnostics page re-reads `snapshot(cx)` on render (seams §4). Tried: no code path found that caches the snapshot at boot. Assumed fine; verified by manual QA item 7.
- **Whether dependents need engine events beyond observation.** The contract ships no `EventEmitter`; anything async (media keys, scrobbling) can observe. If a dependent needs "track changed" without holding the entity, that is a contract extension in a follow-up plan edit, stated here so no one works around it silently.
- **Plan 16 integration.** Feeding `WaveformState::attach_source` from the rodio tone generator would need sample-level output rodio does not expose through `Player`; it would require a cpal tap or a parallel synth. Out of scope; tried: reading plan 16's contract: its producer is push-based, rodio's Player is pull-based with no callback. Deferred until a dependent asks.
