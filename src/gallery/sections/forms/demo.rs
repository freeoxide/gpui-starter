//! Shared scaffolding for the form-area sections: the titled demo box
//! (ported from the story crate's `section()` helper), the toolbar row, and
//! the size switcher the Form and Questionnaire stories expose.

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

/// Shared check-menu toggles; the size variants drive the shared size
/// switcher and the Form section also listens for its two layout toggles.
#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_forms, no_json)]
pub(crate) enum FormToggle {
    SizeXSmall,
    SizeSmall,
    SizeMedium,
    SizeLarge,
    Horizontal,
    MultipleColumns,
}

impl FormToggle {
    /// The size this toggle selects; the size switcher entries map here.
    pub(crate) fn size(&self) -> Option<Size> {
        match self {
            FormToggle::SizeXSmall => Some(Size::XSmall),
            FormToggle::SizeSmall => Some(Size::Small),
            FormToggle::SizeMedium => Some(Size::Medium),
            FormToggle::SizeLarge => Some(Size::Large),
            _ => None,
        }
    }
}

fn size_label(size: Size) -> &'static str {
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
        Box::new(FormToggle::SizeXSmall),
    )
    .menu_with_check(
        "Small",
        size == Size::Small,
        Box::new(FormToggle::SizeSmall),
    )
    .menu_with_check(
        "Medium",
        size == Size::Medium,
        Box::new(FormToggle::SizeMedium),
    )
    .menu_with_check(
        "Large",
        size == Size::Large,
        Box::new(FormToggle::SizeLarge),
    )
}

/// The size switcher the story toolbars carried, on the published split
/// button instead of the story crate's custom Popover trigger.
pub(crate) fn size_dropdown(id: &'static str, size: Size) -> DropdownButton {
    DropdownButton::new(id)
        .button(
            Button::new(SharedString::from(format!("{id}-trigger")))
                .label(format!("Size: {}", size_label(size))),
        )
        .dropdown_menu(move |menu, _, _| add_size_items(menu, size))
}

/// Toolbar row for a section, right-aligned like the upstream story toolbar.
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
