//! Input section, ported from the upstream `InputStory` with the standalone
//! `input` example folded in as its opening subsection.

use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, Size, StyledExt as _, WindowExt as _,
    button::{Button, ButtonVariants as _},
    h_flex,
    input::{Copy, Input, InputContentType, InputEvent, InputState, MaskPattern, Paste, SelectAll},
    label::Label,
    v_flex,
};
use gpui_kit::*;
use regex::Regex;

use crate::gallery::registry::GallerySection;

use super::demo::{DemoToggle, demo_toolbar, section, size_dropdown};
use super::tokens::TokenComposer;

const DEMO_TEXT: &str = "Hello 世界，this is GPUI Kit, this is a long text.";

/// One row of the content-type demo: a fixed label lane and the input.
struct ContentTypeInput {
    label: &'static str,
    content_type: InputContentType,
    input: Entity<InputState>,
    mask_toggle: bool,
}

pub struct InputSection {
    size: Size,
    greeting: SharedString,
    name_input: Entity<InputState>,
    input1: Entity<InputState>,
    input2: Entity<InputState>,
    input_esc: Entity<InputState>,
    input_text_centered: Entity<InputState>,
    input_text_right: Entity<InputState>,
    mask_input: Entity<InputState>,
    disabled_input: Entity<InputState>,
    readonly_input: Entity<InputState>,
    prefix_input1: Entity<InputState>,
    suffix_input1: Entity<InputState>,
    both_input1: Entity<InputState>,
    complete_input: Entity<InputState>,
    complete_disabled_input: Entity<InputState>,
    validation_input: Entity<InputState>,
    phone_input: Entity<InputState>,
    mask_input2: Entity<InputState>,
    currency_input: Entity<InputState>,
    custom_input: Entity<InputState>,
    custom_menu_input: Entity<InputState>,
    color_input: Entity<InputState>,
    content_type_inputs: Vec<ContentTypeInput>,
    tokens: Entity<TokenComposer>,
    _subscriptions: Vec<Subscription>,
}

impl InputSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let name_input = cx.new(|cx| InputState::new(window, cx).placeholder("Enter your name"));

        let input1 = cx.new(|cx| InputState::new(window, cx).default_value(DEMO_TEXT));
        let input2 = cx.new(|cx| InputState::new(window, cx).placeholder("Enter text here…"));
        let input_esc = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Enter text and clear it by pressing ESC")
                .clean_on_escape()
        });

        let mask_input = cx.new(|cx| {
            InputState::new(window, cx)
                .masked(true)
                .placeholder("Enter your password…")
                .default_value("this-is-password-中文🚀🎉")
        });

        let prefix_input1 =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search some thing…"));
        let suffix_input1 = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("This input only support [a-zA-Z0-9] characters.")
                .pattern(Regex::new(r"^[a-zA-Z0-9]*$").unwrap())
        });
        let both_input1 = cx.new(|cx| {
            InputState::new(window, cx).placeholder("This input have prefix and suffix.")
        });
        let complete_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Search account…")
                .default_value("jane.doe@example.com")
        });
        let complete_disabled_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Search account…")
                .default_value("disabled.account@example.com")
        });

        let phone_input = cx.new(|cx| InputState::new(window, cx).mask_pattern("(999)-999-9999"));
        let mask_input2 = cx.new(|cx| InputState::new(window, cx).mask_pattern("AAA-###-AAA"));
        let currency_input = cx.new(|cx| {
            InputState::new(window, cx).mask_pattern(MaskPattern::Number {
                separator: Some(','),
                fraction: Some(3),
            })
        });
        let custom_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Custom Input use monospace, 0123456789.")
                .context_menu(false)
        });

        let custom_menu_input =
            cx.new(|cx| InputState::new(window, cx).placeholder("Input with custom context menu…"));

        let color_input = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Type something…")
                .default_value("Custom text color input")
        });

        let input_text_centered = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Enter text to test center layout…")
                .default_value("Centered Text")
        });

        let input_text_right = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Enter text to test right layout…")
                .default_value("Right Aligned Text")
        });

        let content_type_inputs = vec![
            Self::new_content_type_input(
                window,
                cx,
                "Name",
                InputContentType::Name,
                "Full name",
                "Jane Doe",
                false,
            ),
            Self::new_content_type_input(
                window,
                cx,
                "Username",
                InputContentType::Username,
                "Username",
                "jane.doe",
                false,
            ),
            Self::new_content_type_input(
                window,
                cx,
                "Password",
                InputContentType::Password,
                "Current password",
                "current-password",
                true,
            ),
            Self::new_content_type_input(
                window,
                cx,
                "New password",
                InputContentType::NewPassword,
                "New password",
                "new-password",
                true,
            ),
            Self::new_content_type_input(
                window,
                cx,
                "One-time code",
                InputContentType::OneTimeCode,
                "123456",
                "123456",
                false,
            ),
            Self::new_content_type_input(
                window,
                cx,
                "Email",
                InputContentType::EmailAddress,
                "Email address",
                "jane.doe@example.com",
                false,
            ),
            Self::new_content_type_input(
                window,
                cx,
                "Telephone",
                InputContentType::TelephoneNumber,
                "Telephone number",
                "+1 415 555 0198",
                false,
            ),
            Self::new_content_type_input(
                window,
                cx,
                "URL",
                InputContentType::Url,
                "Website URL",
                "https://example.com",
                false,
            ),
            Self::new_content_type_input(
                window,
                cx,
                "Credit card number",
                InputContentType::CreditCardNumber,
                "Card number",
                "4242 4242 4242 4242",
                false,
            ),
            Self::new_content_type_input(
                window,
                cx,
                "Credit card expiration",
                InputContentType::CreditCardExpiration,
                "MM/YY",
                "12/28",
                false,
            ),
            Self::new_content_type_input(
                window,
                cx,
                "Credit card security code",
                InputContentType::CreditCardSecurityCode,
                "CVC",
                "123",
                true,
            ),
            Self::new_content_type_input(
                window,
                cx,
                "Postal code",
                InputContentType::PostalCode,
                "Postal code",
                "94107",
                false,
            ),
        ];

        // Typing in the plain field drives the disabled field, the story's
        // one visible demonstration of `set_value`.
        let subscriptions = vec![
            cx.subscribe_in(&name_input, window, |this, state, event, _, cx| {
                if matches!(event, InputEvent::Change) {
                    this.greeting = format!("Hello, {}!", state.read(cx).value()).into();
                    cx.notify();
                }
            }),
            cx.subscribe_in(&input2, window, |this, state, event, window, cx| {
                if matches!(event, InputEvent::Change) {
                    let text = state.read(cx).value();
                    this.disabled_input.update(cx, |this, cx| {
                        this.set_value(text, window, cx);
                    });
                }
            }),
        ];

        Self {
            size: Size::Medium,
            greeting: SharedString::default(),
            name_input,
            input1,
            input2,
            input_esc,
            input_text_centered,
            input_text_right,
            mask_input,
            disabled_input: cx
                .new(|cx| InputState::new(window, cx).default_value("This is disabled input")),
            readonly_input: cx
                .new(|cx| InputState::new(window, cx).default_value("This is read-only input")),
            prefix_input1,
            suffix_input1,
            both_input1,
            complete_input,
            complete_disabled_input,
            validation_input: cx.new(|cx| {
                InputState::new(window, cx)
                    .validate(|s, _| s.parse::<f32>().is_ok())
                    .placeholder("validate to limit float number.")
            }),
            phone_input,
            mask_input2,
            currency_input,
            custom_input,
            custom_menu_input,
            color_input,
            content_type_inputs,
            tokens: TokenComposer::new("input", false, window, cx),
            _subscriptions: subscriptions,
        }
    }

    fn new_content_type_input(
        window: &mut Window,
        cx: &mut Context<Self>,
        label: &'static str,
        content_type: InputContentType,
        placeholder: &'static str,
        default_value: &'static str,
        masked: bool,
    ) -> ContentTypeInput {
        let input = cx.new(|cx| {
            let state = InputState::new(window, cx)
                .placeholder(placeholder)
                .default_value(default_value);

            if masked { state.masked(true) } else { state }
        });

        ContentTypeInput {
            label,
            content_type,
            input,
            mask_toggle: masked,
        }
    }

    fn render_content_type_input(item: &ContentTypeInput) -> impl IntoElement {
        let input = Input::new(&item.input)
            .content_type(item.content_type)
            .flex_1();
        let input = if item.mask_toggle {
            input.mask_toggle()
        } else {
            input
        };

        h_flex()
            .w_full()
            .items_center()
            .gap_3()
            .child(
                Label::new(item.label)
                    .w_48()
                    .flex_shrink_0()
                    .text_sm()
                    .whitespace_nowrap(),
            )
            .child(input)
    }
}

impl Render for InputSection {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let size = self.size;

        v_flex()
            .w_full()
            .gap_6()
            .p_4()
            .on_action(cx.listener(|this, action: &DemoToggle, _, cx| {
                if let Some(size) = action.size() {
                    this.size = size;
                    cx.notify();
                }
            }))
            .child(demo_toolbar(vec![
                size_dropdown("input-size", size).into_any_element(),
            ]))
            .child(
                section("input-hello", "Hello, name")
                    .description("A retained input state and its change event.")
                    .w_128()
                    .v_flex()
                    .child(Input::new(&self.name_input).with_size(size))
                    .child(div().text_sm().child(self.greeting.clone())),
            )
            .child(
                section("input-default", "Default")
                    .description("Text, email, and clearable inputs.")
                    .w_128()
                    .v_flex()
                    .child(
                        Input::new(&self.input1)
                            .with_size(size)
                            .cleanable(true),
                    )
                    .child(
                        Input::new(&self.input2)
                            .with_size(size)
                            .role(Role::EmailInput),
                    ),
            )
            .child(
                section("input-states", "States")
                    .description("Disabled, read-only and revealable password inputs.")
                    .w_128()
                    .v_flex()
                    .child(
                        Input::new(&self.disabled_input)
                            .with_size(size)
                            .disabled(true),
                    )
                    .child(
                        Input::new(&self.readonly_input)
                            .with_size(size)
                            .readonly(true),
                    )
                    .child(
                        Input::new(&self.mask_input)
                            .with_size(size)
                            .content_type(InputContentType::Password)
                            .mask_toggle()
                            .cleanable(true),
                    ),
            )
            .child(
                section("input-content-type", "Content type")
                    .description("Content types adapt input behavior.")
                    .w_128()
                    .v_flex()
                    .children(
                        self.content_type_inputs
                            .iter()
                            .map(Self::render_content_type_input),
                    ),
            )
            .child(
                section("input-alignment", "Alignment")
                    .description("Align text to the center or end.")
                    .w_128()
                    .child(
                        h_flex()
                            .w_full()
                            .gap_4()
                            .flex_wrap()
                            .child(
                                Input::new(&self.input_text_centered)
                                    .with_size(size)
                                    .text_center()
                                    .flex_1(),
                            )
                            .child(
                                Input::new(&self.input_text_right)
                                    .with_size(size)
                                    .text_right()
                                    .flex_1(),
                            ),
                    ),
            )
            .child(
                section("input-prefix-suffix", "Prefix and suffix")
                    .description("Add icons or actions inside the field.")
                    .w_128()
                    .v_flex()
                    .child(
                        Input::new(&self.prefix_input1)
                            .with_size(size)
                            .cleanable(true)
                            .prefix(Icon::new(IconName::Search).small()),
                    )
                    .child(
                        Input::new(&self.both_input1)
                            .with_size(size)
                            .cleanable(true)
                            .prefix(div().child(Icon::new(IconName::Search).small()))
                            .suffix(
                                Button::new("input-both-info")
                                    .text()
                                    .icon(IconName::Info)
                                    .xsmall(),
                            ),
                    )
                    .child(
                        Input::new(&self.suffix_input1)
                            .with_size(size)
                            .cleanable(true)
                            .suffix(
                                Button::new("input-suffix-info")
                                    .text()
                                    .icon(IconName::Info)
                                    .xsmall(),
                            ),
                    ),
            )
            .child(
                section("input-composed", "Composed states")
                    .description("Composed inputs support disabled state.")
                    .w_128()
                    .v_flex()
                    .child(
                        Input::new(&self.complete_input)
                            .with_size(size)
                            .cleanable(true)
                            .prefix(Icon::new(IconName::Search).small())
                            .suffix(
                                Button::new("input-complete-info")
                                    .text()
                                    .icon(IconName::Info)
                                    .xsmall(),
                            ),
                    )
                    .child(
                        Input::new(&self.complete_disabled_input)
                            .with_size(size)
                            .cleanable(true)
                            .disabled(true)
                            .prefix(Icon::new(IconName::Search).small())
                            .suffix(
                                Button::new("input-complete-disabled-info")
                                    .text()
                                    .icon(IconName::Info)
                                    .xsmall(),
                            ),
                    ),
            )
            .child(
                section("input-currency", "Currency")
                    .description("Format currency while retaining its value.")
                    .w_128()
                    .v_flex()
                    .child(Input::new(&self.currency_input).with_size(size))
                    .child(div().child(format!(
                        "Value: {:?}",
                        self.currency_input.read(cx).value()
                    ))),
            )
            .child(
                section("input-phone-mask", "Phone mask")
                    .description("Expose formatted and raw phone values.")
                    .w_128()
                    .v_flex()
                    .child(Input::new(&self.phone_input).with_size(size))
                    .child(
                        v_flex()
                            .child(format!(
                                "Value: {:?}",
                                self.phone_input.read(cx).value()
                            ))
                            .child(format!(
                                "Unmask Value: {:?}",
                                self.phone_input.read(cx).unmask_value()
                            )),
                    ),
            )
            .child(
                section("input-mask-pattern", "Mask pattern")
                    .description("Combine letter and number placeholders.")
                    .w_128()
                    .v_flex()
                    .child(Input::new(&self.mask_input2).with_size(size))
                    .child(
                        v_flex()
                            .child(format!(
                                "Value: {:?}",
                                self.mask_input2.read(cx).value()
                            ))
                            .child(format!(
                                "Unmask Value: {:?}",
                                self.mask_input2.read(cx).unmask_value()
                            )),
                    ),
            )
            .child(
                section("input-validation", "Validation")
                    .description("Validate values while the user types.")
                    .w_128()
                    .v_flex()
                    .child(Input::new(&self.validation_input).with_size(size)),
            )
            .child(
                section("input-clear-on-escape", "Clear on Escape")
                    .description("Clear a value with its action or Escape.")
                    .w_128()
                    .v_flex()
                    .child(
                        Input::new(&self.input_esc)
                            .with_size(size)
                            .cleanable(true),
                    ),
            )
            .child(
                section("input-focused-value", "Focused value")
                    .description("Read the value of the focused input.")
                    .w_128()
                    .whitespace_normal()
                    .overflow_hidden()
                    .child(div().child(format!(
                        "Value: {:?}",
                        window.focused_input(cx).map(|input| input.value(cx))
                    ))),
            )
            .child(
                section("input-custom-appearance", "Custom appearance")
                    .description("Remove the default field appearance.")
                    .w_128()
                    .child(
                        div()
                            .border_b_2()
                            .px_6()
                            .py_3()
                            .font_family(cx.theme().mono_font_family.clone())
                            .border_color(cx.theme().border)
                            .bg(cx.theme().secondary)
                            .text_color(cx.theme().secondary_foreground)
                            .w_full()
                            .child(
                                Input::new(&self.custom_input)
                                    .with_size(size)
                                    .appearance(false),
                            ),
                    ),
            )
            .child(
                section("input-context-menu", "Context menu")
                    .description("Add actions to the editing menu.")
                    .w_128()
                    .v_flex()
                    .child(
                        Input::new(&self.custom_menu_input).with_size(size).context_menu(
                            |menu, _, _| {
                                menu.menu("Custom Action", Box::new(SelectAll))
                                    .separator()
                                    .menu("Copy", Box::new(Copy))
                                    .menu("Paste", Box::new(Paste))
                            },
                        ),
                    ),
            )
            .child(
                section("input-text-color", "Text color")
                    .description("Apply a semantic text color.")
                    .w_128()
                    .v_flex()
                    .child(
                        Input::new(&self.color_input)
                            .with_size(size)
                            .text_color(cx.theme().info),
                    ),
            )
            .child(
                section("input-tokens", "Atomic inline tokens")
                    .description(
                        "References keep their identity through selection, deletion and undo. Copy returns the underlying text.",
                    )
                    .w_full()
                    .child(self.tokens.clone()),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "input",
        "Input",
        "Capture and validate short-form text, credentials, identifiers, and formatted values.",
        InputSection::view(window, cx),
    ));
}
