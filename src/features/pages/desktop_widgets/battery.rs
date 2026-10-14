use gpui_kit::component::{ActiveTheme as _, Icon, IconName, h_flex, v_flex};
use gpui_kit::{prelude::*, *};

use crate::system_controls::{BatteryChargeState, BatteryStatus, ControlState};

use super::catalog::WidgetKind;

pub(crate) struct BatteryModel {
    pub icon: IconName,
    pub pct: String,
    pub state_key: Option<&'static str>,
    pub no_data: Option<SharedString>,
}

pub(crate) fn battery_model(state: &ControlState<BatteryStatus>) -> BatteryModel {
    match state {
        ControlState::Ready(battery) => BatteryModel {
            icon: charge_icon(battery),
            pct: format!("{:.0}%", battery.charge * 100.),
            state_key: state_line_key(battery.state),
            no_data: None,
        },
        state => BatteryModel {
            icon: IconName::Battery,
            pct: "--".to_string(),
            state_key: None,
            no_data: state.reason(),
        },
    }
}

fn state_line_key(state: BatteryChargeState) -> Option<&'static str> {
    match state {
        BatteryChargeState::Charging => Some("widgets_battery_charging"),
        BatteryChargeState::Full => Some("widgets_battery_full"),
        BatteryChargeState::Discharging => Some("widgets_battery_discharging"),
        BatteryChargeState::Empty | BatteryChargeState::Paused | BatteryChargeState::Unknown => {
            None
        }
    }
}

fn charge_icon(battery: &BatteryStatus) -> IconName {
    match battery.state {
        BatteryChargeState::Charging => IconName::BatteryCharging,
        BatteryChargeState::Full => IconName::BatteryFull,
        BatteryChargeState::Discharging => {
            if battery.charge < 0.1 {
                IconName::BatteryWarning
            } else if battery.charge < 0.25 {
                IconName::BatteryLow
            } else {
                IconName::BatteryMedium
            }
        }
        BatteryChargeState::Empty => IconName::BatteryLow,
        BatteryChargeState::Paused | BatteryChargeState::Unknown => IconName::Battery,
    }
}

pub(crate) struct BatteryWidget {
    _subscriptions: Vec<Subscription>,
}

impl BatteryWidget {
    pub(crate) fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let _subscriptions = vec![
            cx.observe_global::<crate::system_controls::SystemControlsSnapshot>(|_, cx| {
                cx.notify();
            }),
        ];
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(std::time::Duration::from_secs(30))
                    .await;
                if this
                    .update(cx, |_, cx| crate::system_controls::refresh(cx))
                    .is_err()
                {
                    break;
                }
            }
        })
        .detach();
        Self { _subscriptions }
    }

    pub(crate) fn view(window: &mut Window, cx: &mut App) -> Entity<BatteryWidget> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for BatteryWidget {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let model = battery_model(&crate::system_controls::snapshot(cx).battery);
        let muted = cx.theme().muted_foreground;
        v_flex()
            .size_full()
            .p_3()
            .gap_2()
            .items_start()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(div().size_4().flex_shrink_0().child(Icon::new(model.icon)))
                    .child(
                        div()
                            .text_sm()
                            .text_color(muted)
                            .child(crate::i18n::localize(WidgetKind::Battery.title_key())),
                    ),
            )
            .child(
                div()
                    .text_2xl()
                    .font_weight(FontWeight::BOLD)
                    .child(model.pct),
            )
            .when_some(model.state_key, |this, key| {
                this.child(
                    div()
                        .text_sm()
                        .text_color(muted)
                        .child(crate::i18n::localize(key)),
                )
            })
            .when_some(model.no_data, |this, reason| {
                this.child(
                    v_flex()
                        .gap_1()
                        .child(
                            div()
                                .text_sm()
                                .text_color(muted)
                                .child(crate::i18n::localize("widgets_no_data")),
                        )
                        .child(
                            div()
                                .text_sm()
                                .text_color(muted)
                                .child(crate::i18n::localize(&reason)),
                        ),
                )
            })
    }
}

#[cfg(test)]
#[path = "battery.test.rs"]
mod battery_test;
