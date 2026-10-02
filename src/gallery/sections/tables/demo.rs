//! Shared scaffolding for the table-family sections: the titled demo box
//! (ported from the story crate's `section()` helper), the toolbar row, the
//! size switcher, and the deterministic stand-in for the `fake`/`rand` crates
//! the upstream stories generated their demo data with.

use std::sync::{LazyLock, Mutex};

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

/// Size knob shared by the Table and DataTable toolbars; the data table also
/// offers the 48px custom size its context menu sets.
#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_tables, no_json)]
pub(crate) enum DemoToggle {
    Size48Px,
    SizeXSmall,
    SizeSmall,
    SizeMedium,
    SizeLarge,
}

impl DemoToggle {
    /// The size this toggle selects.
    pub(crate) fn size(&self) -> Option<Size> {
        match self {
            DemoToggle::Size48Px => Some(Size::Size(px(48.))),
            DemoToggle::SizeXSmall => Some(Size::XSmall),
            DemoToggle::SizeSmall => Some(Size::Small),
            DemoToggle::SizeMedium => Some(Size::Medium),
            DemoToggle::SizeLarge => Some(Size::Large),
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

/// The size switcher the upstream story toolbar carried.
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

/// One titled demo box, ported from the story crate's `section()`; the
/// `sub_title` slot rides the title row (the tree's keyboard hint uses it).
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

// Neither `fake` nor `rand` is an app dependency, so the demo data comes from
// one shared deterministic sequence instead of those crates.
static RNG: LazyLock<Mutex<Lcg>> = LazyLock::new(|| Mutex::new(Lcg(0x4D59_5F53_4545_4401)));

struct Lcg(u64);

impl Lcg {
    fn next_f64(&mut self, min: f64, max: f64) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let unit = (self.0 >> 11) as f64 / (1u64 << 53) as f64;
        min + (max - min) * unit
    }

    // The unit draw is strictly below 1.0, so the truncation stays in range.
    fn next_usize(&mut self, min: usize, max: usize) -> usize {
        if max <= min {
            return min;
        }
        min + (self.next_f64(0., 1.) * (max - min) as f64) as usize
    }
}

/// A deterministic value in `min..max`, standing in for the `fake` crate's
/// ranged draws.
pub(crate) fn rng_f64(min: f64, max: f64) -> f64 {
    RNG.lock().unwrap().next_f64(min, max)
}

/// A deterministic value in `min..max`, standing in for `rand::random::<usize>()`.
pub(crate) fn rng_usize(min: usize, max: usize) -> usize {
    RNG.lock().unwrap().next_usize(min, max)
}

/// A deterministic pick from `items`, standing in for `SliceRandom::choose`.
pub(crate) fn rng_pick<T>(items: &[T]) -> Option<&T> {
    if items.is_empty() {
        None
    } else {
        items.get(rng_usize(0, items.len()))
    }
}
