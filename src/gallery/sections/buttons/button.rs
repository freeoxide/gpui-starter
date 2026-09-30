//! Button section, ported from the upstream `ButtonStory`.

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, Selectable as _, Sizable as _,
    button::{Button, ButtonCustomVariant, ButtonGroup, ButtonVariants as _},
    h_flex,
    progress::ProgressCircle,
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};
use serde::Deserialize;

use crate::gallery::registry::GallerySection;

use super::demo::{DemoState, DemoToggle, demo_toolbar, options_dropdown, section, size_dropdown};

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_button_section, no_json)]
enum ButtonToggle {
    Multiple,
}

pub struct ButtonSection {
    demo: DemoState,
    toggle_multiple: bool,
}

impl ButtonSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self {
            demo: DemoState::default(),
            toggle_multiple: false,
        })
    }
}

impl Render for ButtonSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let demo = &self.demo;
        let button = |id: &'static str| Button::new(id).with_size(demo.size);

        let custom_variant = ButtonCustomVariant::new(cx)
            .color(cx.theme().magenta)
            .foreground(cx.theme().magenta)
            .hover(cx.theme().magenta.opacity(0.1))
            .active(cx.theme().magenta.opacity(0.2));

        v_flex()
            .w_full()
            .gap_6()
            .p_4()
            .on_action(cx.listener(|this, action: &DemoToggle, window, cx| {
                if this.demo.apply(action, window, cx) {
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this, _: &ButtonToggle, _, cx| {
                this.toggle_multiple = !this.toggle_multiple;
                cx.notify();
            }))
            .child(demo_toolbar(vec![
                size_dropdown("button-size", demo.size).into_any_element(),
                options_dropdown("button-options", demo).into_any_element(),
            ]))
            .child(
                section("button-variants", "Variants")
                    .description("Visual treatments communicate action priority.")
                    .w_128()
                    .child(
                        button("button-default")
                            .label("Default")
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-primary")
                            .primary()
                            .label("Primary")
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-secondary")
                            .secondary()
                            .label("Secondary")
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-danger")
                            .danger()
                            .label("Danger")
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-warning")
                            .warning()
                            .label("Warning")
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-success")
                            .success()
                            .label("Success")
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-info")
                            .info()
                            .label("Info")
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-ghost")
                            .ghost()
                            .label("Ghost")
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-link")
                            .link()
                            .label("Link")
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-text")
                            .text()
                            .label("Text")
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    ),
            )
            .child(
                section("button-icons", "Icons")
                    .description("Icons can lead labels or appear in custom content.")
                    .child(
                        button("button-icon-confirm")
                            .outline()
                            .label("Confirm")
                            .icon(IconName::Check)
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-icon-abort")
                            .outline()
                            .label("Abort")
                            .icon(IconName::Close)
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-icon-maximize")
                            .outline()
                            .label("Maximize")
                            .icon(Icon::new(IconName::Maximize))
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-icon-custom-child")
                            .child(
                                h_flex()
                                    .items_center()
                                    .gap_2()
                                    .child("Custom Child")
                                    .child(IconName::ChevronDown)
                                    .child(IconName::Eye),
                            )
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-icon-ghost")
                            .ghost()
                            .icon(IconName::Check)
                            .label("Confirm")
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-icon-link")
                            .link()
                            .icon(IconName::Check)
                            .label("Link")
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-icon-text")
                            .text()
                            .icon(IconName::Check)
                            .label("Text Button")
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    ),
            )
            .child(
                section("button-progress", "Progress")
                    .description("Buttons can show determinate progress.")
                    .child(
                        h_flex()
                            .gap_4()
                            .child(
                                button("button-progress-primary")
                                    .primary()
                                    .icon(
                                        ProgressCircle::new("button-progress-circle-primary")
                                            .color(cx.theme().primary_foreground)
                                            .value(25.),
                                    )
                                    .label("Installing…"),
                            )
                            .child(
                                button("button-progress-default")
                                    .icon(
                                        ProgressCircle::new("button-progress-circle-default")
                                            .value(35.),
                                    )
                                    .label("Installing…"),
                            )
                            .child(
                                button("button-progress-half")
                                    .icon(
                                        ProgressCircle::new("button-progress-circle-half")
                                            .value(68.),
                                    )
                                    .label("Installing…"),
                            )
                            .child(
                                button("button-progress-most")
                                    .icon(
                                        ProgressCircle::new("button-progress-circle-most")
                                            .value(85.),
                                    )
                                    .label("Installing…"),
                            ),
                    ),
            )
            .child(
                section("button-outline", "Outline")
                    .description("Outlined treatments keep actions visually quiet.")
                    .w_128()
                    .child(
                        button("button-outline-primary")
                            .primary()
                            .outline()
                            .label("Primary Button")
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-outline-default")
                            .outline()
                            .label("Normal Button")
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-outline-danger")
                            .danger()
                            .outline()
                            .label("Danger Button")
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-outline-warning")
                            .warning()
                            .outline()
                            .label("Warning Button")
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-outline-success")
                            .success()
                            .outline()
                            .label("Success Button")
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-outline-info")
                            .info()
                            .outline()
                            .label("Info Button")
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-outline-ghost")
                            .ghost()
                            .outline()
                            .label("Ghost Button")
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-outline-link")
                            .link()
                            .outline()
                            .label("Link Button")
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-outline-text")
                            .text()
                            .outline()
                            .label("Text Button")
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    ),
            )
            .child(
                section("button-dropdown-caret", "Dropdown")
                    .description("A caret indicates an attached menu.")
                    .w_128()
                    .child(
                        button("button-dropdown-primary")
                            .primary()
                            .dropdown_caret(true)
                            .label("Primary Button")
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-dropdown-default")
                            .label("Default Button")
                            .dropdown_caret(true)
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-dropdown-secondary")
                            .secondary()
                            .label("Secondary Button")
                            .dropdown_caret(true)
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-dropdown-ghost")
                            .ghost()
                            .dropdown_caret(true)
                            .label("Ghost Button")
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-dropdown-link")
                            .link()
                            .dropdown_caret(true)
                            .label("Link Button")
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-dropdown-outline")
                            .outline()
                            .dropdown_caret(true)
                            .label("Small Button")
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    ),
            )
            .child(
                section("button-group-horizontal", "Horizontal group").child(
                    ButtonGroup::new("button-group")
                        .outline()
                        .disabled(demo.disabled)
                        .child(
                            button("button-group-one")
                                .label("One")
                                .disabled(demo.disabled)
                                .selected(demo.selected)
                                .when(demo.compact, |this| this.compact()),
                        )
                        .child(
                            button("button-group-two")
                                .label("Two")
                                .disabled(demo.disabled)
                                .selected(demo.selected)
                                .when(demo.compact, |this| this.compact()),
                        )
                        .child(
                            button("button-group-three")
                                .label("Three")
                                .disabled(demo.disabled)
                                .selected(demo.selected)
                                .when(demo.compact, |this| this.compact()),
                        ),
                ),
            )
            .child(
                section("button-group-vertical", "Vertical group").child(
                    ButtonGroup::new("button-group-vertical")
                        .outline()
                        .layout(Axis::Vertical)
                        .disabled(demo.disabled)
                        .child(
                            button("button-vertical-one")
                                .label("One")
                                .disabled(demo.disabled)
                                .selected(demo.selected)
                                .when(demo.compact, |this| this.compact()),
                        )
                        .child(
                            button("button-vertical-two")
                                .label("Two")
                                .disabled(demo.disabled)
                                .selected(demo.selected)
                                .when(demo.compact, |this| this.compact()),
                        )
                        .child(
                            button("button-vertical-three")
                                .label("Three")
                                .disabled(demo.disabled)
                                .selected(demo.selected)
                                .when(demo.compact, |this| this.compact()),
                        ),
                ),
            )
            .child(
                section("button-group-selection", "Selection group")
                    .description("Groups support single or multiple selection.")
                    .child(
                        ButtonGroup::new("button-selection-group")
                            .outline()
                            .compact()
                            .multiple(self.toggle_multiple)
                            .child(
                                button("button-selection-disabled")
                                    .label("Disabled")
                                    .selected(demo.disabled),
                            )
                            .child(
                                button("button-selection-loading")
                                    .label("Loading")
                                    .selected(demo.loading),
                            )
                            .child(
                                button("button-selection-selected")
                                    .label("Selected")
                                    .selected(demo.selected),
                            )
                            .child(
                                button("button-selection-compact")
                                    .label("Compact")
                                    .selected(demo.compact),
                            )
                            .on_click(cx.listener(|view, selected: &Vec<usize>, _, cx| {
                                view.demo.disabled = selected.contains(&0);
                                view.demo.loading = selected.contains(&1);
                                view.demo.selected = selected.contains(&2);
                                view.demo.compact = selected.contains(&3);
                                cx.notify();
                            })),
                    ),
            )
            .child(
                section("button-icon-only", "Icon-only")
                    .description("Compact actions can omit visible labels.")
                    .child(
                        button("button-icon-only-search")
                            .icon(IconName::Search)
                            .loading_icon(IconName::LoaderCircle)
                            .primary()
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-icon-only-info")
                            .icon(IconName::Info)
                            .loading(true)
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-icon-only-danger")
                            .icon(IconName::Close)
                            .danger()
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-icon-only-small")
                            .icon(IconName::Search)
                            .primary()
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-icon-only-outline")
                            .icon(IconName::Search)
                            .outline()
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-icon-only-ghost")
                            .icon(IconName::ArrowLeft)
                            .loading_icon(IconName::LoaderCircle)
                            .ghost()
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    ),
            )
            .child(
                section("button-custom-size", "Custom size")
                    .description("A fixed pixel size is available for compact icon actions.")
                    .child(
                        button("button-custom-size-heart")
                            .icon(IconName::Heart)
                            .size(px(24.))
                            .ghost()
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    ),
            )
            .child(
                section("button-custom-color", "Custom color")
                    .child(
                        button("button-custom-color-solid")
                            .custom(custom_variant)
                            .label("Custom Button")
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-custom-color-outline")
                            .outline()
                            .custom(custom_variant)
                            .label("Outline Button")
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    )
                    .child(
                        button("button-custom-color-icon")
                            .outline()
                            .icon(IconName::Bell)
                            .custom(custom_variant)
                            .label("Icon Button")
                            .disabled(demo.disabled)
                            .selected(demo.selected)
                            .loading(demo.loading)
                            .when(demo.compact, |this| this.compact()),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "button",
        "Button",
        "Displays a button or a component that looks like a button.",
        ButtonSection::view(window, cx),
    ));
}
