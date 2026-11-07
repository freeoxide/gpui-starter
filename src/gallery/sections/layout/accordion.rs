//! Accordion section, ported from the upstream `AccordionStory`.

use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, Size, StyledExt as _,
    accordion::{Accordion, AccordionItem},
    button::{Button, DropdownButton},
    checkbox::Checkbox,
    h_flex,
    switch::Switch,
    tag::Tag,
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};
use serde::Deserialize;

use crate::gallery::demo::{self, SizeToggle, demo_toolbar, section};
use crate::gallery::registry::GallerySection;

/// Knobs of the upstream options toolbar plus the size switcher.
#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_layout, no_json)]
enum DemoToggle {
    Multiple,
    Icons,
    Disabled,
    Bordered,
    SizeXSmall,
    SizeSmall,
    SizeMedium,
    SizeLarge,
}

impl DemoToggle {
    /// The size this toggle selects.
    fn size(&self) -> Option<Size> {
        match self {
            Self::SizeXSmall => Some(Size::XSmall),
            Self::SizeSmall => Some(Size::Small),
            Self::SizeMedium => Some(Size::Medium),
            Self::SizeLarge => Some(Size::Large),
            _ => None,
        }
    }
}

impl SizeToggle for DemoToggle {
    fn xsmall() -> Self {
        Self::SizeXSmall
    }
    fn small() -> Self {
        Self::SizeSmall
    }
    fn medium() -> Self {
        Self::SizeMedium
    }
    fn large() -> Self {
        Self::SizeLarge
    }
}

/// The theme-derived values a settings row draws with, read once so the row
/// builder itself needs no app context.
#[derive(Clone, Copy)]
struct SettingsItemStyle {
    icon_bg: Hsla,
    muted: Hsla,
    icon_radius: Pixels,
}

/// A settings row: the icon sits in a rounded square, and the content lines up
/// with the title rather than with the icon.
fn settings_item(
    item: AccordionItem,
    icon: IconName,
    title: &'static str,
    tag: Option<Tag>,
    body: &'static str,
    style: SettingsItemStyle,
) -> AccordionItem {
    let SettingsItemStyle {
        icon_bg,
        muted,
        icon_radius,
    } = style;

    item.title(
        h_flex()
            .gap_2()
            .items_center()
            .child(
                h_flex()
                    .flex_none()
                    .size(rems(2.))
                    .items_center()
                    .justify_center()
                    .rounded(icon_radius)
                    .bg(icon_bg)
                    .child(Icon::new(icon).small().text_color(muted)),
            )
            .child(div().font_semibold().child(title))
            .children(tag.map(|tag| tag.small())),
    )
    .title_style({
        let mut style = StyleRefinement::default();
        style.padding.top = Some(rems(0.5).into());
        style.padding.bottom = Some(rems(0.5).into());
        style
    })
    .content_style({
        let mut style = StyleRefinement::default();
        style.text.color = Some(muted);
        // Past the icon square, so the text starts under the title.
        style.padding.left = Some(rems(3.25).into());
        style.padding.top = Some(px(0.).into());
        style.padding.bottom = Some(rems(0.75).into());
        style
    })
    .child(body)
}

pub struct AccordionSection {
    open_ixs: Vec<usize>,
    styled_open_ixs: Vec<usize>,
    size: Size,
    bordered: bool,
    disabled: bool,
    multiple: bool,
    show_icon: bool,
}

impl AccordionSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self {
            open_ixs: vec![0],
            styled_open_ixs: vec![0],
            size: Size::default(),
            bordered: false,
            disabled: false,
            multiple: false,
            show_icon: false,
        })
    }
}

impl Render for AccordionSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let size = self.size;
        let multiple = self.multiple;
        let show_icon = self.show_icon;
        let disabled = self.disabled;
        let bordered = self.bordered;
        let settings_item_style = SettingsItemStyle {
            icon_bg: cx.theme().secondary.opacity(0.5),
            muted: cx.theme().muted_foreground,
            icon_radius: cx.theme().radius,
        };

        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .p_4()
            .on_action(cx.listener(|this, action: &DemoToggle, _, cx| {
                if let Some(size) = action.size() {
                    this.size = size;
                } else {
                    match action {
                        DemoToggle::Multiple => this.multiple = !this.multiple,
                        DemoToggle::Icons => this.show_icon = !this.show_icon,
                        DemoToggle::Disabled => this.disabled = !this.disabled,
                        DemoToggle::Bordered => this.bordered = !this.bordered,
                        _ => {}
                    }
                }
                cx.notify();
            }))
            .child(demo_toolbar(vec![
                demo::size_dropdown::<DemoToggle>("accordion-size", size).into_any_element(),
                DropdownButton::new("accordion-options")
                    .button(Button::new("accordion-options-trigger").label("Options"))
                    .dropdown_menu(move |menu, _, _| {
                        menu.menu_with_check("Multiple", multiple, Box::new(DemoToggle::Multiple))
                            .menu_with_check("Icons", show_icon, Box::new(DemoToggle::Icons))
                            .menu_with_check("Disabled", disabled, Box::new(DemoToggle::Disabled))
                            .menu_with_check("Bordered", bordered, Box::new(DemoToggle::Bordered))
                    })
                    .into_any_element(),
            ]))
            .child(
                section("accordion-default", "Default")
                    .description("Expand one item at a time by default.")
                    .w(rems(30.))
                    .child(
                        Accordion::new("accordion-default")
                            .bordered(self.bordered)
                            .with_size(size)
                            .disabled(self.disabled)
                            .multiple(self.multiple)
                            .item(|this| {
                                this.open(self.open_ixs.contains(&0))
                                    .when(show_icon, |this| this.icon(IconName::Info))
                                    .title("Is it accessible?")
                                    .child(
                                        "Yes. Each item is a button with an aria-expanded \
                                         state, so screen readers announce whether the \
                                         section is open, and the whole group can be \
                                         reached with the keyboard.",
                                    )
                            })
                            .item(|this| {
                                this.open(self.open_ixs.contains(&1))
                                    .when(show_icon, |this| this.icon(IconName::Inbox))
                                    .title("Can it hold any content?")
                                    .child(
                                        v_flex()
                                            .gap_3()
                                            .child(
                                                "An item takes any element as its content, \
                                                 not just text. The height animation measures \
                                                 whatever you put in it.",
                                            )
                                            .child(
                                                h_flex()
                                                    .gap_4()
                                                    .child(
                                                        Switch::new("accordion-switch")
                                                            .label("Switch"),
                                                    )
                                                    .child(
                                                        Checkbox::new("accordion-checkbox")
                                                            .label("Or a Checkbox"),
                                                    ),
                                            ),
                                    )
                            })
                            .item(|this| {
                                this.open(self.open_ixs.contains(&2))
                                    .when(show_icon, |this| this.icon(IconName::Moon))
                                    .title("Is it animated?")
                                    .child(
                                        "Yes. Expanding and collapsing animates the height \
                                         of the content, and the chevron rotates to follow. \
                                         Items below move along with it.",
                                    )
                            })
                            .item(|this| {
                                this.title("Disabled item")
                                    .disabled(true)
                                    .child("This item cannot be expanded.")
                            })
                            .on_toggle_click(cx.listener(|this, open_ixs: &[usize], _, cx| {
                                this.open_ixs = open_ixs.to_vec();
                                cx.notify();
                            })),
                    ),
            )
            .child(
                section("accordion-custom-style", "Custom style")
                    .w(rems(30.))
                    .child(
                        div()
                            .w_full()
                            .p_1()
                            .rounded(cx.theme().radius_lg * 2.)
                            .bg(cx.theme().secondary.opacity(0.5))
                            .border_1()
                            .border_color(cx.theme().border.opacity(0.5))
                            .child(
                                Accordion::new("accordion-custom-style")
                                    .multiple(self.multiple)
                                    .disabled(self.disabled)
                                    .item(|this| {
                                        settings_item(
                                            this,
                                            IconName::Settings,
                                            "Account Settings",
                                            Some(Tag::success().outline().child("New")),
                                            "Manage your account preferences, security \
                                             settings, and personal information. You can also \
                                             configure two-factor authentication here.",
                                            settings_item_style,
                                        )
                                        .open(self.styled_open_ixs.contains(&0))
                                    })
                                    .item(|this| {
                                        settings_item(
                                            this,
                                            IconName::Eye,
                                            "Privacy & Security",
                                            None,
                                            "Control who can see your profile and how your \
                                             data is used.",
                                            settings_item_style,
                                        )
                                        .open(self.styled_open_ixs.contains(&1))
                                    })
                                    .item(|this| {
                                        settings_item(
                                            this,
                                            IconName::Info,
                                            "Help & Support",
                                            None,
                                            "Browse the documentation, or get in touch with \
                                             the support team.",
                                            settings_item_style,
                                        )
                                        .open(self.styled_open_ixs.contains(&2))
                                    })
                                    .on_toggle_click(cx.listener(
                                        |this, open_ixs: &[usize], _, cx| {
                                            this.styled_open_ixs = open_ixs.to_vec();
                                            cx.notify();
                                        },
                                    )),
                            ),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "accordion",
        "Accordion",
        "The accordion uses collapse internally to make it collapsible.",
        AccordionSection::view(window, cx),
    ));
}
