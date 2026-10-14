use gpui_kit::component::slider::{Slider, SliderEvent, SliderState, SliderValue};
use gpui_kit::component::{ActiveTheme as _, Icon, IconName, h_flex, v_flex};
use gpui_kit::{prelude::*, *};

use crate::platform::clock::Instant;
use crate::system_controls::{ControlState, SystemControlsSnapshot};

pub(crate) fn percent_of(level: f32) -> f32 {
    (level.clamp(0., 1.) * 100.).round()
}

pub(crate) fn level_of(percent: f32) -> f32 {
    (percent / 100.).clamp(0., 1.)
}

pub(crate) fn should_sync(now: Instant, suppress_until: Option<Instant>) -> bool {
    suppress_until.is_none_or(|t| now >= t)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SliderTarget {
    Brightness,
    Volume,
}

pub(crate) struct SystemSlidersWidget {
    brightness: Entity<SliderState>,
    volume: Entity<SliderState>,
    suppress_until: Option<Instant>,
    _subscriptions: Vec<Subscription>,
}

fn seed_percent<T>(state: &ControlState<T>, level: impl Fn(&T) -> f32) -> f32 {
    match state {
        ControlState::Ready(value) => percent_of(level(value)),
        _ => 0.,
    }
}

fn suppress_window() -> Instant {
    Instant::now() + std::time::Duration::from_secs(2)
}

impl SystemSlidersWidget {
    pub(crate) fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let snapshot = crate::system_controls::snapshot(cx);
        let brightness = cx.new(|_| {
            SliderState::new()
                .min(0.)
                .max(100.)
                .step(1.)
                .default_value(seed_percent(&snapshot.brightness, |b| b.level))
        });
        let volume = cx.new(|_| {
            SliderState::new()
                .min(0.)
                .max(100.)
                .step(1.)
                .default_value(seed_percent(&snapshot.volume, |v| v.level))
        });
        let _subscriptions = vec![
            cx.subscribe(
                &brightness,
                |this: &mut Self,
                 _: Entity<SliderState>,
                 event: &SliderEvent,
                 cx: &mut Context<Self>| {
                    this.on_event(SliderTarget::Brightness, event, cx);
                },
            ),
            cx.subscribe(
                &volume,
                |this: &mut Self,
                 _: Entity<SliderState>,
                 event: &SliderEvent,
                 cx: &mut Context<Self>| {
                    this.on_event(SliderTarget::Volume, event, cx);
                },
            ),
            cx.observe_global_in::<SystemControlsSnapshot>(window, |this, window, cx| {
                this.sync_from_snapshot(window, cx);
                cx.notify();
            }),
        ];
        Self {
            brightness,
            volume,
            suppress_until: None,
            _subscriptions,
        }
    }

    pub(crate) fn view(window: &mut Window, cx: &mut App) -> Entity<SystemSlidersWidget> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn on_event(&mut self, target: SliderTarget, event: &SliderEvent, cx: &mut App) {
        match event {
            SliderEvent::Change(_) => {
                self.suppress_until = Some(suppress_window());
            }
            SliderEvent::Release(value) => {
                let SliderValue::Single(percent) = value else {
                    return;
                };
                self.commit(target, *percent, cx);
            }
        }
    }

    fn commit(&mut self, target: SliderTarget, percent: f32, cx: &mut App) {
        match target {
            SliderTarget::Brightness => {
                crate::system_controls::set_brightness(level_of(percent), cx);
            }
            SliderTarget::Volume => {
                crate::system_controls::set_volume(level_of(percent), cx);
            }
        }
        self.suppress_until = Some(suppress_window());
    }

    fn sync_from_snapshot(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if !should_sync(Instant::now(), self.suppress_until) {
            return;
        }
        let snapshot = crate::system_controls::snapshot(cx);
        if let ControlState::Ready(brightness) = &snapshot.brightness {
            let percent = percent_of(brightness.level);
            self.brightness
                .update(cx, |state, cx| state.set_value(percent, window, cx));
        }
        if let ControlState::Ready(volume) = &snapshot.volume {
            let percent = percent_of(volume.level);
            self.volume
                .update(cx, |state, cx| state.set_value(percent, window, cx));
        }
    }

    fn render_row<T>(
        &self,
        slider: &Entity<SliderState>,
        state: &ControlState<T>,
        icon: IconName,
        label_key: &'static str,
        muted: Hsla,
        cx: &App,
    ) -> Div {
        let label = h_flex()
            .gap_2()
            .items_center()
            .child(div().size_4().flex_shrink_0().child(Icon::new(icon)))
            .child(
                div()
                    .text_sm()
                    .text_color(muted)
                    .child(crate::i18n::localize(label_key)),
            );
        match state {
            ControlState::Ready(_) => {
                let percent = slider.read(cx).value().start();
                v_flex()
                    .gap_1()
                    .child(
                        label.child(
                            div()
                                .text_sm()
                                .text_color(cx.theme().foreground)
                                .child(format!("{:.0}%", percent)),
                        ),
                    )
                    .child(Slider::new(slider).horizontal())
            }
            state => v_flex()
                .gap_1()
                .child(label)
                .child(Slider::new(slider).horizontal().disabled(true))
                .child(
                    v_flex().gap_1().child(
                        div()
                            .text_sm()
                            .text_color(muted)
                            .child(crate::i18n::localize("widgets_no_data")),
                    ),
                )
                .when_some(state.reason(), |this, reason| {
                    this.child(
                        div()
                            .text_sm()
                            .text_color(muted)
                            .child(crate::i18n::localize(&reason)),
                    )
                }),
        }
    }
}

impl Render for SystemSlidersWidget {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = crate::system_controls::snapshot(cx);
        let muted = cx.theme().muted_foreground;
        v_flex()
            .size_full()
            .p_3()
            .gap_3()
            .child(self.render_row(
                &self.brightness,
                &snapshot.brightness,
                IconName::Sun,
                "widgets_brightness",
                muted,
                cx,
            ))
            .child(self.render_row(
                &self.volume,
                &snapshot.volume,
                IconName::Settings2,
                "widgets_volume",
                muted,
                cx,
            ))
    }
}

#[cfg(test)]
#[path = "system_sliders.test.rs"]
mod system_sliders_test;
