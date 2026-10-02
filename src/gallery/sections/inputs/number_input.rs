//! Number Input section, ported from the upstream `NumberInputStory`.

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, IconName, Sizable as _,
    button::{Button, ButtonVariants as _},
    input::{InputEvent, InputState, MaskPattern, NumberInput, NumberInputEvent, StepAction},
    v_flex,
};
use gpui_kit::{
    App, AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, Styled as _,
    Subscription, Window, rems,
};
use regex::Regex;

use crate::gallery::registry::GallerySection;

use super::demo::section;

pub struct NumberInputSection {
    number_input1_value: i64,
    number_input1: Entity<InputState>,
    number_input2: Entity<InputState>,
    number_input3: Entity<InputState>,
    number_input4: Entity<InputState>,
    disabled_input: Entity<InputState>,

    _subscriptions: Vec<Subscription>,
}

impl NumberInputSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        // Opt out of internal stepping via `set_step(None)`, so the
        // NumberInput emits NumberInputEvent::Step and this section is
        // responsible for updating the value.
        let number_input1_value = 1;
        let number_input1 = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Normal Integer")
                .default_value(number_input1_value.to_string())
        });
        number_input1.update(cx, |state, cx| state.set_step(None, window, cx));

        // With min, the NumberInput steps the value internally (step
        // default: 1) and clamps it to the range, no event handling needed.
        let number_input2 = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Unsized Integer")
                .pattern(Regex::new(r"^\d+$").unwrap())
                .min(0.)
        });

        let number_input3 = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Mask pattern")
                .mask_pattern(MaskPattern::Number {
                    separator: Some(','),
                    fraction: Some(2),
                })
                .default_value("1234.56")
                .step(100.)
                .min(0.)
        });

        // The step varies by direction at the boundary 1.0: 0.1 going down,
        // 0.5 going up.
        let number_input4 = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Styling")
                .default_value("0.9")
                .step_by(|value, action, _cx| match action {
                    StepAction::Increment => {
                        if value < 1.0 {
                            0.1
                        } else {
                            0.5
                        }
                    }
                    StepAction::Decrement => {
                        if value <= 1.0 {
                            0.1
                        } else {
                            0.5
                        }
                    }
                })
                .min(0.)
        });

        let disabled_input = cx.new(|cx| {
            InputState::new(window, cx)
                .default_value("100")
                .placeholder("Disabled")
        });

        // Typing keeps the section's copy of the value in step with the
        // field, so the Step handler applies deltas from the edited value.
        let _subscriptions = vec![
            cx.subscribe_in(&number_input1, window, Self::on_input_event),
            cx.subscribe_in(&number_input1, window, Self::on_number_input_event),
        ];

        Self {
            number_input1,
            number_input1_value,
            number_input2,
            number_input3,
            number_input4,
            disabled_input,
            _subscriptions,
        }
    }

    fn on_input_event(
        &mut self,
        state: &Entity<InputState>,
        event: &InputEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if matches!(event, InputEvent::Change) {
            let text = state.read(cx).value();
            if state == &self.number_input1 {
                if let Ok(value) = text.parse::<i64>() {
                    self.number_input1_value = value;
                }
            }
        }
    }

    fn on_number_input_event(
        &mut self,
        this: &Entity<InputState>,
        event: &NumberInputEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match event {
            NumberInputEvent::Step(step_action) => {
                if this == &self.number_input1 {
                    match step_action {
                        StepAction::Decrement => {
                            self.number_input1_value -= 1;
                        }
                        StepAction::Increment => {
                            self.number_input1_value += 1;
                        }
                    }
                    this.update(cx, |input, cx| {
                        input.set_value(self.number_input1_value.to_string(), window, cx);
                    });
                }
            }
        }
    }
}

impl Render for NumberInputSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_6()
            .p_4()
            .child(
                section("number-input-default", "Default")
                    .description("Application-managed step events.")
                    .w_128()
                    .items_center()
                    .child(NumberInput::new(&self.number_input1).w(rems(16.25))),
            )
            .child(
                section("number-input-disabled", "Disabled")
                    .description("Read-only disabled state.")
                    .w_128()
                    .items_center()
                    .child(
                        NumberInput::new(&self.disabled_input)
                            .w(rems(16.25))
                            .disabled(true),
                    ),
            )
            .child(
                section("number-input-suffix", "Suffix")
                    .description("Small size with a suffix action.")
                    .w_128()
                    .items_center()
                    .child(
                        NumberInput::new(&self.number_input2)
                            .w(rems(16.25))
                            .small()
                            .suffix(
                                Button::new("number-input-info")
                                    .text()
                                    .icon(IconName::Info)
                                    .xsmall(),
                            ),
                    ),
            )
            .child(
                section("number-input-format", "Number format")
                    .description("Grouping, decimals, range, and step.")
                    .w_128()
                    .items_center()
                    .child(NumberInput::new(&self.number_input3).w(rems(16.25))),
            )
            .child(
                section("number-input-custom-style", "Custom style")
                    .description("Appearance-free input with dynamic steps.")
                    .w_128()
                    .items_center()
                    .child(
                        NumberInput::new(&self.number_input4)
                            .w(rems(16.25))
                            .appearance(false)
                            .bg(cx.theme().secondary)
                            .text_color(cx.theme().info),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "number-input",
        "Number Input",
        "Adjust constrained numeric values precisely with typing or increment and decrement controls.",
        NumberInputSection::view(window, cx),
    ));
}
