use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, IconName, IndexPath, Sizable as _,
    alert::Alert,
    button::Button,
    h_flex,
    label::Label,
    progress::Progress,
    select::{Select, SelectEvent, SelectState},
    slider::{Slider, SliderEvent, SliderState},
    switch::Switch,
    v_flex,
};
use gpui_kit::{prelude::*, *};

use crate::accessibility::A11yExt as _;
use crate::microphone::{
    self, CaptureStatus, DeviceOption, MicrophonePermissionState, MicrophoneState, dbfs,
};
use crate::ui::waveform::{Waveform, WaveformMode, WaveformState};

pub struct MicrophonePage {
    waveform: Entity<WaveformState>,
    device_select: Entity<SelectState<Vec<DeviceOption>>>,
    gain_slider: Entity<SliderState>,
    device_ids: Vec<String>,
    _subscriptions: Vec<Subscription>,
}

impl MicrophonePage {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let snapshot = microphone::snapshot(cx);
        let waveform = snapshot.waveform.clone();
        let gain_slider = cx.new(|_| {
            SliderState::new()
                .min(-24.)
                .max(24.)
                .step(1.)
                .default_value(snapshot.gain_db)
        });
        let items = device_options(&snapshot);
        let selected_index = snapshot
            .selected_device
            .as_ref()
            .and_then(|id| items.iter().position(|item| &item.id == id))
            .map(IndexPath::new);
        let device_ids = items.iter().map(|item| item.id.clone()).collect();
        let device_select = cx.new(|cx| SelectState::new(items, selected_index, window, cx));

        cx.subscribe(&gain_slider, |_, _, event: &SliderEvent, cx| match event {
            SliderEvent::Change(value) => microphone::set_gain_db_live(value.start(), cx),
            SliderEvent::Release(value) => microphone::set_gain_db(value.start(), cx),
        })
        .detach();
        cx.subscribe(
            &device_select,
            |_, _, event: &SelectEvent<Vec<DeviceOption>>, cx| {
                let SelectEvent::Confirm(value) = event;
                microphone::select_device(value.clone(), cx);
            },
        )
        .detach();
        cx.observe(&waveform, |_, _, cx| cx.notify()).detach();
        cx.observe_global_in::<MicrophoneState>(window, |this, window, cx| {
            cx.notify();
            reconcile_devices(this, window, cx);
        })
        .detach();

        Self {
            waveform,
            device_select,
            gain_slider,
            device_ids,
            _subscriptions: Vec::new(),
        }
    }
}

fn device_options(snapshot: &microphone::MicrophoneSnapshot) -> Vec<DeviceOption> {
    snapshot
        .devices
        .iter()
        .map(|device| DeviceOption {
            id: device.id.clone(),
            name: device.name.clone(),
        })
        .collect()
}

fn reconcile_devices(
    this: &mut MicrophonePage,
    window: &mut Window,
    cx: &mut Context<MicrophonePage>,
) {
    let items = {
        let devices = microphone::devices(cx);
        let unchanged = devices.len() == this.device_ids.len()
            && devices
                .iter()
                .zip(this.device_ids.iter())
                .all(|(device, id)| &device.id == id);
        if unchanged {
            return;
        }
        devices
            .iter()
            .map(|device| DeviceOption {
                id: device.id.clone(),
                name: device.name.clone(),
            })
            .collect::<Vec<_>>()
    };
    this.device_ids = items.iter().map(|item| item.id.clone()).collect();
    this.device_select
        .update(cx, |state, cx| state.set_items(items, window, cx));
}

fn status_label(status: &CaptureStatus) -> String {
    match status {
        CaptureStatus::Idle => crate::i18n::localize("microphone_status_idle"),
        CaptureStatus::Starting => crate::i18n::localize("microphone_status_starting"),
        CaptureStatus::Capturing { .. } => crate::i18n::localize("microphone_status_capturing"),
        CaptureStatus::Error(_) => crate::i18n::localize("microphone_status_error"),
    }
}

/// `id` keys the row; the label is localized and would re-key the row on every
/// locale switch. The combined "label: value" text is the accessible label.
fn status_row(
    id: &'static str,
    label: impl Into<SharedString>,
    value: impl Into<SharedString>,
) -> impl IntoElement {
    let label = label.into();
    let value = value.into();
    let text = format!("{label}: {value}");
    div()
        .id(ElementId::Name(SharedString::from(format!(
            "microphone-status-{id}"
        ))))
        .a11y(Role::Paragraph, text)
        .flex()
        .items_center()
        .justify_between()
        .gap_4()
        .child(Label::new(label))
        .child(div().text_sm().child(value))
}

impl Render for MicrophonePage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Borrowed reads only: render runs at the waveform's ~30 Hz tick while
        // capturing; an owned snapshot clone per frame is pure heap churn.
        let busy = matches!(
            microphone::status(cx),
            CaptureStatus::Starting | CaptureStatus::Capturing { .. }
        );
        let demo_only = microphone::demo_only(cx);
        let can_start = demo_only
            || (*microphone::permission_state(cx) == MicrophonePermissionState::Authorized
                && !microphone::devices(cx).is_empty());
        let gain = microphone::gain_db(cx);
        let status_text = status_label(microphone::status(cx));
        let permission_text = microphone::permission_state(cx).label();
        let permission_guidance = permission_alert(microphone::permission_state(cx));
        let error_message = match microphone::status(cx) {
            CaptureStatus::Error(message) => Some(message.clone()),
            _ => None,
        };
        let (rms, peak) = microphone::meter(cx).read();
        let mode = self.waveform.read(cx).mode();
        let title = crate::i18n::localize("microphone_title");

        v_flex()
            .min_h_full()
            .p_6()
            .gap_4()
            .child(
                h_flex()
                    .w_full()
                    .flex_wrap()
                    .justify_between()
                    .items_center()
                    .child(
                        div()
                            .id("microphone-title")
                            .a11y(Role::Heading, title.clone())
                            .aria_level(1)
                            .text_xl()
                            .font_weight(FontWeight::BOLD)
                            .child(title),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .flex_shrink_0()
                            .child(
                                Button::new("mic-start")
                                    .outline()
                                    .label(crate::i18n::localize("microphone_start"))
                                    .icon(IconName::Mic)
                                    .disabled(!can_start || busy)
                                    .on_click(|_, _, cx| microphone::start(cx)),
                            )
                            .child(
                                Button::new("mic-stop")
                                    .outline()
                                    .label(crate::i18n::localize("microphone_stop"))
                                    .disabled(!busy)
                                    .on_click(|_, _, cx| microphone::stop(cx)),
                            ),
                    ),
            )
            .when_else(
                !demo_only,
                |this| {
                    this.child(
                        h_flex()
                            .gap_3()
                            .items_center()
                            .child(Label::new(crate::i18n::localize("microphone_device")))
                            .child(
                                Select::new(&self.device_select)
                                    .flex_1()
                                    .accessibility_label(crate::i18n::localize("microphone_device"))
                                    .placeholder(crate::i18n::localize("microphone_no_devices"))
                                    .disabled(busy),
                            )
                            .child(
                                Button::new("mic-refresh")
                                    .outline()
                                    .small()
                                    .label(crate::i18n::localize("microphone_refresh_devices"))
                                    .on_click(|_, _, cx| microphone::refresh_devices(cx)),
                            ),
                    )
                },
                |this| {
                    this.child(Alert::info(
                        "mic-demo-note",
                        crate::i18n::localize("microphone_demo_note"),
                    ))
                },
            )
            .child(
                h_flex()
                    .gap_3()
                    .items_center()
                    .child(Label::new(crate::i18n::localize("microphone_gain")))
                    .child(Slider::new(&self.gain_slider).flex_1())
                    .child(
                        div()
                            .w_12()
                            .text_sm()
                            .text_right()
                            .child(format!("{:+.0} dB", gain)),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(crate::i18n::localize("microphone_gain_note")),
            )
            .child(
                h_flex()
                    .gap_3()
                    .items_center()
                    .child(Label::new(crate::i18n::localize("microphone_level")))
                    .child(
                        Progress::new("mic-level")
                            .flex_1()
                            .value(peak * 100.)
                            .accessibility_label(crate::i18n::localize("microphone_level")),
                    )
                    .child(div().text_sm().child(format!(
                        "RMS {} · Peak {}",
                        dbfs(rms),
                        dbfs(peak)
                    ))),
            )
            .child(
                div()
                    .h_64()
                    .w_full()
                    .border_1()
                    .rounded_lg()
                    .overflow_hidden()
                    .child(Waveform::new("mic-waveform", self.waveform.clone())),
            )
            .child(
                h_flex().gap_2().items_center().child(
                    Switch::new("mic-spectrum")
                        .checked(matches!(mode, WaveformMode::Spectrum))
                        .label(crate::i18n::localize("microphone_spectrum"))
                        .on_click(cx.listener(|this, checked: &bool, _, cx| {
                            let mode = if *checked {
                                WaveformMode::Spectrum
                            } else {
                                WaveformMode::Oscilloscope
                            };
                            this.waveform.update(cx, |state, cx| {
                                state.set_mode(mode, cx);
                            });
                        })),
                ),
            )
            .child(status_row(
                "status",
                crate::i18n::localize("microphone_status"),
                status_text,
            ))
            .child(status_row(
                "permission",
                crate::i18n::localize("microphone_permission"),
                permission_text,
            ))
            .child(status_row(
                "dropped",
                crate::i18n::localize("microphone_dropped"),
                self.waveform.read(cx).dropped_samples().to_string(),
            ))
            .children(permission_guidance)
            .when_some(error_message, |this, message| {
                this.child(Alert::error("mic-error-alert", message))
            })
    }
}

fn permission_alert(permission: &MicrophonePermissionState) -> Option<impl IntoElement> {
    let request_button = || {
        Button::new("mic-request-permission")
            .outline()
            .label(crate::i18n::localize("microphone_request_permission"))
            .on_click(|_, _, cx| microphone::request_permission(cx))
    };
    match permission {
        MicrophonePermissionState::NotDetermined => Some(
            v_flex()
                .gap_2()
                .child(Alert::warning(
                    "mic-permission-alert",
                    crate::i18n::localize("microphone_permission_not_determined"),
                ))
                .child(request_button()),
        ),
        MicrophonePermissionState::Unavailable(reason) => Some(
            v_flex()
                .gap_2()
                .child(Alert::warning(
                    "mic-permission-alert",
                    format!(
                        "{} {}",
                        reason,
                        crate::i18n::localize("microphone_permission_bundle_hint")
                    ),
                ))
                .child(request_button()),
        ),
        MicrophonePermissionState::Denied | MicrophonePermissionState::Restricted => Some(
            v_flex()
                .gap_2()
                .child(Alert::warning(
                    "mic-permission-alert",
                    crate::i18n::localize("microphone_permission_denied_guidance"),
                ))
                .child(
                    Button::new("mic-open-settings")
                        .outline()
                        .label(crate::i18n::localize("microphone_open_settings"))
                        .on_click(|_, _, cx| microphone::open_microphone_settings(cx)),
                ),
        ),
        _ => None,
    }
}

#[cfg(test)]
#[path = "microphone.test.rs"]
mod microphone_test;
