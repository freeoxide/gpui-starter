//! Checkbox section, ported from the upstream `CheckboxStory`.

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Sizable as _, Size, StyledExt as _, checkbox::Checkbox,
    h_flex, text::markdown, v_flex,
};
use gpui_kit::{
    App, AppContext, Context, Entity, InteractiveElement, IntoElement, ParentElement, Render,
    Styled, Window, div, rems,
};

use crate::gallery::registry::GallerySection;

use super::demo::{DemoToggle, demo_toolbar, section, size_dropdown};

pub struct CheckboxSection {
    check1: bool,
    check2: bool,
    check3: bool,
    check4: bool,
    check5: bool,
    check6: bool,
    size: Size,
}

impl CheckboxSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self {
            check1: false,
            check2: true,
            check3: false,
            check4: false,
            check5: false,
            check6: false,
            size: Size::default(),
        })
    }
}

impl Render for CheckboxSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
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
                size_dropdown("checkbox-size", size).into_any_element(),
            ]))
            .child(
                section("checkbox-default", "Default")
                    .description("Checked and unchecked options can be mixed freely.")
                    .child(
                        Checkbox::new("checkbox-updates")
                            .with_size(size)
                            .checked(self.check1)
                            .label("Product updates")
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                this.check1 = *checked;
                                cx.notify();
                            })),
                    )
                    .child(
                        Checkbox::new("checkbox-remember")
                            .with_size(size)
                            .checked(self.check2)
                            .label("Remember this device")
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                this.check2 = *checked;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                section("checkbox-without-label", "Without label")
                    .description("The label can be supplied by surrounding content.")
                    .child(
                        Checkbox::new("checkbox-unlabelled")
                            .with_size(size)
                            .checked(self.check3)
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                this.check3 = *checked;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                section("checkbox-disabled", "Disabled")
                    .description("Both checked and unchecked values remain visible.")
                    .w_128()
                    .child(
                        h_flex()
                            .items_center()
                            .gap_6()
                            .child(
                                Checkbox::new("checkbox-disabled-checked")
                                    .with_size(size)
                                    .label("Checked")
                                    .checked(true)
                                    .disabled(true),
                            )
                            .child(
                                Checkbox::new("checkbox-disabled-unchecked")
                                    .with_size(size)
                                    .label("Unchecked")
                                    .checked(false)
                                    .disabled(true),
                            ),
                    ),
            )
            .child(
                section("checkbox-labels", "Labels")
                    .description("Labels can wrap and include supporting content.")
                    .w_128()
                    .v_flex()
                    .items_center()
                    .gap_5()
                    .child(
                        Checkbox::new("checkbox-description")
                            .with_size(size)
                            .w(rems(20.))
                            .checked(self.check4)
                            .label("Automatic updates")
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Download updates when the application is idle."),
                            )
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                this.check4 = *checked;
                                cx.notify();
                            })),
                    )
                    .child(
                        Checkbox::new("checkbox-wrapping")
                            .with_size(size)
                            .w(rems(20.))
                            .checked(self.check6)
                            .label("Notify me when a new device signs in to my account")
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                this.check6 = *checked;
                                cx.notify();
                            })),
                    )
                    .child(
                        Checkbox::new("checkbox-markdown")
                            .with_size(size)
                            .w(rems(20.))
                            .checked(self.check5)
                            .label("Accept the terms")
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(markdown(
                                        "Read the [terms of service](https://github.com) before continuing.",
                                    )),
                            )
                            .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                this.check5 = *checked;
                                cx.notify();
                            })),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "checkbox",
        "Checkbox",
        "Select one or more independent options.",
        CheckboxSection::view(window, cx),
    ));
}
