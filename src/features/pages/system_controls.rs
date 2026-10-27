use gpui_kit::component::button::Button;
use gpui_kit::component::label::Label;
use gpui_kit::component::slider::{Slider, SliderEvent, SliderState};
use gpui_kit::component::switch::Switch;
use gpui_kit::component::{ActiveTheme as _, Disableable as _, Icon, IconName, h_flex, v_flex};
use gpui_kit::{prelude::*, *};

use crate::accessibility::A11yExt as _;
use crate::system_controls::{
    BatteryChargeState, BatteryStatus, BluetoothAccess, BluetoothStatus, ControlState,
    REASON_MACOS_BLUETOOTH_POWER, SystemControlsSnapshot,
};

#[cfg(not(target_family = "wasm"))]
const INTERVAL: std::time::Duration = std::time::Duration::from_secs(2);

pub struct SystemControlsPage {
    volume: Entity<SliderState>,
    brightness: Entity<SliderState>,
    _subscriptions: Vec<Subscription>,
}

impl SystemControlsPage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let volume = cx.new(|_| {
            SliderState::new()
                .min(0.)
                .max(1.)
                .step(0.01)
                .default_value(0.)
        });
        let brightness = cx.new(|_| {
            SliderState::new()
                .min(0.)
                .max(1.)
                .step(0.01)
                .default_value(0.)
        });
        let _subscriptions = vec![
            cx.subscribe(&volume, |_, _, event: &SliderEvent, cx| match event {
                SliderEvent::Change(_) => cx.notify(),
                SliderEvent::Release(value) => {
                    crate::system_controls::set_volume(value.start(), cx);
                }
            }),
            cx.subscribe(&brightness, |_, _, event: &SliderEvent, cx| match event {
                SliderEvent::Change(_) => cx.notify(),
                SliderEvent::Release(value) => {
                    crate::system_controls::set_brightness(value.start(), cx);
                }
            }),
            cx.observe_global_in::<SystemControlsSnapshot>(window, |this, window, cx| {
                this.sync_sliders(window, cx);
                cx.notify();
            }),
        ];

        #[cfg(not(target_family = "wasm"))]
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(INTERVAL).await;
                let result = this.update(cx, |_, cx| crate::system_controls::refresh(cx));
                if result.is_err() {
                    break;
                }
            }
        })
        .detach();
        crate::system_controls::refresh(cx);

        Self {
            volume,
            brightness,
            _subscriptions,
        }
    }

    fn sync_sliders(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let snapshot = crate::system_controls::snapshot(cx);
        if let ControlState::Ready(volume) = &snapshot.volume {
            let level = volume.level;
            self.volume.update(cx, |state, cx| {
                if (state.value().start() - level).abs() > f32::EPSILON {
                    state.set_value(level, window, cx);
                }
            });
        }
        if let ControlState::Ready(brightness) = &snapshot.brightness {
            let level = brightness.level;
            self.brightness.update(cx, |state, cx| {
                if (state.value().start() - level).abs() > f32::EPSILON {
                    state.set_value(level, window, cx);
                }
            });
        }
    }

    fn render_battery(
        &self,
        snapshot: &SystemControlsSnapshot,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let label = crate::i18n::localize("system_controls_battery");
        let (icon, body) = match &snapshot.battery {
            ControlState::Ready(battery) => {
                let state_text = crate::i18n::localize(battery_state_key(battery.state));
                let low_power = match battery.low_power_mode {
                    Some(true) => crate::i18n::localize("system_controls_on"),
                    Some(false) => crate::i18n::localize("system_controls_off"),
                    None => crate::i18n::localize("system_controls_not_exposed"),
                };
                (
                    battery_icon(battery),
                    v_flex()
                        .gap_2()
                        .child(value_row(
                            crate::i18n::localize("system_controls_charge"),
                            format!("{}% · {}", (battery.charge * 100.).round(), state_text),
                            cx,
                        ))
                        .child(value_row(
                            crate::i18n::localize("system_controls_low_power_mode"),
                            low_power,
                            cx,
                        ))
                        .into_any_element(),
                )
            }
            state => (IconName::Battery, reason_row(state, cx).into_any_element()),
        };
        group("battery-group", label, icon, None, cx).child(body)
    }

    fn render_bluetooth(
        &self,
        snapshot: &SystemControlsSnapshot,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let label = crate::i18n::localize("system_controls_bluetooth");
        let body = match bluetooth_panel(snapshot) {
            BluetoothPanel::Enable { busy } => enable_body(busy, cx).into_any_element(),
            BluetoothPanel::Live => live_body(snapshot, cx).into_any_element(),
            BluetoothPanel::UnsupportedPlatform => {
                reason_row(&snapshot.bluetooth, cx).into_any_element()
            }
            BluetoothPanel::Denied { reason } => denied_body(reason, cx).into_any_element(),
        };
        group("bluetooth-group", label, IconName::Network, None, cx).child(body)
    }

    fn render_volume(
        &self,
        snapshot: &SystemControlsSnapshot,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let label = crate::i18n::localize("system_controls_volume");
        match &snapshot.volume {
            ControlState::Ready(volume) => {
                let mute_label = crate::i18n::localize("system_controls_mute");
                let status = format!("{:.0}%", self.volume.read(cx).value().start() * 100.);
                group(
                    "volume-group",
                    label.clone(),
                    IconName::Settings2,
                    Some(status),
                    cx,
                )
                .child(
                    v_flex()
                        .gap_2()
                        .child(Slider::new(&self.volume).accessibility_label(label))
                        .child(
                            h_flex()
                                .items_center()
                                .justify_between()
                                .child(Label::new(mute_label.clone()))
                                .child(
                                    Switch::new("volume-mute")
                                        .accessibility_label(mute_label)
                                        .checked(volume.muted)
                                        .on_click(|checked, _, cx| {
                                            crate::system_controls::set_volume_muted(*checked, cx);
                                        }),
                                ),
                        ),
                )
            }
            state => group("volume-group", label, IconName::Settings2, None, cx)
                .child(reason_row(state, cx)),
        }
    }

    fn render_brightness(
        &self,
        snapshot: &SystemControlsSnapshot,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let label = crate::i18n::localize("system_controls_brightness");
        match &snapshot.brightness {
            ControlState::Ready(_) => {
                let status = format!("{:.0}%", self.brightness.read(cx).value().start() * 100.);
                group(
                    "brightness-group",
                    label.clone(),
                    IconName::Sun,
                    Some(status),
                    cx,
                )
                .child(Slider::new(&self.brightness).accessibility_label(label))
            }
            state => group("brightness-group", label, IconName::Sun, None, cx)
                .child(reason_row(state, cx)),
        }
    }
}

impl Render for SystemControlsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let snapshot = crate::system_controls::snapshot(cx);
        let title = crate::i18n::localize("system_controls_title");
        v_flex()
            .min_h_full()
            .p_6()
            .gap_4()
            .child(
                div()
                    .id("system-controls-title")
                    .a11y(Role::Heading, title.clone())
                    .aria_level(1)
                    .text_xl()
                    .font_weight(FontWeight::BOLD)
                    .child(title),
            )
            .child(self.render_battery(&snapshot, cx))
            .child(self.render_bluetooth(&snapshot, cx))
            .child(self.render_volume(&snapshot, cx))
            .child(self.render_brightness(&snapshot, cx))
    }
}

fn battery_state_key(state: BatteryChargeState) -> &'static str {
    match state {
        BatteryChargeState::Charging => "system_controls_state_charging",
        BatteryChargeState::Discharging => "system_controls_state_discharging",
        BatteryChargeState::Full => "system_controls_state_full",
        BatteryChargeState::Empty => "system_controls_state_empty",
        BatteryChargeState::Paused => "system_controls_state_paused",
        BatteryChargeState::Unknown => "system_controls_state_unknown",
    }
}

fn battery_icon(battery: &BatteryStatus) -> IconName {
    match battery.state {
        BatteryChargeState::Charging => IconName::BatteryCharging,
        BatteryChargeState::Full => IconName::BatteryFull,
        _ if battery.charge < 0.2 => IconName::BatteryLow,
        _ => IconName::Battery,
    }
}

fn group(
    id: &'static str,
    label: String,
    icon: IconName,
    status: Option<String>,
    cx: &App,
) -> Stateful<Div> {
    let header = h_flex()
        .items_center()
        .justify_between()
        .child(
            h_flex()
                .items_center()
                .gap_2()
                .child(div().size_4().flex_shrink_0().child(Icon::new(icon)))
                .child(Label::new(label.clone())),
        )
        .when_some(status, |this, status| {
            this.child(
                div()
                    .text_sm()
                    .text_color(cx.theme().muted_foreground)
                    .child(status),
            )
        });
    v_flex()
        .gap_2()
        .p_3()
        .border_1()
        .border_color(cx.theme().border)
        .rounded(cx.theme().radius_lg)
        .id(id)
        .a11y(Role::Group, label)
        .child(header)
}

fn value_row(label: String, value: String, cx: &App) -> Div {
    h_flex()
        .items_center()
        .justify_between()
        .child(Label::new(label))
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground)
                .child(value),
        )
}

fn reason_row<T>(state: &ControlState<T>, cx: &App) -> Div {
    let detail = match state {
        ControlState::Unsupported { reason_key } => crate::i18n::localize_with_args(
            "system_controls_unsupported_detail",
            &[("reason", &crate::i18n::localize(reason_key))],
        ),
        ControlState::Unavailable { error } => crate::i18n::localize_with_args(
            "system_controls_unavailable_detail",
            &[("error", error)],
        ),
        ControlState::Ready(_) => String::new(),
    };
    div()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(detail)
}

/// What the Bluetooth section renders for a gate state. Branch on the
/// `BluetoothAccess` enum, never on the Denied reason text.
#[derive(Clone, Debug, PartialEq)]
enum BluetoothPanel {
    Enable { busy: bool },
    Live,
    UnsupportedPlatform,
    Denied { reason: SharedString },
}

fn bluetooth_panel(snapshot: &SystemControlsSnapshot) -> BluetoothPanel {
    match &snapshot.bluetooth_access {
        BluetoothAccess::NotRequested => BluetoothPanel::Enable { busy: false },
        BluetoothAccess::Requesting => BluetoothPanel::Enable { busy: true },
        BluetoothAccess::Active => BluetoothPanel::Live,
        BluetoothAccess::Denied { reason } => match snapshot.bluetooth {
            ControlState::Unsupported { .. } => BluetoothPanel::UnsupportedPlatform,
            _ => BluetoothPanel::Denied {
                reason: reason.clone(),
            },
        },
    }
}

fn hint_text(text: SharedString, cx: &App) -> Div {
    div()
        .text_sm()
        .text_color(cx.theme().muted_foreground)
        .child(text)
}

fn enable_body(busy: bool, cx: &App) -> Div {
    // A loading button is inert, so the handshake cannot be double-fired.
    let enable = Button::new("bluetooth-enable")
        .label(crate::i18n::localize("system_controls_bluetooth_enable"))
        .when(busy, |button| {
            button.icon(IconName::LoaderCircle).loading(true)
        })
        .on_click(|_, _, cx| crate::system_controls::enable_bluetooth(cx));
    v_flex()
        .gap_2()
        .child(hint_text(
            crate::i18n::localize("system_controls_bluetooth_enable_hint").into(),
            cx,
        ))
        .child(h_flex().child(enable))
}

fn live_body(snapshot: &SystemControlsSnapshot, cx: &App) -> Div {
    let rows = match &snapshot.bluetooth {
        ControlState::Ready(bluetooth) => v_flex()
            .gap_2()
            .child(powered_row(bluetooth, cx))
            .child(devices_list(bluetooth, cx))
            .into_any_element(),
        state => reason_row(state, cx).into_any_element(),
    };
    v_flex().gap_2().child(rows).child(
        h_flex().child(
            Button::new("bluetooth-stop")
                .outline()
                .label(crate::i18n::localize("system_controls_bluetooth_stop"))
                .on_click(|_, _, cx| crate::system_controls::disable_bluetooth(cx)),
        ),
    )
}

fn denied_body(reason: SharedString, cx: &App) -> Div {
    v_flex()
        .gap_2()
        .child(hint_text(
            crate::i18n::localize("system_controls_bluetooth_denied_hint").into(),
            cx,
        ))
        .child(hint_text(reason, cx))
        .child(
            h_flex().child(
                Button::new("bluetooth-retry")
                    .label(crate::i18n::localize("system_controls_bluetooth_retry"))
                    // Manual only: the service never re-polls a denied gate.
                    .on_click(|_, _, cx| crate::system_controls::enable_bluetooth(cx)),
            ),
        )
}

fn powered_row(bluetooth: &BluetoothStatus, cx: &App) -> Div {
    let label = crate::i18n::localize("system_controls_powered");
    let Some(powered) = bluetooth.powered else {
        return value_row(
            label,
            crate::i18n::localize("system_controls_not_exposed"),
            cx,
        );
    };
    let can_toggle = cfg!(target_os = "linux");
    h_flex()
        .flex_wrap()
        .items_center()
        .justify_between()
        .gap_2()
        .child(
            h_flex()
                .items_center()
                .gap_2()
                .child(Label::new(label.clone()))
                .when(!can_toggle, |this| {
                    this.child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(crate::i18n::localize(REASON_MACOS_BLUETOOTH_POWER)),
                    )
                }),
        )
        .child(
            Switch::new("bluetooth-powered")
                .accessibility_label(label)
                .checked(powered)
                .disabled(!can_toggle)
                .on_click(|checked, _, cx| {
                    crate::system_controls::set_bluetooth_powered(*checked, cx);
                }),
        )
}

fn devices_list(bluetooth: &BluetoothStatus, cx: &App) -> AnyElement {
    if bluetooth.devices.is_empty() {
        return div()
            .text_sm()
            .text_color(cx.theme().muted_foreground)
            .child(crate::i18n::localize("system_controls_no_devices"))
            .into_any_element();
    }
    let devices = &bluetooth.devices;
    let label = crate::i18n::localize("system_controls_devices");
    div()
        .id("bluetooth-devices")
        .a11y(Role::List, label)
        .gap_1()
        .aria_size_of_set(devices.len())
        .children(devices.iter().enumerate().map(|(index, device)| {
            let connected = if device.connected {
                crate::i18n::localize("system_controls_connected")
            } else {
                crate::i18n::localize("system_controls_not_connected")
            };
            let item_label = format!("{}. {connected}.", device.alias);
            h_flex()
                .id(ElementId::Name(SharedString::from(format!(
                    "bt-device-{}",
                    device.address
                ))))
                .a11y(Role::ListItem, item_label)
                .aria_position_in_set(index)
                .aria_size_of_set(devices.len())
                .items_center()
                .justify_between()
                .child(div().min_w_0().child(device.alias.clone()))
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .flex_shrink_0()
                        .child(connected),
                )
        }))
        .into_any_element()
}

#[cfg(test)]
#[path = "system_controls.test.rs"]
mod system_controls_test;
