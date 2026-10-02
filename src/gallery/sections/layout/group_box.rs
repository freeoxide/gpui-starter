//! GroupBox section, ported from the upstream `GroupBoxStory`.

use gpui_kit::component::{
    ActiveTheme as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    checkbox::Checkbox,
    group_box::{GroupBox, GroupBoxVariants as _},
    h_flex,
    radio::{Radio, RadioGroup},
    switch::Switch,
    text::markdown,
    v_flex,
};
use gpui_kit::*;

use crate::gallery::registry::GallerySection;

use super::demo::section;

pub struct GroupBoxSection {
    email_options: [bool; 3],
    profile_private: bool,
    private_contributions: bool,
    compact_private: bool,
    theme: Option<usize>,
}

impl GroupBoxSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self {
            email_options: [false; 3],
            profile_private: true,
            private_contributions: false,
            compact_private: true,
            theme: Some(2),
        })
    }
}

impl Render for GroupBoxSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .p_4()
            .child(
                section("group-box-default", "Default").w_128().child(
                    GroupBox::new()
                        .title("Email notifications")
                        .child(
                            Checkbox::new("group-box-all")
                                .label("All activity")
                                .checked(self.email_options[0])
                                .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                    this.email_options[0] = *checked;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Checkbox::new("group-box-news-letter")
                                .label("Product updates")
                                .checked(self.email_options[1])
                                .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                    this.email_options[1] = *checked;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Checkbox::new("group-box-account-activity")
                                .label("Account activity")
                                .checked(self.email_options[2])
                                .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                    this.email_options[2] = *checked;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Button::new("group-box-ok")
                                .primary()
                                .label("Save preferences"),
                        ),
                ),
            )
            .child(
                section("group-box-filled", "Filled").w_128().child(
                    GroupBox::new()
                        .id("group-box-activity")
                        .fill()
                        .title("Contributions & activity")
                        .footer("Private contributions never reveal repository names.")
                        .child(
                            h_flex()
                                .justify_between()
                                .child("Make profile private and hide activity")
                                .child(
                                    Switch::new("group-box-profile-private")
                                        .checked(self.profile_private)
                                        .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                            this.profile_private = *checked;
                                            cx.notify();
                                        })),
                                ),
                        )
                        .child(
                            h_flex()
                                .justify_between()
                                .child("Include private contributions on my profile")
                                .child(
                                    Switch::new("group-box-private-contributions")
                                        .checked(self.private_contributions)
                                        .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                            this.private_contributions = *checked;
                                            cx.notify();
                                        })),
                                ),
                        )
                        .child(Button::new("group-box-btn-1").primary().label("Save")),
                ),
            )
            .child(
                section("group-box-outlined", "Outlined").w_128().child(
                    GroupBox::new()
                        .id("group-box-appearance")
                        .outline()
                        .title("Appearance")
                        .child(
                            RadioGroup::vertical("group-box-theme")
                                .child(Radio::new("group-box-light").label("Light"))
                                .child(Radio::new("group-box-dark").label("Dark"))
                                .child(Radio::new("group-box-system").label("System"))
                                .selected_index(self.theme)
                                .on_click(cx.listener(|this, selected: &usize, _, cx| {
                                    this.theme = Some(*selected);
                                    cx.notify();
                                })),
                        ),
                ),
            )
            .child(
                section("group-box-without-title", "Without Title")
                    .w_128()
                    .child(
                        GroupBox::new().outline().child(
                            h_flex()
                                .justify_between()
                                .child("Make profile private and hide activity")
                                .child(
                                    Switch::new("group-box-compact-private")
                                        .checked(self.compact_private)
                                        .on_click(cx.listener(|this, checked: &bool, _, cx| {
                                            this.compact_private = *checked;
                                            cx.notify();
                                        })),
                                ),
                        ),
                    ),
            )
            .child(
                section("group-box-custom-style", "Custom Style")
                    .w_128()
                    .child(
                        GroupBox::new()
                            .outline()
                            .bg(cx.theme().group_box)
                            .rounded_xl()
                            .p_5()
                            .title("This is a custom style")
                            .title_style(
                                StyleRefinement::default()
                                    .font_semibold()
                                    .line_height(relative(1.0))
                                    .px_3(),
                            )
                            .content_style(
                                StyleRefinement::default()
                                    .rounded_xl()
                                    .py_3()
                                    .px_4()
                                    .border_2(),
                            )
                            .child(markdown(
                                "You can use `title_style` to customize the style \
                                of the title. \n \
                                And any style in `GroupBox` will apply to the content container.",
                            )),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "group-box",
        "Group Box",
        "A styled container with an optional title that groups related content together.",
        GroupBoxSection::view(window, cx),
    ));
}
