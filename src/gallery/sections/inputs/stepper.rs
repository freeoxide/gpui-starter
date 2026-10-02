//! Stepper section, ported from the upstream `StepperStory`.

use gpui_kit::component::{
    IconName, Sizable as _, Size, StyledExt as _,
    stepper::{Stepper, StepperItem},
    v_flex,
};
use gpui_kit::*;

use crate::gallery::registry::GallerySection;

use super::demo::{DemoToggle, demo_toolbar, options_dropdown, section, size_dropdown};

pub struct StepperSection {
    size: Size,
    stepper0_step: usize,
    stepper1_step: usize,
    stepper2_step: usize,
    stepper3_step: usize,
    disabled: bool,
}

impl StepperSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self {
            size: Size::default(),
            stepper0_step: 1,
            stepper1_step: 0,
            stepper2_step: 2,
            stepper3_step: 0,
            disabled: false,
        })
    }
}

impl Render for StepperSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_6()
            .p_4()
            .on_action(cx.listener(|this, action: &DemoToggle, _, cx| {
                if let Some(size) = action.size() {
                    this.size = size;
                } else if matches!(action, DemoToggle::StepperDisabled) {
                    this.disabled = !this.disabled;
                } else {
                    return;
                }
                cx.notify();
            }))
            .child(demo_toolbar(vec![
                size_dropdown("stepper-size", self.size).into_any_element(),
                options_dropdown(
                    "stepper-options",
                    vec![("Disabled", self.disabled, DemoToggle::StepperDisabled)],
                )
                .into_any_element(),
            ]))
            .child(
                section("stepper-horizontal", "Horizontal Stepper")
                    .w(rems(30.))
                    .v_flex()
                    .child(
                        Stepper::new("stepper0")
                            .w_full()
                            .with_size(self.size)
                            .disabled(self.disabled)
                            .selected_index(self.stepper0_step)
                            .items([
                                StepperItem::new().child("Step 1"),
                                StepperItem::new().child("Step 2"),
                                StepperItem::new().child("Step 3"),
                            ])
                            .on_click(cx.listener(|this, step, _, cx| {
                                this.stepper0_step = *step;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                section("stepper-icon", "Icon Stepper")
                    .w(rems(30.))
                    .v_flex()
                    .child(
                        Stepper::new("stepper1")
                            .w_full()
                            .with_size(self.size)
                            .disabled(self.disabled)
                            .selected_index(self.stepper1_step)
                            .items([
                                StepperItem::new()
                                    .icon(IconName::Calendar)
                                    .child("Order Details"),
                                StepperItem::new().icon(IconName::Inbox).child("Shipping"),
                                StepperItem::new().icon(IconName::Frame).child("Preview"),
                                StepperItem::new().icon(IconName::Info).child("Finish"),
                            ])
                            .on_click(cx.listener(|this, step, _, cx| {
                                this.stepper1_step = *step;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                section("stepper-vertical", "Vertical Stepper")
                    .w(rems(30.))
                    .v_flex()
                    .child(
                        Stepper::new("stepper3")
                            .vertical()
                            .with_size(self.size)
                            .disabled(self.disabled)
                            .selected_index(self.stepper2_step)
                            .items_center()
                            .items([
                                StepperItem::new().pb_8().icon(IconName::Building2).child(
                                    v_flex().child("Step 1").child("Description for step 1."),
                                ),
                                StepperItem::new().pb_8().icon(IconName::Asterisk).child(
                                    v_flex().child("Step 2").child("Description for step 2."),
                                ),
                                StepperItem::new().pb_8().icon(IconName::Folder).child(
                                    v_flex().child("Step 3").child("Description for step 3."),
                                ),
                                StepperItem::new().icon(IconName::CircleCheck).child(
                                    v_flex().child("Step 4").child("Description for step 4."),
                                ),
                            ])
                            .on_click(cx.listener(|this, step, _, cx| {
                                this.stepper2_step = *step;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                section("stepper-text-center", "Text Center")
                    .w(rems(30.))
                    .v_flex()
                    .child(
                        Stepper::new("stepper4")
                            .with_size(self.size)
                            .disabled(self.disabled)
                            .selected_index(self.stepper3_step)
                            .text_center(true)
                            .items([
                                StepperItem::new().child(
                                    v_flex()
                                        .items_center()
                                        .child("Step 1")
                                        .child("Desc for step 1."),
                                ),
                                StepperItem::new().child(
                                    v_flex()
                                        .items_center()
                                        .child("Step 2")
                                        .child("Desc for step 2."),
                                ),
                                StepperItem::new().child(
                                    v_flex()
                                        .items_center()
                                        .child("Step 3")
                                        .child("Desc for step 3."),
                                ),
                            ])
                            .on_click(cx.listener(|this, step, _, cx| {
                                this.stepper3_step = *step;
                                cx.notify();
                            })),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "stepper",
        "Stepper",
        "A step-by-step process for users to navigate through a series of steps.",
        StepperSection::view(window, cx),
    ));
}
