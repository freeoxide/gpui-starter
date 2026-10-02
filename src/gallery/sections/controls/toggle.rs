//! Toggle section, ported from the upstream `ToggleStory`.

use gpui_kit::component::{
    IconName, Sizable as _, Size, StyledExt as _,
    button::{Toggle, ToggleGroup, ToggleVariants as _},
    h_flex, v_flex,
};
use gpui_kit::{
    App, AppContext, Context, Entity, FontWeight, InteractiveElement, IntoElement, ParentElement,
    Render, Styled, Window, div,
};

use crate::gallery::registry::GallerySection;

use super::demo::{DemoToggle, demo_toolbar, section, size_dropdown};

pub struct ToggleSection {
    single_toggle: usize,
    checked: Vec<bool>,
    size: Size,
}

impl ToggleSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self {
            single_toggle: 1,
            checked: vec![false; 12],
            size: Size::Medium,
        })
    }
}

impl Render for ToggleSection {
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
                size_dropdown("toggle-size", size).into_any_element(),
            ]))
            .child(
                section("toggle-default", "Default")
                    .description("Text and icon toggles with clear selected states.")
                    .w_128()
                    .v_flex()
                    .items_center()
                    .gap_3()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Toggle::new("toggle-preview")
                                    .label("Preview")
                                    .with_size(size)
                                    .checked(self.single_toggle == 1)
                                    .on_click(cx.listener(|this, checked, _, cx| {
                                        this.single_toggle = usize::from(*checked);
                                        cx.notify();
                                    })),
                            )
                            .child(
                                Toggle::new("toggle-favorite")
                                    .icon(IconName::Star)
                                    .with_size(size)
                                    .checked(self.checked[0])
                                    .on_click(cx.listener(|this, checked, _, cx| {
                                        this.checked[0] = *checked;
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .child(
                section("toggle-variants", "Variants")
                    .description("Ghost and outline treatments for different surfaces.")
                    .w_128()
                    .v_flex()
                    .items_center()
                    .gap_4()
                    .child(
                        v_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child("Ghost"),
                            )
                            .child(
                                ToggleGroup::new("toggle-ghost-group")
                                    .with_size(size)
                                    .child(
                                        Toggle::new("toggle-ghost-bell")
                                            .icon(IconName::Bell)
                                            .checked(self.checked[1]),
                                    )
                                    .child(
                                        Toggle::new("toggle-ghost-inbox")
                                            .icon(IconName::Inbox)
                                            .checked(self.checked[2]),
                                    )
                                    .child(
                                        Toggle::new("toggle-ghost-check")
                                            .icon(IconName::Check)
                                            .checked(self.checked[3]),
                                    )
                                    .on_click(cx.listener(|this, values: &Vec<bool>, _, cx| {
                                        this.checked[1..4].copy_from_slice(values);
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        v_flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_sm()
                                    .font_weight(FontWeight::MEDIUM)
                                    .child("Outline"),
                            )
                            .child(
                                ToggleGroup::new("toggle-outline-group")
                                    .outline()
                                    .with_size(size)
                                    .child(
                                        Toggle::new("toggle-outline-bell")
                                            .icon(IconName::Bell)
                                            .checked(self.checked[4]),
                                    )
                                    .child(
                                        Toggle::new("toggle-outline-inbox")
                                            .icon(IconName::Inbox)
                                            .checked(self.checked[5]),
                                    )
                                    .child(
                                        Toggle::new("toggle-outline-check")
                                            .icon(IconName::Check)
                                            .checked(self.checked[6]),
                                    )
                                    .on_click(cx.listener(|this, values: &Vec<bool>, _, cx| {
                                        this.checked[4..7].copy_from_slice(values);
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .child(
                section("toggle-group", "Group")
                    .description("Connected toggles keep related choices together.")
                    .w_128()
                    .v_flex()
                    .items_center()
                    .child(
                        ToggleGroup::new("toggle-segmented-group")
                            .segmented()
                            .outline()
                            .with_size(size)
                            .child(
                                Toggle::new("toggle-segmented-bold")
                                    .label("Bold")
                                    .checked(self.checked[7]),
                            )
                            .child(
                                Toggle::new("toggle-segmented-italic")
                                    .label("Italic")
                                    .checked(self.checked[8]),
                            )
                            .child(
                                Toggle::new("toggle-segmented-code")
                                    .label("Code")
                                    .checked(self.checked[9]),
                            )
                            .on_click(cx.listener(|this, values: &Vec<bool>, _, cx| {
                                this.checked[7..10].copy_from_slice(values);
                                cx.notify();
                            })),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "toggle",
        "Toggle",
        "Turn an option on or off, alone or in a group.",
        ToggleSection::view(window, cx),
    ));
}
