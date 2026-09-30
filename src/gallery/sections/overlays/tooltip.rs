//! Tooltip section, ported from the upstream `TooltipStory` and the
//! `tooltip_top_edge` example.

use gpui_kit::component::{
    ActiveTheme as _, IconName, Placement, StyledExt as _,
    button::{Button, ButtonVariant, ButtonVariants as _, Toggle},
    checkbox::Checkbox,
    clipboard::Clipboard,
    h_flex,
    radio::Radio,
    switch::Switch,
    tooltip::Tooltip,
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

use crate::gallery::registry::GallerySection;

use super::demo::section;

actions!(gallery_overlays_tooltip, [Info]);

const CONTEXT: &str = "gallery-tooltip";

pub fn init(cx: &mut App) {
    cx.bind_keys([KeyBinding::new("ctrl-shift-delete", Info, Some(CONTEXT))]);
}

pub struct TooltipSection {
    removable_button_visible: bool,
}

impl TooltipSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self {
            removable_button_visible: true,
        })
    }
}

impl Render for TooltipSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_3()
            .p_4()
            .child(
                section("tooltip-button", "Button")
                    .description(
                        "Prefer the left, bottom, or right side, with an optional keyboard shortcut hint.",
                    )
                    .child(
                        Button::new("tooltip-search")
                            .label("Search")
                            .with_variant(ButtonVariant::Primary)
                            .tooltip("This is a search Button.")
                            .tooltip_placement(Placement::Left),
                    )
                    .child(
                        Button::new("tooltip-info")
                            .label("Info")
                            .tooltip_with_action(
                                "This is a tooltip with Action for display keybinding.",
                                &Info,
                                Some(CONTEXT),
                            )
                            .tooltip_placement(Placement::Bottom),
                    )
                    .child(
                        Button::new("tooltip-right")
                            .label("Hover me")
                            .tooltip("This tooltip prefers the right side.")
                            .tooltip_placement(Placement::Right),
                    ),
            )
            .child(
                section("tooltip-checkbox", "Checkbox")
                    .description("Tooltips work on selection controls.")
                    .child(
                        Checkbox::new("tooltip-checkbox-control")
                            .label("Remember me")
                            .checked(true)
                            .tooltip("This is a tooltip"),
                    ),
            )
            .child(
                section("tooltip-radio", "Radio")
                    .description("Explain an individual radio option.")
                    .child(
                        Radio::new("tooltip-radio-control")
                            .label("Radio with tooltip")
                            .checked(true)
                            .tooltip("This is a radio button"),
                    ),
            )
            .child(
                section("tooltip-switch", "Switch")
                    .description("Add context without extending the visible label.")
                    .child(
                        Switch::new("tooltip-switch-control")
                            .checked(true)
                            .tooltip("This is a switch"),
                    ),
            )
            .child(
                section("tooltip-toggle", "Toggle")
                    .description("Describe text and icon-only toggles.")
                    .child(
                        h_flex()
                            .gap_2()
                            .child(
                                Toggle::new("tooltip-toggle-text")
                                    .label("Bold")
                                    .tooltip("Toggle bold"),
                            )
                            .child(
                                Toggle::new("tooltip-toggle-icon")
                                    .icon(IconName::Heart)
                                    .tooltip("Toggle favorite"),
                            ),
                    ),
            )
            .child(
                section("tooltip-clipboard", "Clipboard")
                    .description("Clarify the copy action.")
                    .child(
                        Clipboard::new("tooltip-clipboard-control")
                            .value("Hello, World!")
                            .tooltip("Copy to clipboard"),
                    ),
            )
            .child(
                section("tooltip-custom-content", "Custom content")
                    .description("Build tooltip content with an action hint.")
                    .child(
                        div()
                            .child("Hover me")
                            .id("tooltip-custom-trigger")
                            .tooltip(|window, cx| {
                                Tooltip::new("This is a default tooltip style by GPUI Kit.")
                                    .action(&Info, Some(CONTEXT))
                                    .build(window, cx)
                            }),
                    ),
            )
            .child(
                section("tooltip-removed-trigger", "Removed trigger")
                    .description("Dismiss cleanly when the trigger leaves the view.")
                    .child(
                        h_flex()
                            .gap_2()
                            .when(self.removable_button_visible, |this| {
                                this.child(
                                    Button::new("tooltip-remove-trigger")
                                        .danger()
                                        .label("Remove me")
                                        .tooltip("Clicking this button removes the trigger.")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.removable_button_visible = false;
                                            cx.notify();
                                        })),
                                )
                            })
                            .when(!self.removable_button_visible, |this| {
                                this.child(
                                    Button::new("tooltip-restore-trigger")
                                        .label("Restore button")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.removable_button_visible = true;
                                            cx.notify();
                                        })),
                                )
                            }),
                    ),
            )
            .child(
                section("tooltip-top-edge", "Top edge")
                    .description("A tooltip that does not fit above flips below the trigger.")
                    .v_flex()
                    .items_start()
                    .relative()
                    .min_h(rems(6.))
                    .w_full()
                    // Tall enough that it cannot fit between the trigger and
                    // the window top while this section is near the pane top,
                    // so the Placement::Top fallback genuinely flips below.
                    .child(
                        div().absolute().top_0().left_6().child(
                            Button::new("tooltip-top-edge-trigger")
                                .primary()
                                .label("Hover for tooltip")
                                .tooltip(
                                    "This tooltip is intentionally tall:\n\
                                     taller than the space that can remain\n\
                                     between the trigger and the top of the\n\
                                     window while this section is near the\n\
                                     top of the pane.\n\
                                     With no room above the trigger, the\n\
                                     positioner flips the tooltip below it\n\
                                     instead of clipping it or moving the\n\
                                     trigger.\n\
                                     The preferred placement is still Top:\n\
                                     the flip is the fallback that keeps\n\
                                     the tooltip inside the window.\n\
                                     Scroll the pane down and hover again:\n\
                                     with room above, it opens on top.",
                                )
                                .tooltip_placement(Placement::Top),
                        ),
                    )
                    .child(
                        div()
                            .absolute()
                            .top_16()
                            .left_6()
                            .max_w(rems(26.))
                            .text_color(cx.theme().muted_foreground)
                            .child(
                                "Keep this section near the top of the pane and hover the \
                                 button: the tall tooltip cannot fit above, so it flips \
                                 below the trigger without moving it.",
                            ),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "tooltip",
        "Tooltip",
        "Describe a control on hover.",
        TooltipSection::view(window, cx),
    ));
}
