//! Slider section, ported from the upstream `SliderStory`.

use gpui_kit::component::{
    ActiveTheme as _, Colorize as _, StyledExt as _, WindowExt,
    button::{Button, DropdownButton},
    clipboard::Clipboard,
    h_flex,
    slider::{Slider, SliderEvent, SliderScale, SliderState, SliderValue},
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

use crate::gallery::registry::GallerySection;

use super::demo::{DemoToggle, demo_toolbar, section};

pub struct SliderSection {
    slider1: Entity<SliderState>,
    slider1_released_value: f32,
    slider3: Entity<SliderState>,
    slider3_released_value: SliderValue,
    slider_hsl: [Entity<SliderState>; 4],
    slider_logarithmic: Entity<SliderState>,
    slider_reverse: Entity<SliderState>,
    slider_duration: Entity<SliderState>,
    slider_temperature: Entity<SliderState>,
    disabled: bool,
    _subscriptions: Vec<Subscription>,
}

impl SliderSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(_: &mut Window, cx: &mut Context<Self>) -> Self {
        let slider1 = cx.new(|_| {
            SliderState::new()
                .min(-255.)
                .max(255.)
                .default_value(75.)
                .step(15.)
        });

        let slider_hsl = [
            cx.new(|_| {
                SliderState::new()
                    .min(0.)
                    .max(1.)
                    .step(0.01)
                    .default_value(0.38)
            }),
            cx.new(|_| {
                SliderState::new()
                    .min(0.)
                    .max(1.)
                    .step(0.01)
                    .default_value(0.5)
            }),
            cx.new(|_| {
                SliderState::new()
                    .min(0.)
                    .max(1.)
                    .step(0.01)
                    .default_value(0.5)
            }),
            cx.new(|_| {
                SliderState::new()
                    .min(0.)
                    .max(1.)
                    .step(0.01)
                    .default_value(0.5)
            }),
        ];

        let slider3 = cx.new(|_| {
            SliderState::new()
                .min(0.)
                .max(100.)
                .default_value(12.0..45.0)
                .step(1.)
        });

        let slider_logarithmic = cx.new(|_| {
            SliderState::new()
                .min(0.25)
                .max(4.0)
                .default_value(1.0)
                .step(0.05)
                .scale(SliderScale::Logarithmic)
        });

        let slider_reverse = cx.new(|_| {
            SliderState::new()
                .min(0.)
                .max(10.)
                .step(1.)
                .default_value(5.)
        });

        let slider_duration = cx.new(|_| {
            SliderState::new()
                .min(0.)
                .max(12.)
                .step(1.)
                .default_value(5.)
        });

        let slider_temperature = cx.new(|_| {
            SliderState::new()
                .max(6500.)
                .min(2000.)
                .step(100.)
                .default_value(3600.)
        });

        // Only the release latches stay cached: a released-at readout cannot
        // be derived from the live state. Everything else reads its entity.
        let _subscriptions = vec![
            cx.subscribe(&slider1, |this, _, event: &SliderEvent, cx| {
                if let SliderEvent::Release(value) = event {
                    this.slider1_released_value = value.start();
                    cx.notify();
                }
            }),
            cx.subscribe(&slider3, |this, _, event: &SliderEvent, cx| {
                if let SliderEvent::Release(value) = event {
                    this.slider3_released_value = *value;
                    cx.notify();
                }
            }),
        ];

        Self {
            slider1_released_value: 75.,
            slider1,
            slider3_released_value: (12.0, 45.0).into(),
            slider3,
            slider_hsl,
            slider_logarithmic,
            slider_reverse,
            slider_duration,
            slider_temperature,
            disabled: false,
            _subscriptions,
        }
    }
}

impl Render for SliderSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let slider1_value = zero_positive(self.slider1.read(cx).value().start());
        let slider_hsl_value = hsla(
            self.slider_hsl[0].read(cx).value().start(),
            self.slider_hsl[1].read(cx).value().start(),
            self.slider_hsl[2].read(cx).value().start(),
            self.slider_hsl[3].read(cx).value().start(),
        );
        let rgb = SharedString::from(slider_hsl_value.to_hex());
        let neutral_temperature = if cx.theme().mode.is_dark() {
            cx.theme().foreground
        } else {
            cx.theme().background
        };
        let warm_temperature = neutral_temperature.blend(cx.theme().warning.opacity(0.85));
        let cool_temperature = neutral_temperature.blend(cx.theme().info.opacity(0.65));
        let temperature_radius = cx.theme().radius_full();
        let disabled = self.disabled;

        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .p_4()
            .on_action(cx.listener(|this, action: &DemoToggle, _, cx| {
                if matches!(action, DemoToggle::Disabled) {
                    this.disabled = !this.disabled;
                    cx.notify();
                }
            }))
            .child(demo_toolbar(vec![
                DropdownButton::new("slider-options")
                    .button(Button::new("slider-options-trigger").label("Options"))
                    .dropdown_menu(move |menu, _, _| {
                        menu.menu_with_check("Disabled", disabled, Box::new(DemoToggle::Disabled))
                    })
                    .into_any_element(),
            ]))
            .child(
                section("slider-default", "Default")
                    .description("Adjust a single value within a defined range.")
                    .w_128()
                    .items_center()
                    .child(
                        v_flex()
                            .w(rems(22.5))
                            .gap_4()
                            .p_4()
                            .rounded(cx.theme().radius_lg)
                            .border_1()
                            .border_color(cx.theme().border)
                            .child(
                                h_flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .font_weight(FontWeight::MEDIUM)
                                            .child("Output volume"),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(format!("{slider1_value}")),
                                    ),
                            )
                            .child(
                                Slider::new(&self.slider1)
                                    .disabled(self.disabled)
                                    .accessibility_label("Output volume"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!(
                                        "Released at {}",
                                        zero_positive(self.slider1_released_value)
                                    )),
                            ),
                    ),
            )
            .child(
                section("slider-range", "Range")
                    .description("Choose minimum and maximum values together.")
                    .w_128()
                    .items_center()
                    .child(
                        v_flex()
                            .w(rems(22.5))
                            .gap_4()
                            .p_4()
                            .rounded(cx.theme().radius_lg)
                            .bg(cx.theme().muted.opacity(0.4))
                            .child(
                                h_flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div().font_weight(FontWeight::MEDIUM).child("Price range"),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(format!("${}", self.slider3.read(cx).value())),
                                    ),
                            )
                            .child(
                                Slider::new(&self.slider3)
                                    .disabled(self.disabled)
                                    .accessibility_label("Price range"),
                            )
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("Released at ${}", self.slider3_released_value)),
                            ),
                    ),
            )
            .child(
                section("slider-reverse", "Reverse")
                    .description("Reverse the fill direction for remaining capacity.")
                    .w_128()
                    .items_center()
                    .child(
                        v_flex()
                            .w(rems(22.5))
                            .gap_4()
                            .child(
                                h_flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .font_weight(FontWeight::MEDIUM)
                                            .child("Storage remaining"),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(format!(
                                                "{} GB",
                                                10. - self.slider_reverse.read(cx).value().start()
                                            )),
                                    ),
                            )
                            .child(
                                Slider::new(&self.slider_reverse)
                                    .horizontal()
                                    .reverse()
                                    .disabled(self.disabled)
                                    .accessibility_label("Storage remaining"),
                            ),
                    ),
            )
            .child(
                section("slider-duration", "Duration")
                    .description("Compose a slider with a labeled tick scale.")
                    .w_128()
                    .items_center()
                    .child(
                        v_flex()
                            .w(rems(22.5))
                            .gap_2()
                            .child(
                                h_flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .font_weight(FontWeight::MEDIUM)
                                            .child("Duration (months)"),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(format!(
                                                "{:.0}",
                                                self.slider_duration.read(cx).value().start()
                                            )),
                                    ),
                            )
                            .child(
                                v_flex()
                                    .px_4()
                                    .child(
                                        Slider::new(&self.slider_duration)
                                            .disabled(self.disabled)
                                            .accessibility_label("Duration (months)"),
                                    )
                                    .child(div().relative().w_full().h(rems(2.125)).children(
                                        (0..=12).map(|month| {
                                            let major = month % 2 == 0;
                                            v_flex()
                                                .absolute()
                                                .left(relative(month as f32 / 12.))
                                                .ml(-rems(1.))
                                                .w(rems(2.))
                                                .items_center()
                                                .child(
                                                    // One-device-pixel tick hairline.
                                                    div()
                                                        .w(px(1.))
                                                        .h(if major {
                                                            rems(0.375)
                                                        } else {
                                                            rems(0.1875)
                                                        })
                                                        .bg(cx.theme().muted_foreground),
                                                )
                                                .when(major, |this| {
                                                    this.child(
                                                        div()
                                                            .mt_2()
                                                            .text_sm()
                                                            .text_color(cx.theme().muted_foreground)
                                                            .child(month.to_string()),
                                                    )
                                                })
                                        }),
                                    )),
                            ),
                    ),
            )
            .child(
                section("slider-color-temperature", "Color temperature")
                    .description("Place a color scale beside a regular slider.")
                    .w_128()
                    .items_center()
                    .child(
                        v_flex()
                            .w(rems(22.5))
                            .gap_3()
                            .child(
                                h_flex()
                                    .items_center()
                                    .justify_between()
                                    .child(
                                        div()
                                            .font_weight(FontWeight::MEDIUM)
                                            .child("Color temperature"),
                                    )
                                    .child(
                                        div()
                                            .text_sm()
                                            .text_color(cx.theme().muted_foreground)
                                            .child(format!(
                                                "{:.0} K",
                                                self.slider_temperature.read(cx).value().start()
                                            )),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .w_full()
                                    .h_3()
                                    .child(
                                        div()
                                            .flex_1()
                                            .h_full()
                                            .corner_radii(Corners {
                                                top_left: temperature_radius,
                                                top_right: px(0.),
                                                bottom_right: px(0.),
                                                bottom_left: temperature_radius,
                                            })
                                            .bg(linear_gradient(
                                                90.,
                                                linear_color_stop(warm_temperature, 0.),
                                                linear_color_stop(neutral_temperature, 1.),
                                            )),
                                    )
                                    .child(
                                        div()
                                            .flex_1()
                                            .h_full()
                                            .corner_radii(Corners {
                                                top_left: px(0.),
                                                top_right: temperature_radius,
                                                bottom_right: temperature_radius,
                                                bottom_left: px(0.),
                                            })
                                            .bg(linear_gradient(
                                                90.,
                                                linear_color_stop(neutral_temperature, 0.),
                                                linear_color_stop(cool_temperature, 1.),
                                            )),
                                    ),
                            )
                            .child(
                                Slider::new(&self.slider_temperature)
                                    .disabled(self.disabled)
                                    .accessibility_label("Color temperature"),
                            ),
                    ),
            )
            .child(
                section("slider-color-picker", "Color Picker")
                    .sub_title(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(h_flex().text_color(slider_hsl_value).child(rgb.clone()))
                            .child(Clipboard::new("slider-copy-hsl").value(rgb).on_copied(
                                |_, window, cx| {
                                    window.push_notification("Color copied to clipboard.", cx)
                                },
                            )),
                    )
                    .w_128()
                    .items_center()
                    .justify_around()
                    .child(
                        v_flex()
                            .h_32()
                            .gap_3()
                            .items_center()
                            .justify_center()
                            .child(
                                Slider::new(&self.slider_hsl[0])
                                    .vertical()
                                    .disabled(self.disabled)
                                    .accessibility_label("Hue"),
                            )
                            .child(
                                v_flex()
                                    .items_center()
                                    .child("Hue")
                                    .child(format!("{:.0}", slider_hsl_value.h * 360.)),
                            ),
                    )
                    .child(
                        v_flex()
                            .h_32()
                            .gap_3()
                            .items_center()
                            .justify_center()
                            .child(
                                Slider::new(&self.slider_hsl[1])
                                    .vertical()
                                    .disabled(self.disabled)
                                    .accessibility_label("Saturation"),
                            )
                            .child(
                                v_flex()
                                    .items_center()
                                    .child("Saturation")
                                    .child(format!("{:.0}", slider_hsl_value.s * 100.)),
                            ),
                    )
                    .child(
                        v_flex()
                            .h_32()
                            .gap_3()
                            .items_center()
                            .justify_center()
                            .child(
                                Slider::new(&self.slider_hsl[2])
                                    .vertical()
                                    .disabled(self.disabled)
                                    .accessibility_label("Lightness"),
                            )
                            .child(
                                v_flex()
                                    .items_center()
                                    .child("Lightness")
                                    .child(format!("{:.0}", slider_hsl_value.l * 100.)),
                            ),
                    )
                    .child(
                        v_flex()
                            .h_32()
                            .gap_3()
                            .items_center()
                            .justify_center()
                            .child(
                                Slider::new(&self.slider_hsl[3])
                                    .vertical()
                                    .disabled(self.disabled)
                                    .accessibility_label("Alpha"),
                            )
                            .child(
                                v_flex()
                                    .items_center()
                                    .child("Alpha")
                                    .child(format!("{:.0}", slider_hsl_value.a * 100.)),
                            ),
                    ),
            )
            .child(
                section("slider-playback-speed", "Playback speed")
                    .description("Logarithmic scales provide finer control near common values.")
                    .w_128()
                    .items_center()
                    .child(
                        v_flex()
                            .w(rems(22.5))
                            .gap_4()
                            .child(
                                h_flex()
                                    .items_center()
                                    .justify_between()
                                    .child(div().font_weight(FontWeight::MEDIUM).child("Speed"))
                                    .child(format!(
                                        "{:.2}×",
                                        self.slider_logarithmic.read(cx).value().start()
                                    )),
                            )
                            .child(
                                Slider::new(&self.slider_logarithmic)
                                    .horizontal()
                                    .disabled(self.disabled)
                                    .accessibility_label("Speed"),
                            ),
                    ),
            )
    }
}

/// Step snapping just below zero yields IEEE negative zero, whose Display is
/// "-0"; fold it to +0 for the readouts.
fn zero_positive(value: f32) -> f32 {
    if value == 0. { 0. } else { value }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "slider",
        "Slider",
        "Displays a slider control for selecting a value within a range.",
        SliderSection::view(window, cx),
    ));
}
