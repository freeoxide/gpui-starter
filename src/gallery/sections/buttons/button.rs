//! Button section, ported from the upstream `ButtonStory`.

use gpui_kit::component::{
    ActiveTheme as _, Disableable as _, Icon, IconName, Selectable as _, Sizable as _,
    button::{Button, ButtonCustomVariant, ButtonGroup, ButtonVariants as _, DropdownButton},
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
        let multiple = self.toggle_multiple;
        let button = |id: &'static str| {
            Button::new(id)
                .with_size(demo.size)
                .disabled(demo.disabled)
                .selected(demo.selected)
                .loading(demo.loading)
                .when(demo.compact, |this| this.compact())
        };

        let magenta = cx.theme().magenta;
        let custom_variant = ButtonCustomVariant::new(cx)
            .color(magenta)
            .foreground(magenta)
            .hover(magenta.opacity(0.1))
            .active(magenta.opacity(0.2));
        // The solid variant paints ~magenta behind the label, so its text
        // needs the extreme the current mode's magenta is furthest from.
        let solid_foreground = if cx.theme().mode.is_dark() {
            black()
        } else {
            white()
        };
        let solid_custom_variant = ButtonCustomVariant::new(cx)
            .color(magenta)
            .foreground(solid_foreground)
            .hover(magenta.opacity(0.1))
            .active(magenta.opacity(0.2));

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
                // The shared Options dropdown takes no extras, so the story's
                // "Multiple selection" check lives in this section's own menu.
                DropdownButton::new("button-selection-options")
                    .button(Button::new("button-selection-options-trigger").label("Selection"))
                    .dropdown_menu(move |menu, _, _| {
                        menu.menu_with_check(
                            "Multiple selection",
                            multiple,
                            Box::new(ButtonToggle::Multiple),
                        )
                    })
                    .into_any_element(),
            ]))
            .child(
                section("button-variants", "Variants")
                    .description("Visual treatments communicate action priority.")
                    .w_128()
                    .child(button("button-default").label("Default"))
                    .child(button("button-primary").primary().label("Primary"))
                    .child(button("button-secondary").secondary().label("Secondary"))
                    .child(button("button-danger").danger().label("Danger"))
                    .child(button("button-warning").warning().label("Warning"))
                    .child(button("button-success").success().label("Success"))
                    .child(button("button-info").info().label("Info"))
                    .child(button("button-ghost").ghost().label("Ghost"))
                    .child(button("button-link").link().label("Link"))
                    .child(button("button-text").text().label("Text")),
            )
            .child(
                section("button-icons", "Icons")
                    .description("Icons can lead labels or appear in custom content.")
                    .child(
                        button("button-icon-confirm")
                            .outline()
                            .label("Confirm")
                            .icon(IconName::Check),
                    )
                    .child(
                        button("button-icon-abort")
                            .outline()
                            .label("Abort")
                            .icon(IconName::Close),
                    )
                    .child(
                        button("button-icon-maximize")
                            .outline()
                            .label("Maximize")
                            .icon(Icon::new(IconName::Maximize)),
                    )
                    .child(
                        button("button-icon-custom-child").child(
                            h_flex()
                                .items_center()
                                .gap_2()
                                .child("Custom Child")
                                .child(IconName::ChevronDown)
                                .child(IconName::Eye),
                        ),
                    )
                    .child(
                        button("button-icon-ghost")
                            .ghost()
                            .icon(IconName::Check)
                            .label("Confirm"),
                    )
                    .child(
                        button("button-icon-link")
                            .link()
                            .icon(IconName::Check)
                            .label("Link"),
                    )
                    .child(
                        button("button-icon-text")
                            .text()
                            .icon(IconName::Check)
                            .label("Text Button"),
                    ),
            )
            .child(
                section("button-progress", "Progress")
                    .description("Buttons can show determinate progress.")
                    .child(
                        h_flex()
                            .gap_4()
                            .child(
                                Button::new("button-progress-primary")
                                    .with_size(demo.size)
                                    .primary()
                                    .icon(
                                        ProgressCircle::new("button-progress-circle-primary")
                                            .color(cx.theme().primary_foreground)
                                            .value(25.),
                                    )
                                    .label("Installing…"),
                            )
                            .child(
                                Button::new("button-progress-default")
                                    .with_size(demo.size)
                                    .icon(
                                        ProgressCircle::new("button-progress-circle-default")
                                            .value(35.),
                                    )
                                    .label("Installing…"),
                            )
                            .child(
                                Button::new("button-progress-half")
                                    .with_size(demo.size)
                                    .icon(
                                        ProgressCircle::new("button-progress-circle-half")
                                            .value(68.),
                                    )
                                    .label("Installing…"),
                            )
                            .child(
                                Button::new("button-progress-most")
                                    .with_size(demo.size)
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
                            .label("Primary Button"),
                    )
                    .child(
                        button("button-outline-default")
                            .outline()
                            .label("Normal Button"),
                    )
                    .child(
                        button("button-outline-danger")
                            .danger()
                            .outline()
                            .label("Danger Button"),
                    )
                    .child(
                        button("button-outline-warning")
                            .warning()
                            .outline()
                            .label("Warning Button"),
                    )
                    .child(
                        button("button-outline-success")
                            .success()
                            .outline()
                            .label("Success Button"),
                    )
                    .child(
                        button("button-outline-info")
                            .info()
                            .outline()
                            .label("Info Button"),
                    )
                    .child(
                        button("button-outline-ghost")
                            .ghost()
                            .outline()
                            .label("Ghost Button"),
                    )
                    .child(
                        button("button-outline-link")
                            .link()
                            .outline()
                            .label("Link Button"),
                    )
                    .child(
                        button("button-outline-text")
                            .text()
                            .outline()
                            .label("Text Button"),
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
                            .label("Primary Button"),
                    )
                    .child(
                        button("button-dropdown-default")
                            .label("Default Button")
                            .dropdown_caret(true),
                    )
                    .child(
                        button("button-dropdown-secondary")
                            .secondary()
                            .label("Secondary Button")
                            .dropdown_caret(true),
                    )
                    .child(
                        button("button-dropdown-ghost")
                            .ghost()
                            .dropdown_caret(true)
                            .label("Ghost Button"),
                    )
                    .child(
                        button("button-dropdown-link")
                            .link()
                            .dropdown_caret(true)
                            .label("Link Button"),
                    )
                    .child(
                        button("button-dropdown-outline")
                            .outline()
                            .dropdown_caret(true)
                            .label("Small Button"),
                    ),
            )
            .child(
                section("button-group-horizontal", "Horizontal group").child(
                    ButtonGroup::new("button-group")
                        .outline()
                        .disabled(demo.disabled)
                        .child(button("button-group-one").label("One"))
                        .child(button("button-group-two").label("Two"))
                        .child(button("button-group-three").label("Three")),
                ),
            )
            .child(
                section("button-group-vertical", "Vertical group").child(
                    ButtonGroup::new("button-group-vertical")
                        .outline()
                        .layout(Axis::Vertical)
                        .disabled(demo.disabled)
                        .child(button("button-vertical-one").label("One"))
                        .child(button("button-vertical-two").label("Two"))
                        .child(button("button-vertical-three").label("Three")),
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
                                Button::new("button-selection-disabled")
                                    .with_size(demo.size)
                                    .label("Disabled")
                                    .selected(demo.disabled),
                            )
                            .child(
                                Button::new("button-selection-loading")
                                    .with_size(demo.size)
                                    .label("Loading")
                                    .selected(demo.loading),
                            )
                            .child(
                                Button::new("button-selection-selected")
                                    .with_size(demo.size)
                                    .label("Selected")
                                    .selected(demo.selected),
                            )
                            .child(
                                Button::new("button-selection-compact")
                                    .with_size(demo.size)
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
                            .primary(),
                    )
                    .child(
                        button("button-icon-only-info")
                            .icon(IconName::Info)
                            .loading(true),
                    )
                    .child(
                        button("button-icon-only-danger")
                            .icon(IconName::Close)
                            .danger(),
                    )
                    .child(
                        button("button-icon-only-small")
                            .icon(IconName::Search)
                            .primary(),
                    )
                    .child(
                        button("button-icon-only-outline")
                            .icon(IconName::Search)
                            .outline(),
                    )
                    .child(
                        button("button-icon-only-ghost")
                            .icon(IconName::ArrowLeft)
                            .loading_icon(IconName::LoaderCircle)
                            .ghost(),
                    ),
            )
            .child(
                section("button-custom-size", "Custom size")
                    .description("A fixed pixel size is available for compact icon actions.")
                    .child(
                        button("button-custom-size-heart")
                            .icon(IconName::Heart)
                            .size(px(24.))
                            .ghost(),
                    ),
            )
            .child(
                section("button-custom-color", "Custom color")
                    .child(
                        button("button-custom-color-solid")
                            .custom(solid_custom_variant)
                            .label("Custom Button"),
                    )
                    .child(
                        button("button-custom-color-outline")
                            .outline()
                            .custom(custom_variant)
                            .label("Outline Button"),
                    )
                    .child(
                        button("button-custom-color-icon")
                            .outline()
                            .icon(IconName::Bell)
                            .custom(custom_variant)
                            .label("Icon Button"),
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
