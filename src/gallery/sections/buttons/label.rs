//! Label section, ported from the upstream `LabelStory`.

use gpui_kit::component::{
    ActiveTheme as _, IconName,
    button::{Button, ButtonVariants as _, DropdownButton, Toggle},
    h_flex,
    input::{Input, InputEvent, InputState},
    label::{HighlightsMatch, Label},
    v_flex,
};
use gpui_kit::*;
use serde::Deserialize;

use crate::gallery::registry::GallerySection;

use super::demo::{demo_toolbar, section};

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_label_section, no_json)]
struct TogglePrefix;

pub struct LabelSection {
    masked: bool,
    highlights_text: SharedString,
    highlights_input: Entity<InputState>,
    prefix: bool,
    _subscriptions: Vec<Subscription>,
}

impl LabelSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| {
            let highlights_input = cx.new(|cx| {
                InputState::new(window, cx)
                    .placeholder("Search labels")
                    .clean_on_escape()
            });

            let _subscriptions = vec![cx.subscribe(
                &highlights_input,
                |this: &mut LabelSection, _, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.highlights_text = this.highlights_input.read(cx).value();
                        cx.notify();
                    }
                },
            )];

            Self {
                masked: false,
                highlights_text: Default::default(),
                highlights_input,
                prefix: false,
                _subscriptions,
            }
        })
    }

    fn highlights(&self) -> HighlightsMatch {
        if self.prefix {
            HighlightsMatch::Prefix(self.highlights_text.clone())
        } else {
            HighlightsMatch::Full(self.highlights_text.clone())
        }
    }
}

impl Render for LabelSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let highlights = self.highlights();
        let masked = self.masked;
        let prefix = self.prefix;

        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .p_4()
            .on_action(cx.listener(|this, _: &TogglePrefix, _, cx| {
                this.prefix = !this.prefix;
                cx.notify();
            }))
            .child(demo_toolbar(vec![
                DropdownButton::new("label-options")
                    .button(Button::new("label-options-trigger").label("Options"))
                    .dropdown_menu(move |menu, _, _| {
                        menu.menu_with_check("Prefix Match", prefix, Box::new(TogglePrefix))
                    })
                    .into_any_element(),
            ]))
            .child(
                section("label-default", "Default")
                    .description("Present primary text with optional supporting context.")
                    .w(rems(35.))
                    .items_center()
                    .child(
                        v_flex()
                            .w(rems(20.))
                            .gap_4()
                            .child(Label::new("Account details"))
                            .child(Label::new("Company address").secondary("Optional"))
                            .child(
                                Label::new("Workspace owner")
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .secondary("Administrator"),
                            ),
                    ),
            )
            .child(
                section("label-highlighting", "Highlighting")
                    .description("Find matching text across Latin and CJK content.")
                    .w(rems(35.))
                    .items_center()
                    .child(
                        v_flex()
                            .w(rems(20.))
                            .gap_4()
                            .child(
                                h_flex()
                                    .w_full()
                                    .gap_2()
                                    .child(div().flex_1().child(Input::new(&self.highlights_input)))
                                    .child(
                                        Toggle::new("label-highlight-mask")
                                            .label("Mask")
                                            .checked(masked)
                                            .on_click(cx.listener(|this, checked, _, cx| {
                                                this.masked = *checked;
                                                cx.notify();
                                            })),
                                    ),
                            )
                            .child(
                                v_flex()
                                    .w_full()
                                    .gap_3()
                                    .p_4()
                                    .rounded(cx.theme().radius_lg)
                                    .border_1()
                                    .border_color(cx.theme().border)
                                    .child(
                                        Label::new("Design system documentation")
                                            .highlights(highlights.clone())
                                            .masked(masked),
                                    )
                                    // Keeps the mixed ASCII/CJK matching visible.
                                    .child(
                                        Label::new("AAA中文BB")
                                            .highlights(highlights)
                                            .masked(masked),
                                    ),
                            ),
                    ),
            )
            .child(
                section("label-layout", "Layout")
                    .description("Labels support alignment and natural wrapping.")
                    .w(rems(35.))
                    .items_center()
                    .child(
                        v_flex()
                            .w(rems(20.))
                            .gap_4()
                            .child(
                                v_flex()
                                    .w_full()
                                    .gap_2()
                                    .p_4()
                                    .rounded(cx.theme().radius_lg)
                                    .bg(cx.theme().muted.opacity(0.4))
                                    .child(Label::new("Start aligned"))
                                    .child(Label::new("Center aligned").text_center())
                                    .child(Label::new("End aligned").text_right()),
                            )
                            .child(
                                div().w_56().child(
                                    Label::new(
                                        "Long labels wrap cleanly inside constrained layouts.",
                                    )
                                    .line_height(rems(1.5)),
                                ),
                            ),
                    ),
            )
            .child(
                section("label-masked", "Masked")
                    .description("Reveal or conceal sensitive values in place.")
                    .w(rems(35.))
                    .items_center()
                    .child(
                        h_flex()
                            .w(rems(20.))
                            .items_center()
                            .justify_between()
                            .p_4()
                            .rounded(cx.theme().radius_lg)
                            .border_1()
                            .border_color(cx.theme().border)
                            .child(
                                v_flex()
                                    .gap_1()
                                    .child(
                                        div()
                                            .text_xs()
                                            .text_color(cx.theme().muted_foreground)
                                            .child("Available balance"),
                                    )
                                    .child(
                                        Label::new("$9,182.10")
                                            .text_2xl()
                                            .font_weight(FontWeight::SEMIBOLD)
                                            .masked(masked),
                                    ),
                            )
                            .child(
                                Button::new("label-mask-toggle")
                                    .ghost()
                                    .icon(if masked {
                                        IconName::EyeOff
                                    } else {
                                        IconName::Eye
                                    })
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        this.masked = !this.masked;
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "label",
        "Label",
        "Display concise text with hierarchy, highlighting, and masking.",
        LabelSection::view(window, cx),
    ));
}
