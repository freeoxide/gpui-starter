//! Scaffolding shared by every gallery area: the titled demo box (ported
//! from the story crate's `section()` helper), the section toolbar, the
//! size switcher, and the punt page for examples this app does not port.

use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Size, StyledExt as _,
    button::{Button, DropdownButton},
    group_box::{GroupBox, GroupBoxVariants as _},
    h_flex,
    text::TextView,
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

/// Toolbar row for a section, right-aligned like the upstream story toolbar.
pub(crate) fn demo_toolbar(children: Vec<AnyElement>) -> Div {
    h_flex().w_full().justify_end().gap_1().children(children)
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

/// The four standard toolbar sizes every area's toggle set carries. The
/// action type stays per-area so a section never reacts to another area's
/// toolbar.
pub(crate) trait SizeToggle: Sized {
    fn xsmall() -> Self;
    fn small() -> Self;
    fn medium() -> Self;
    fn large() -> Self;
}

/// The size switcher the story toolbars carried, on the published split
/// button instead of the story crate's custom Popover trigger.
pub(crate) fn size_dropdown<T: SizeToggle + Action>(
    id: &'static str,
    size: Size,
) -> DropdownButton {
    DropdownButton::new(id)
        .button(Button::new(format!("{id}-trigger")).label(format!("Size: {}", size_label(size))))
        .dropdown_menu(move |menu, _, _| {
            menu.menu_with_check("XSmall", size == Size::XSmall, Box::new(T::xsmall()))
                .menu_with_check("Small", size == Size::Small, Box::new(T::small()))
                .menu_with_check("Medium", size == Size::Medium, Box::new(T::medium()))
                .menu_with_check("Large", size == Size::Large, Box::new(T::large()))
        })
}

/// One titled demo box, ported from the story crate's `section()` helper;
/// the `sub_title` slot rides the title row.
#[derive(IntoElement)]
pub(crate) struct DemoSection {
    id: SharedString,
    title: &'static str,
    description: Option<&'static str>,
    sub_title: Vec<AnyElement>,
    base: Div,
    children: Vec<AnyElement>,
}

pub(crate) fn section(id: &'static str, title: &'static str) -> DemoSection {
    DemoSection {
        id: id.into(),
        title,
        description: None,
        sub_title: vec![],
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

    pub(crate) fn sub_title(mut self, sub_title: impl IntoElement) -> Self {
        self.sub_title.push(sub_title.into_any_element());
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
                    )
                    .children(self.sub_title),
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

/// The section a story renders when the upstream example is deliberately
/// not ported: its icon and lead, the code shape it would need, and the
/// notes that explain the punt.
pub(crate) fn punt_page(
    id: &'static str,
    title: &'static str,
    description: &'static str,
    icon: IconName,
    lead: &'static str,
    body: &'static str,
    code_id: &'static str,
    code_shape: &'static str,
    notes: &[&'static str],
    cx: &App,
) -> impl IntoElement {
    let muted = cx.theme().muted_foreground;
    let paragraph = |line: &'static str| div().text_sm().text_color(muted).child(line);

    v_flex().w_full().items_center().gap_6().p_4().child(
        section(id, title)
            .description(description)
            .w(rems(30.))
            .v_flex()
            .items_start()
            .gap_3()
            .child(
                h_flex()
                    .gap_3()
                    .child(Icon::new(icon).size_6())
                    .child(div().text_sm().child(lead)),
            )
            .child(paragraph(body))
            .child(
                v_flex()
                    .w_full()
                    .gap_2()
                    .child(TextView::markdown(code_id, code_shape)),
            )
            .children(notes.iter().copied().map(paragraph)),
    )
}
