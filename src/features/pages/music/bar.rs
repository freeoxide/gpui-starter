use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    slider::{Slider, SliderEvent, SliderState},
    v_flex,
};
use gpui_kit::{StyledImage as _, prelude::*, *};

use crate::accessibility::A11yExt as _;

use super::model::format_time;
use super::{Library, PlayerEngine, TransportStatus};

/// Persistent bottom-chrome transport row. `player` is observed for
/// re-render; mount once at the shell level.
pub struct NowPlayingBar {
    player: Entity<PlayerEngine>,
    seek: Entity<SliderState>,
    volume: Entity<SliderState>,
    scrub: Option<f32>,
    last_seek_write: f32,
    _subscriptions: Vec<Subscription>,
}

impl NowPlayingBar {
    pub fn new(player: Entity<PlayerEngine>, _window: &mut Window, cx: &mut Context<Self>) -> Self {
        let seek = cx.new(|_| {
            SliderState::new()
                .min(0.)
                .max(1.)
                .step(0.001)
                .default_value(0.)
        });
        let volume = cx.new(|_| {
            SliderState::new()
                .min(0.)
                .max(1.)
                .step(0.01)
                .default_value(0.8)
        });
        let _subscriptions = vec![
            cx.observe(&player, |_, _, cx| cx.notify()),
            cx.subscribe(&seek, |this, _, event: &SliderEvent, cx| match event {
                SliderEvent::Change(value) => {
                    this.scrub = Some(value.start());
                    cx.notify();
                }
                SliderEvent::Release(value) => this.on_seek_release(value.start(), cx),
            }),
            cx.subscribe(&volume, |this, _, event: &SliderEvent, cx| {
                if let SliderEvent::Change(value) = event {
                    this.on_volume_change(value.start(), cx);
                }
            }),
        ];
        Self {
            player,
            seek,
            volume,
            scrub: None,
            last_seek_write: 0.,
            _subscriptions,
        }
    }

    pub(crate) fn on_seek_release(&mut self, value: f32, cx: &mut Context<Self>) {
        self.scrub = None;
        let duration = self.player.read(cx).duration_ms() as f32;
        self.player.update(cx, |player, cx| {
            player.seek_ms((value * duration) as u64, cx)
        });
        cx.notify();
    }

    pub(crate) fn on_volume_change(&mut self, value: f32, cx: &mut Context<Self>) {
        self.player
            .update(cx, |player, cx| player.set_volume(value, cx));
        cx.notify();
    }

    pub(crate) fn on_toggle_play(&mut self, cx: &mut Context<Self>) {
        self.player
            .update(cx, |player, cx| player.toggle_playback(cx));
        cx.notify();
    }

    pub(crate) fn on_next(&mut self, cx: &mut Context<Self>) {
        self.player.update(cx, |player, cx| player.next(cx));
        cx.notify();
    }

    pub(crate) fn on_previous(&mut self, cx: &mut Context<Self>) {
        self.player.update(cx, |player, cx| player.previous(cx));
        cx.notify();
    }
}

impl Render for NowPlayingBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let progress = self.player.read(cx).progress();
        // SliderState::set_value notifies the SliderState only, never this
        // bar, and the last_seek_write guard makes the write conditional.
        if self.scrub.is_none() && progress != self.last_seek_write {
            self.seek
                .update(cx, |state, cx| state.set_value(progress, window, cx));
            self.last_seek_write = progress;
        }

        let engine = self.player.read(cx);
        let track = engine.current_track();
        let library = Library::catalog();
        let duration = engine.duration_ms();
        let display_ms = match self.scrub {
            Some(fraction) => (fraction * duration as f32) as u64,
            None => engine.position_ms(),
        };

        let cover_slot: AnyElement = match track
            .as_ref()
            .and_then(|track| library.track_album(track.id))
            .and_then(|album_id| library.cover(album_id))
        {
            Some(cover) => img(cover)
                .size_8()
                .rounded_lg()
                .object_fit(ObjectFit::Cover)
                .flex_shrink_0()
                .into_any_element(),
            None => div()
                .id("npb-cover-fallback")
                .size_8()
                .rounded_lg()
                .bg(cx.theme().muted)
                .flex()
                .items_center()
                .justify_center()
                .flex_shrink_0()
                .child(Icon::new(IconName::Play).xsmall())
                .into_any_element(),
        };

        let meta_column = match &track {
            Some(track) => v_flex()
                .min_w_0()
                .w(rems(24.))
                .child(
                    div()
                        .id("npb-title")
                        .truncate()
                        .text_sm()
                        .child(track.title.clone()),
                )
                .child(
                    div()
                        .id("npb-artist")
                        .truncate()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(track.artist.clone()),
                ),
            None => v_flex().min_w_0().w(rems(24.)).child(
                div()
                    .id("npb-title")
                    .truncate()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(crate::i18n::localize("music_nothing_playing")),
            ),
        };

        let playing = engine.status() == TransportStatus::Playing;
        let play_button = if playing {
            Button::new("npb-play")
                .ghost()
                .xsmall()
                .icon(IconName::Pause)
                .accessibility_label(crate::i18n::localize("music_pause"))
                .on_click(cx.listener(|this, _, _, cx| this.on_toggle_play(cx)))
        } else {
            Button::new("npb-play")
                .ghost()
                .xsmall()
                .icon(IconName::Play)
                .accessibility_label(crate::i18n::localize("music_play"))
                .on_click(cx.listener(|this, _, _, cx| this.on_toggle_play(cx)))
        };

        h_flex()
            .id("now-playing-bar")
            .a11y(Role::Region, crate::i18n::localize("music_bar_region"))
            .w_full()
            .h_10()
            .items_center()
            .gap_3()
            .px_3()
            .border_t_1()
            .border_color(cx.theme().status_bar_border)
            .bg(cx.theme().tokens.status_bar)
            .child(cover_slot)
            .child(meta_column)
            .child(
                h_flex()
                    .flex_shrink_0()
                    .gap_1()
                    .child(
                        Button::new("npb-prev")
                            .ghost()
                            .xsmall()
                            .icon(IconName::ChevronLeft)
                            .accessibility_label(crate::i18n::localize("music_previous"))
                            .on_click(cx.listener(|this, _, _, cx| this.on_previous(cx))),
                    )
                    .child(play_button)
                    .child(
                        Button::new("npb-next")
                            .ghost()
                            .xsmall()
                            .icon(IconName::ChevronRight)
                            .accessibility_label(crate::i18n::localize("music_next"))
                            .on_click(cx.listener(|this, _, _, cx| this.on_next(cx))),
                    ),
            )
            .child(
                h_flex()
                    .id("npb-time")
                    .flex_shrink_0()
                    .gap_1()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(div().child(format_time(display_ms)))
                    .child(div().child("/"))
                    .child(div().child(format_time(duration))),
            )
            .child(
                Slider::new(&self.seek)
                    .horizontal()
                    .flex_1()
                    .min_w_0()
                    .accessibility_label(crate::i18n::localize("music_seek")),
            )
            .child(
                h_flex()
                    .id("npb-volume")
                    .a11y(Role::Group, crate::i18n::localize("music_volume"))
                    .flex_shrink_0()
                    .gap_2()
                    .items_center()
                    .child(Icon::new(IconName::Settings2).xsmall())
                    .child(
                        Slider::new(&self.volume)
                            .w(rems(8.))
                            .accessibility_label(crate::i18n::localize("music_volume")),
                    ),
            )
    }
}

#[cfg(test)]
#[path = "bar.test.rs"]
mod bar_test;
