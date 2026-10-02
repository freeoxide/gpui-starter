//! Switch section, ported from the upstream `SwitchStory`.

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Sizable as _, Size, h_flex, separator::Separator,
    switch::Switch, v_flex,
};
use gpui_kit::{
    App, AppContext, Context, Entity, FontWeight, InteractiveElement, IntoElement, ParentElement,
    Render, Styled, Window, div, rems,
};

use crate::gallery::registry::GallerySection;

use super::demo::{DemoToggle, demo_toolbar, section, size_dropdown};

pub struct SwitchSection {
    switch1: bool,
    switch2: bool,
    switch3: bool,
    switch4: bool,
    switch5: bool,
    long_label_checked: bool,
    size: Size,
}

impl SwitchSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self {
            switch1: true,
            switch2: false,
            switch3: true,
            switch4: true,
            switch5: false,
            long_label_checked: false,
            size: Size::default(),
        })
    }
}

impl Render for SwitchSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
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
                size_dropdown("switch-size", size).into_any_element(),
            ]))
            .child(
                section("switch-default", "Default")
                    .description("Switches work well in a compact settings list.")
                    .w_128()
                    .items_stretch()
                    .child(
                        v_flex()
                            .w_full()
                            .border_1()
                            .border_color(theme.border)
                            .rounded(theme.radius_lg)
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .gap_6()
                                    .p_4()
                                    .child(
                                        v_flex()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .child("Product updates"),
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(theme.muted_foreground)
                                                    .child("New features and release notes."),
                                            ),
                                    )
                                    .child(
                                        Switch::new("switch-updates")
                                            .with_size(size)
                                            .checked(self.switch1)
                                            .on_click(cx.listener(|this, checked, _, cx| {
                                                this.switch1 = *checked;
                                                cx.notify();
                                            })),
                                    ),
                            )
                            .child(Separator::horizontal())
                            .child(
                                h_flex()
                                    .w_full()
                                    .items_center()
                                    .justify_between()
                                    .gap_6()
                                    .p_4()
                                    .child(
                                        v_flex()
                                            .gap_1()
                                            .child(
                                                div()
                                                    .font_weight(FontWeight::MEDIUM)
                                                    .child("Security alerts"),
                                            )
                                            .child(
                                                div()
                                                    .text_sm()
                                                    .text_color(theme.muted_foreground)
                                                    .child("Important activity on your account."),
                                            ),
                                    )
                                    .child(
                                        Switch::new("switch-alerts")
                                            .with_size(size)
                                            .checked(self.switch2)
                                            .on_click(cx.listener(|this, checked, _, cx| {
                                                this.switch2 = *checked;
                                                cx.notify();
                                            })),
                                    ),
                            ),
                    ),
            )
            .child(
                section("switch-long-labels", "Long labels")
                    .description("Long setting names wrap while the track keeps its size.")
                    .child(
                        v_flex()
                            .w(rems(20.))
                            .border_1()
                            .border_color(theme.border)
                            .rounded(theme.radius_lg)
                            .child(
                                Switch::new("switch-long-label")
                                    .p_4()
                                    .with_size(size)
                                    .label("Automatically transcribe downloaded episodes")
                                    .checked(self.long_label_checked)
                                    .on_change(cx.listener(|this, checked, _, cx| {
                                        this.long_label_checked = *checked;
                                        cx.notify();
                                    })),
                            )
                            .child(Separator::horizontal())
                            .child(
                                Switch::new("switch-long-label-disabled")
                                    .p_4()
                                    .with_size(size)
                                    .label("Automatically download new episodes")
                                    .checked(true)
                                    .disabled(true),
                            ),
                    ),
            )
            .child(
                section("switch-disabled", "Disabled")
                    .description("Unavailable switches preserve their current value.")
                    .w_128()
                    .child(
                        Switch::new("switch-disabled")
                            .with_size(size)
                            .checked(self.switch3)
                            .disabled(true),
                    )
                    .child(
                        Switch::new("switch-disabled-airplane")
                            .with_size(size)
                            .w(rems(12.5))
                            .label("Airplane mode")
                            .checked(true)
                            .disabled(true),
                    ),
            )
            .child(
                section("switch-color", "Color")
                    .description("Semantic colors can reinforce the setting state.")
                    .child(
                        Switch::new("switch-success")
                            .with_size(size)
                            .checked(self.switch4)
                            .label("Success")
                            .color(theme.success)
                            .on_click(cx.listener(|this, checked, _, cx| {
                                this.switch4 = *checked;
                                cx.notify();
                            })),
                    )
                    .child(
                        Switch::new("switch-destructive")
                            .with_size(size)
                            .checked(self.switch5)
                            .label("Destructive")
                            .color(theme.danger)
                            .on_click(cx.listener(|this, checked, _, cx| {
                                this.switch5 = *checked;
                                cx.notify();
                            })),
                    )
                    .child(
                        Switch::new("switch-success-disabled")
                            .with_size(size)
                            .checked(true)
                            .label("Disabled")
                            .color(theme.success)
                            .disabled(true),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "switch",
        "Switch",
        "Turn a setting on or off.",
        SwitchSection::view(window, cx),
    ));
}
