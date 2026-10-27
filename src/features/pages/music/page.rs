use std::collections::HashMap;
use std::sync::Arc;

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, IconName, Sizable as _, VirtualListScrollHandle,
    button::{Button, ButtonVariants as _},
    h_flex, v_flex,
};
use gpui_kit::{StyledImage as _, prelude::*, *};

use crate::accessibility::A11yExt as _;
use crate::ui::widgets::{
    bounded_list_height, render_virtual_list, uniform_item_sizes, virtual_list_item,
};

use super::model::{AlbumId, PlaylistId, TrackId, format_time, move_within};
use super::{Album, Library, PlayerEngine, Playlist};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum DetailSource {
    Album(AlbumId),
    Playlist(PlaylistId),
}

/// Library view with an albums grid and playlist detail. `player` is observed
/// for re-render; playback state itself lives in the engine.
pub struct MusicPage {
    player: Entity<PlayerEngine>,
    library: Arc<Library>,
    detail: Option<DetailSource>,
    orders: HashMap<PlaylistId, Vec<TrackId>>,
    queue_panel_open: bool,
    detail_scroll: VirtualListScrollHandle,
    _subscriptions: Vec<Subscription>,
}

impl MusicPage {
    pub fn new(player: Entity<PlayerEngine>, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let subscriptions = vec![cx.observe(&player, |_, _, cx| cx.notify())];
        Self {
            player,
            library: Library::catalog(),
            detail: None,
            orders: HashMap::new(),
            queue_panel_open: false,
            detail_scroll: VirtualListScrollHandle::new(),
            _subscriptions: subscriptions,
        }
    }

    pub(crate) fn open_detail(&mut self, source: DetailSource, cx: &mut Context<Self>) {
        self.detail = Some(source);
        cx.notify();
    }

    pub(crate) fn close_detail(&mut self, cx: &mut Context<Self>) {
        self.detail = None;
        cx.notify();
    }

    pub(crate) fn toggle_queue_panel(&mut self, cx: &mut Context<Self>) {
        self.queue_panel_open = !self.queue_panel_open;
        cx.notify();
    }

    pub(crate) fn move_track(&mut self, ix: usize, delta: isize, cx: &mut Context<Self>) {
        let Some(DetailSource::Playlist(id)) = self.detail else {
            return;
        };
        let catalog_order = self.playlist_catalog_order(id);
        let order = self.orders.entry(id).or_insert(catalog_order);
        if move_within(order, ix, delta) {
            cx.notify();
        }
    }

    pub(crate) fn detail_order(&self) -> Vec<TrackId> {
        match self.detail {
            Some(DetailSource::Album(id)) => self
                .library
                .album(id)
                .map_or_else(Vec::new, |album| album.track_ids.clone()),
            Some(DetailSource::Playlist(id)) => self
                .orders
                .get(&id)
                .cloned()
                .unwrap_or_else(|| self.playlist_catalog_order(id)),
            None => Vec::new(),
        }
    }

    fn playlist_catalog_order(&self, id: PlaylistId) -> Vec<TrackId> {
        self.library
            .playlist(id)
            .map_or_else(Vec::new, |playlist| playlist.track_ids.clone())
    }

    fn play_detail_at(&self, ix: usize, cx: &mut Context<Self>) {
        let order = self.detail_order();
        self.player
            .update(cx, |player, cx| player.play_queue(order, ix, cx));
    }
}

impl Render for MusicPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let content = match self.detail {
            Some(source) => self.render_detail(source, cx),
            None => self.render_library(cx),
        };

        h_flex()
            .id("music-page")
            .items_stretch()
            .w_full()
            .min_h_full()
            .p_6()
            .gap_4()
            .child(v_flex().flex_1().min_w_0().gap_6().child(content))
            .when(self.queue_panel_open, |this| {
                this.child(self.render_queue_panel(cx))
            })
    }
}

fn track_count_label(count: usize) -> String {
    crate::i18n::localize_with_args("music_album_count", &[("count", &count.to_string())])
}

impl MusicPage {
    fn render_library(&self, cx: &mut Context<Self>) -> Div {
        let playlists_label = crate::i18n::localize("music_library_playlists");
        let albums_label = crate::i18n::localize("music_library_albums");

        v_flex()
            .gap_4()
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .id("music-playlists-heading")
                            .a11y(Role::Heading, playlists_label.clone())
                            .aria_level(2)
                            .text_lg()
                            .font_weight(FontWeight::BOLD)
                            .child(playlists_label),
                    )
                    .child(self.render_queue_toggle(cx)),
            )
            .children(
                self.library
                    .playlists()
                    .iter()
                    .map(|playlist| self.render_playlist_row(playlist, cx)),
            )
            .child(
                div()
                    .id("music-albums-heading")
                    .a11y(Role::Heading, albums_label.clone())
                    .aria_level(2)
                    .text_lg()
                    .font_weight(FontWeight::BOLD)
                    .child(albums_label),
            )
            .child(
                h_flex().flex_wrap().gap_4().children(
                    self.library
                        .albums()
                        .iter()
                        .map(|album| self.render_album_card(album, cx)),
                ),
            )
    }

    fn render_playlist_row(&self, playlist: &Playlist, cx: &mut Context<Self>) -> Stateful<Div> {
        let muted = cx.theme().muted;
        let muted_foreground = cx.theme().muted_foreground;
        let count_label = track_count_label(playlist.track_ids.len());
        let label = format!("{}. {}", playlist.title, count_label);
        let source = DetailSource::Playlist(playlist.id);
        let page = cx.entity();
        div()
            .id(("music-playlist", playlist.id.0))
            .a11y(Role::Button, label)
            .rounded_lg()
            .px_3()
            .py_2()
            .hover(move |style| style.bg(muted))
            .on_click(cx.listener(move |this, _, _, cx| this.open_detail(source, cx)))
            .on_a11y_action(AccessibleAction::Click, move |_, _, cx| {
                page.update(cx, |this, cx| this.open_detail(source, cx));
            })
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(div().text_sm().child(playlist.title.clone()))
                    .child(
                        div()
                            .text_xs()
                            .text_color(muted_foreground)
                            .child(count_label),
                    ),
            )
    }

    fn render_album_card(&self, album: &Album, cx: &mut Context<Self>) -> Stateful<Div> {
        let muted_foreground = cx.theme().muted_foreground;
        let label = format!("{}. {}", album.title, album.artist);
        let source = DetailSource::Album(album.id);
        let cover = self.library.cover(album.id);
        let page = cx.entity();
        v_flex()
            .id(("music-album", album.id.0))
            .a11y(Role::Button, label)
            .w(rems(10.))
            .gap_2()
            .on_click(cx.listener(move |this, _, _, cx| this.open_detail(source, cx)))
            .on_a11y_action(AccessibleAction::Click, move |_, _, cx| {
                page.update(cx, |this, cx| this.open_detail(source, cx));
            })
            .when_some(cover, |this, cover| {
                this.child(
                    img(cover)
                        .w_full()
                        .h(rems(10.))
                        .rounded_lg()
                        .object_fit(ObjectFit::Cover),
                )
            })
            .child(
                div()
                    .truncate()
                    .text_sm()
                    .font_weight(FontWeight::MEDIUM)
                    .child(album.title.clone()),
            )
            .child(
                div()
                    .truncate()
                    .text_xs()
                    .text_color(muted_foreground)
                    .child(album.artist.clone()),
            )
    }

    fn render_queue_toggle(&self, cx: &mut Context<Self>) -> Button {
        Button::new("music-queue-toggle")
            .ghost()
            .xsmall()
            .icon(IconName::Menu)
            .accessibility_label(crate::i18n::localize("music_queue"))
            .on_click(cx.listener(|this, _, _, cx| this.toggle_queue_panel(cx)))
    }

    fn render_detail(&self, source: DetailSource, cx: &mut Context<Self>) -> Div {
        let title = match source {
            DetailSource::Album(id) => self
                .library
                .album(id)
                .map_or_else(String::new, |album| album.title.to_string()),
            DetailSource::Playlist(id) => self
                .library
                .playlist(id)
                .map_or_else(String::new, |playlist| playlist.title.to_string()),
        };
        let order = self.detail_order();
        let track_count = order.len();
        let item_sizes = uniform_item_sizes(track_count, px(40.));
        let list_height = bounded_list_height(&item_sizes, px(0.), px(480.));
        let is_playlist = matches!(source, DetailSource::Playlist(_));
        let source_key = match source {
            DetailSource::Album(id) => id.0,
            DetailSource::Playlist(id) => id.0,
        };
        let scroll_handle = self.detail_scroll.clone();

        v_flex()
            .gap_4()
            .child(
                h_flex()
                    .items_center()
                    .gap_2()
                    .flex_wrap()
                    .child(
                        Button::new("music-back")
                            .ghost()
                            .xsmall()
                            .icon(IconName::ArrowLeft)
                            .accessibility_label(crate::i18n::localize("music_back"))
                            .on_click(cx.listener(|this, _, _, cx| this.close_detail(cx))),
                    )
                    .child(
                        div()
                            .id("music-detail-title")
                            .a11y(Role::Heading, title.clone())
                            .aria_level(2)
                            .text_xl()
                            .font_weight(FontWeight::BOLD)
                            .truncate()
                            .min_w_0()
                            .child(title),
                    )
                    .child(
                        div()
                            .id("music-detail-count")
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(track_count_label(track_count)),
                    )
                    .child(
                        Button::new("music-play-all")
                            .outline()
                            .xsmall()
                            .icon(IconName::Play)
                            .label(crate::i18n::localize("music_play_all"))
                            .on_click(cx.listener(|this, _, _, cx| this.play_detail_at(0, cx))),
                    )
                    .child(self.render_queue_toggle(cx)),
            )
            .child(
                div()
                    .on_scroll(|_, _, cx| cx.stop_propagation())
                    .child(render_virtual_list(
                        cx,
                        ("music-tracks", source_key),
                        item_sizes,
                        list_height,
                        px(0.),
                        &scroll_handle,
                        true,
                        move |this: &mut Self, visible, _window, cx| {
                            let order = this.detail_order();
                            let total = order.len();
                            visible
                                .filter_map(|ix| {
                                    let track_id = *order.get(ix)?;
                                    let track = this.library.track(track_id)?;
                                    let page = cx.entity();
                                    let duration = format_time(track.duration_ms);
                                    let label = format!(
                                        "{}. {}. {} ({})",
                                        ix + 1,
                                        track.title,
                                        track.artist,
                                        duration
                                    );
                                    let row =
                                        virtual_list_item(("music-track", ix), label, ix, total)
                                            .h(px(40.))
                                            .on_click(cx.listener(move |this, _, _, cx| {
                                                this.play_detail_at(ix, cx);
                                            }))
                                            .on_a11y_action(
                                                AccessibleAction::Click,
                                                move |_, _, cx| {
                                                    page.update(cx, |this, cx| {
                                                        this.play_detail_at(ix, cx);
                                                    });
                                                },
                                            )
                                            .child(render_track_row(
                                                track.title.clone(),
                                                duration,
                                                is_playlist,
                                                ix,
                                                total,
                                                cx,
                                            ));
                                    Some(row.into_any_element())
                                })
                                .collect()
                        },
                    )),
            )
    }

    fn render_queue_panel(&self, cx: &mut Context<Self>) -> Div {
        let queue_label = crate::i18n::localize("music_queue");
        let accent = cx.theme().accent;
        let accent_foreground = cx.theme().accent_foreground;
        let muted = cx.theme().muted;
        let muted_foreground = cx.theme().muted_foreground;
        let queue = self.player.read(cx).queue().to_vec();
        let current_ix = self.player.read(cx).current_ix();

        let body = if queue.is_empty() {
            v_flex()
                .id("music-queue-empty")
                .a11y(Role::Paragraph, crate::i18n::localize("music_queue_empty"))
                .text_sm()
                .text_color(muted_foreground)
                .child(crate::i18n::localize("music_queue_empty"))
                .into_any_element()
        } else {
            let total = queue.len();
            v_flex()
                .id("music-queue-list")
                .a11y(Role::List, crate::i18n::localize("music_queue"))
                .aria_size_of_set(total)
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .gap_1()
                .children(queue.iter().enumerate().map(|(ix, track_id)| {
                    let track = self.library.track(*track_id);
                    let title =
                        track.map_or_else(|| ix.to_string(), |track| track.title.to_string());
                    let label = track.map_or_else(
                        || title.clone(),
                        |track| format!("{}. {}", track.title, track.artist),
                    );
                    let is_current = current_ix == Some(ix);
                    let hover_bg = if is_current { accent } else { muted };
                    div()
                        .id(("music-queue-item", ix))
                        .a11y(Role::ListItem, label)
                        .aria_position_in_set(ix)
                        .aria_size_of_set(total)
                        .rounded_lg()
                        .px_2()
                        .py_1()
                        .when(is_current, |this| {
                            this.bg(accent).text_color(accent_foreground)
                        })
                        .hover(move |style| style.bg(hover_bg))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.player
                                .update(cx, |player, cx| player.play_queue_index(ix, cx));
                        }))
                        .child(div().truncate().text_sm().child(title))
                }))
                .into_any_element()
        };

        v_flex()
            .w(rems(20.))
            .flex_shrink_0()
            .border_l_1()
            .border_color(cx.theme().border)
            .p_3()
            .gap_2()
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .id("music-queue-heading")
                            .a11y(Role::Heading, queue_label.clone())
                            .aria_level(2)
                            .text_sm()
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(queue_label),
                    )
                    .child(
                        h_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .id("music-queue-count")
                                    .text_xs()
                                    .text_color(muted_foreground)
                                    .child(track_count_label(queue.len())),
                            )
                            .child(
                                Button::new("music-queue-clear")
                                    .ghost()
                                    .xsmall()
                                    .label(crate::i18n::localize("music_queue_clear"))
                                    .disabled(queue.is_empty())
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.player.update(cx, |player, cx| player.clear_queue(cx));
                                    })),
                            ),
                    ),
            )
            .child(body)
    }
}

fn render_track_row(
    title: SharedString,
    duration: SharedString,
    is_playlist: bool,
    ix: usize,
    total: usize,
    cx: &mut Context<MusicPage>,
) -> impl IntoElement {
    let muted_foreground = cx.theme().muted_foreground;
    let move_up_label = crate::i18n::localize("music_move_up");
    let move_down_label = crate::i18n::localize("music_move_down");
    let move_lane = is_playlist.then(|| {
        h_flex()
            .flex_shrink_0()
            .gap_1()
            .child(
                Button::new(("music-up", ix))
                    .ghost()
                    .xsmall()
                    .icon(IconName::ArrowUp)
                    .accessibility_label(move_up_label)
                    .disabled(ix == 0)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.move_track(ix, -1, cx);
                    })),
            )
            .child(
                Button::new(("music-down", ix))
                    .ghost()
                    .xsmall()
                    .icon(IconName::ArrowDown)
                    .accessibility_label(move_down_label)
                    .disabled(ix + 1 >= total)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.move_track(ix, 1, cx);
                    })),
            )
    });

    h_flex()
        .h_full()
        .items_center()
        .gap_2()
        .min_w_0()
        .child(
            div()
                .text_xs()
                .text_color(muted_foreground)
                .flex_shrink_0()
                .w(rems(2.))
                .child((ix + 1).to_string()),
        )
        .child(div().flex_1().min_w_0().truncate().text_sm().child(title))
        .child(
            div()
                .text_xs()
                .text_color(muted_foreground)
                .flex_shrink_0()
                .child(duration),
        )
        .when_some(move_lane, |this, lane| this.child(lane))
}

#[cfg(test)]
#[path = "page.test.rs"]
mod page_test;
