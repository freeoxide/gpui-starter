//! OTP Input section, ported from the upstream `OtpInputStory`.

use gpui_kit::component::{
    Disableable as _, Sizable as _, Size, StyledExt as _,
    input::{OtpEvent, OtpInput, OtpState},
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

use crate::gallery::registry::GallerySection;

use super::demo::{DemoToggle, demo_toolbar, options_dropdown, section, size_dropdown};

pub struct OtpInputSection {
    otp_masked: bool,
    otp_state: Entity<OtpState>,
    otp_value: Option<SharedString>,
    otp_state_small: Entity<OtpState>,
    otp_state_large: Entity<OtpState>,
    otp_state_sized: Entity<OtpState>,
    otp_state_disabled: Entity<OtpState>,
    size: Size,

    _subscriptions: Vec<Subscription>,
}

impl OtpInputSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let otp_state = cx.new(|cx| OtpState::new(6, window, cx).masked(true));

        let _subscriptions = vec![cx.subscribe(&otp_state, |this, state, ev: &OtpEvent, cx| {
            if matches!(ev, OtpEvent::Complete) {
                let text = state.read(cx).value();
                this.otp_value = Some(text.clone());
                cx.notify();
            }
        })];

        Self {
            otp_masked: true,
            otp_state,
            otp_value: None,
            otp_state_small: cx.new(|cx| {
                OtpState::new(6, window, cx)
                    .default_value("123456")
                    .masked(true)
            }),
            otp_state_large: cx.new(|cx| {
                OtpState::new(6, window, cx)
                    .default_value("012345")
                    .masked(true)
            }),
            otp_state_sized: cx.new(|cx| {
                OtpState::new(4, window, cx)
                    .masked(true)
                    .default_value("654321")
            }),
            otp_state_disabled: cx.new(|cx| {
                OtpState::new(6, window, cx)
                    .masked(true)
                    .default_value("123456")
            }),
            size: Size::Medium,
            _subscriptions,
        }
    }

    fn toggle_masked(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.otp_masked = !self.otp_masked;
        for state in [
            &self.otp_state,
            &self.otp_state_small,
            &self.otp_state_large,
            &self.otp_state_sized,
            &self.otp_state_disabled,
        ] {
            state.update(cx, |state, cx| {
                state.set_masked(self.otp_masked, window, cx)
            });
        }
    }
}

impl Render for OtpInputSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_6()
            .p_4()
            .on_action(cx.listener(|this, action: &DemoToggle, window, cx| {
                if let Some(size) = action.size() {
                    this.size = size;
                } else if matches!(action, DemoToggle::OtpMasked) {
                    this.toggle_masked(window, cx);
                } else {
                    return;
                }
                cx.notify();
            }))
            .child(demo_toolbar(vec![
                size_dropdown("otp-input-size", self.size).into_any_element(),
                options_dropdown(
                    "otp-input-options",
                    vec![("Masked", self.otp_masked, DemoToggle::OtpMasked)],
                )
                .into_any_element(),
            ]))
            .child(
                section("otp-input-default", "Default")
                    .description("Six cells with masking and value updates.")
                    .v_flex()
                    .child(OtpInput::new(&self.otp_state).with_size(self.size))
                    .when_some(self.otp_value.clone(), |this, otp| {
                        this.child(format!("Value: {}", otp))
                    }),
            )
            .child(
                section("otp-input-grouping", "Grouping")
                    .description("Cells can be shown as one or several groups.")
                    .v_flex()
                    .gap_4()
                    .child(
                        OtpInput::new(&self.otp_state_small)
                            .groups(1)
                            .with_size(self.size),
                    )
                    .child(
                        OtpInput::new(&self.otp_state_large)
                            .groups(3)
                            .with_size(self.size),
                    ),
            )
            .child(
                section("otp-input-custom-size", "Custom size")
                    .description("Custom cell dimensions.")
                    .child(
                        OtpInput::new(&self.otp_state_sized)
                            .groups(1)
                            .with_size(px(55.)),
                    ),
            )
            .child(
                section("otp-input-disabled", "Disabled")
                    .description("Disabled input with a value.")
                    .child(
                        OtpInput::new(&self.otp_state_disabled)
                            .with_size(self.size)
                            .disabled(true),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "otp-input",
        "OTP Input",
        "Enter short verification and recovery codes with clear grouping and masking controls.",
        OtpInputSection::view(window, cx),
    ));
}
