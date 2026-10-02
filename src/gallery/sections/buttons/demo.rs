//! Shared scaffolding for the button-family sections: the titled demo box
//! (ported from the story crate's `section()` helper), the common demo-state
//! knobs, and the toolbar dropdowns that drive them.

use gpui_kit::component::{
    ActiveTheme as _, Size,
    button::{Button, DropdownButton},
    group_box::{GroupBox, GroupBoxVariants as _},
    h_flex,
    menu::PopupMenu,
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};
use serde::Deserialize;

/// Knobs the upstream stories expose through their options menu; several
/// sections drive every control they render from one copy of this state.
#[derive(Default)]
pub(crate) struct DemoState {
    pub(crate) disabled: bool,
    pub(crate) loading: bool,
    pub(crate) selected: bool,
    pub(crate) compact: bool,
    pub(crate) size: Size,
}

/// Shared check-menu toggles for the button-family sections. Each section
/// offers only the variants it demonstrates, so the rest stay unreachable.
#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_buttons, no_json)]
pub(crate) enum DemoToggle {
    Disabled,
    Loading,
    Selected,
    Compact,
    Shadow,
    SizeXSmall,
    SizeSmall,
    SizeMedium,
    SizeLarge,
}

impl DemoToggle {
    /// The size this toggle selects; size-only sections listen for these.
    pub(crate) fn size(&self) -> Option<Size> {
        match self {
            DemoToggle::SizeXSmall => Some(Size::XSmall),
            DemoToggle::SizeSmall => Some(Size::Small),
            DemoToggle::SizeMedium => Some(Size::Medium),
            DemoToggle::SizeLarge => Some(Size::Large),
            _ => None,
        }
    }
}

impl DemoState {
    /// Apply one shared toggle; returns whether render state changed. The
    /// `Shadow` variant flips the global theme flag instead, as upstream did.
    pub(crate) fn apply(&mut self, toggle: &DemoToggle, window: &mut Window, cx: &mut App) -> bool {
        match toggle {
            DemoToggle::Shadow => {
                gpui_kit::component::Theme::update(cx, |theme| theme.shadow = !theme.shadow);
                window.refresh();
                false
            }
            DemoToggle::Disabled => {
                self.disabled = !self.disabled;
                true
            }
            DemoToggle::Loading => {
                self.loading = !self.loading;
                true
            }
            DemoToggle::Selected => {
                self.selected = !self.selected;
                true
            }
            DemoToggle::Compact => {
                self.compact = !self.compact;
                true
            }
            DemoToggle::SizeXSmall => {
                self.size = Size::XSmall;
                true
            }
            DemoToggle::SizeSmall => {
                self.size = Size::Small;
                true
            }
            DemoToggle::SizeMedium => {
                self.size = Size::Medium;
                true
            }
            DemoToggle::SizeLarge => {
                self.size = Size::Large;
                true
            }
        }
    }
}

pub(crate) fn size_label(size: Size) -> &'static str {
    match size {
        Size::XSmall => "XSmall",
        Size::Small => "Small",
        Size::Medium => "Medium",
        Size::Large => "Large",
        Size::Size(_) => "Custom",
    }
}

fn add_size_items(menu: PopupMenu, size: Size) -> PopupMenu {
    menu.menu_with_check(
        "XSmall",
        size == Size::XSmall,
        Box::new(DemoToggle::SizeXSmall),
    )
    .menu_with_check(
        "Small",
        size == Size::Small,
        Box::new(DemoToggle::SizeSmall),
    )
    .menu_with_check(
        "Medium",
        size == Size::Medium,
        Box::new(DemoToggle::SizeMedium),
    )
    .menu_with_check(
        "Large",
        size == Size::Large,
        Box::new(DemoToggle::SizeLarge),
    )
}

/// The size switcher every story toolbar carried, on the published split
/// button instead of the story crate's custom Popover trigger.
pub(crate) fn size_dropdown(id: &'static str, size: Size) -> DropdownButton {
    DropdownButton::new(id)
        .button(
            Button::new(SharedString::from(format!("{id}-trigger")))
                .label(format!("Size: {}", size_label(size))),
        )
        .dropdown_menu(move |menu, _, _| add_size_items(menu, size))
}

/// The `Options` menu carrying the shared demo knobs.
pub(crate) fn options_dropdown(id: &'static str, demo: &DemoState) -> DropdownButton {
    let disabled = demo.disabled;
    let loading = demo.loading;
    let selected = demo.selected;
    let compact = demo.compact;
    DropdownButton::new(id)
        .button(Button::new(SharedString::from(format!("{id}-trigger"))).label("Options"))
        .dropdown_menu(move |menu, _, cx| {
            menu.menu_with_check("Disabled", disabled, Box::new(DemoToggle::Disabled))
                .menu_with_check("Loading", loading, Box::new(DemoToggle::Loading))
                .menu_with_check("Selected", selected, Box::new(DemoToggle::Selected))
                .menu_with_check("Compact", compact, Box::new(DemoToggle::Compact))
                .separator()
                .menu_with_check("Shadow", cx.theme().shadow, Box::new(DemoToggle::Shadow))
        })
}

/// Toolbar row for a section, right-aligned like the upstream
/// `story_toolbar_group()`.
pub(crate) fn demo_toolbar(children: Vec<AnyElement>) -> Div {
    h_flex().w_full().justify_end().gap_1().children(children)
}

/// One titled demo box, ported from the story crate's `section()` (a
/// GroupBox with a title, optional description, and centered content).
#[derive(IntoElement)]
pub(crate) struct DemoSection {
    id: SharedString,
    title: &'static str,
    description: Option<&'static str>,
    base: Div,
    children: Vec<AnyElement>,
}

pub(crate) fn section(id: &'static str, title: &'static str) -> DemoSection {
    DemoSection {
        id: id.into(),
        title,
        description: None,
        base: h_flex()
            .w_full()
            .flex_wrap()
            .justify_center()
            .items_center()
            .gap_4(),
        children: vec![],
    }
}

impl DemoSection {
    pub(crate) fn description(mut self, description: &'static str) -> Self {
        self.description = Some(description);
        self
    }
}

impl ParentElement for DemoSection {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl Styled for DemoSection {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl RenderOnce for DemoSection {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        GroupBox::new()
            .id(self.id)
            .outline()
            .mb_6()
            .title(
                h_flex()
                    .justify_between()
                    .items_start()
                    .w_full()
                    .gap_4()
                    .child(
                        v_flex()
                            .min_w_0()
                            .flex_1()
                            .gap_1()
                            .child(div().font_weight(FontWeight::MEDIUM).child(self.title))
                            .when_some(self.description, |this, description| {
                                this.child(
                                    div()
                                        .text_xs()
                                        .text_color(cx.theme().muted_foreground)
                                        .child(description),
                                )
                            }),
                    ),
            )
            .content_style(
                StyleRefinement::default()
                    .rounded(cx.theme().radius_lg)
                    .overflow_x_hidden()
                    .items_center()
                    .justify_center(),
            )
            .child(self.base.children(self.children))
    }
}
