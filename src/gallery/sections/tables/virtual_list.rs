//! Virtual List section, ported from the upstream `VirtualListStory`: a
//! large dataset rendered through `v_virtual_list` with configurable axis
//! scrollbars and programmatic scrolling.

use std::{ops::Range, rc::Rc};

use gpui_kit::component::{
    ActiveTheme as _, StyledExt as _, VirtualListScrollHandle,
    button::{Button, DropdownButton},
    h_flex,
    scroll::{ScrollableElement as _, ScrollbarAxis},
    v_flex, v_virtual_list,
};
use gpui_kit::*;
use serde::Deserialize;

use crate::gallery::registry::GallerySection;

use super::demo::{demo_toolbar, section};

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_tables, no_json)]
enum VirtualListDataset {
    Standard,
    Wide,
    Stress,
    Short,
}

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_tables, no_json)]
enum VirtualListAxis {
    Both,
    Vertical,
    Horizontal,
}

impl VirtualListAxis {
    fn axis(&self) -> ScrollbarAxis {
        match self {
            VirtualListAxis::Both => ScrollbarAxis::Both,
            VirtualListAxis::Vertical => ScrollbarAxis::Vertical,
            VirtualListAxis::Horizontal => ScrollbarAxis::Horizontal,
        }
    }
}

// The virtualized cell geometry; the render closure sizes every row from it.
const ITEM_SIZE: Size<Pixels> = size(px(100.), px(30.));

pub struct VirtualListSection {
    scroll_handle: VirtualListScrollHandle,
    items: Vec<String>,
    item_sizes: Rc<Vec<Size<Pixels>>>,
    columns_count: usize,
    axis: ScrollbarAxis,
    size_mode: usize,
    visible_range: Range<usize>,
}

impl VirtualListSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| {
            let items = (0..5000).map(|i| format!("Item {}", i)).collect::<Vec<_>>();
            let item_sizes = items.iter().map(|_| ITEM_SIZE).collect::<Vec<_>>();

            Self {
                scroll_handle: VirtualListScrollHandle::new(),
                items,
                item_sizes: Rc::new(item_sizes),
                columns_count: 100,
                axis: ScrollbarAxis::Both,
                size_mode: 0,
                visible_range: (0..0),
            }
        })
    }

    fn change_dataset(&mut self, n: usize, cx: &mut Context<Self>) {
        self.size_mode = n;
        if n == 0 {
            self.items = (0..5000).map(|i| format!("Item {}", i)).collect::<Vec<_>>();
            self.columns_count = 30;
        } else if n == 1 {
            self.items = (0..100).map(|i| format!("Item {}", i)).collect::<Vec<_>>();
            self.columns_count = 100;
        } else if n == 2 {
            self.items = (0..500000)
                .map(|i| format!("Item {}", i))
                .collect::<Vec<_>>();
            self.columns_count = 100;
        } else {
            self.items = (0..5).map(|i| format!("Item {}", i)).collect::<Vec<_>>();
            self.columns_count = 10;
        }

        self.item_sizes = Rc::new(self.items.iter().map(|_| ITEM_SIZE).collect());
        cx.notify();
    }

    fn change_axis(&mut self, axis: ScrollbarAxis, cx: &mut Context<Self>) {
        self.axis = axis;
        cx.notify();
    }

    fn render_buttons(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let size_mode = self.size_mode;
        let axis = self.axis;

        v_flex()
            .gap_2()
            .child(demo_toolbar(vec![
                DropdownButton::new("virtual-list-dataset")
                    .button(Button::new("virtual-list-dataset-trigger").label(format!(
                        "Dataset: {}",
                        ["Standard", "Wide", "Stress", "Short"][size_mode]
                    )))
                    .dropdown_menu(move |menu, _, _| {
                        ["Standard", "Wide", "Stress", "Short"]
                            .into_iter()
                            .enumerate()
                            .fold(menu, |menu, (index, label)| {
                                menu.menu_with_check(
                                    label,
                                    size_mode == index,
                                    Box::new(match index {
                                        0 => VirtualListDataset::Standard,
                                        1 => VirtualListDataset::Wide,
                                        2 => VirtualListDataset::Stress,
                                        _ => VirtualListDataset::Short,
                                    }),
                                )
                            })
                    })
                    .into_any_element(),
                DropdownButton::new("virtual-list-axis")
                    .button(
                        Button::new("virtual-list-axis-trigger")
                            .label(format!("Axis: {}", axis_label(axis))),
                    )
                    .dropdown_menu(move |menu, _, _| {
                        menu.menu_with_check(
                            "Both",
                            axis.is_both(),
                            Box::new(VirtualListAxis::Both),
                        )
                        .menu_with_check(
                            "Vertical",
                            axis.is_vertical(),
                            Box::new(VirtualListAxis::Vertical),
                        )
                        .menu_with_check(
                            "Horizontal",
                            axis.is_horizontal(),
                            Box::new(VirtualListAxis::Horizontal),
                        )
                    })
                    .into_any_element(),
                Button::new("virtual-list-scroll-top")
                    .label("Top")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.scroll_handle.scroll_to_item(0, ScrollStrategy::Top);
                        cx.notify();
                    }))
                    .into_any_element(),
                Button::new("virtual-list-scroll-row-50")
                    .label("Row 50")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.scroll_handle.scroll_to_item(50, ScrollStrategy::Top);
                        cx.notify();
                    }))
                    .into_any_element(),
                Button::new("virtual-list-scroll-center-25")
                    .label("Center 25")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.scroll_handle
                            .scroll_to_item(25, ScrollStrategy::Center);
                        cx.notify();
                    }))
                    .into_any_element(),
                Button::new("virtual-list-scroll-bottom")
                    .label("Bottom")
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.scroll_handle.scroll_to_bottom();
                        cx.notify();
                    }))
                    .into_any_element(),
            ]))
            .child(format!("Visible: {:?}", self.visible_range))
    }
}

fn axis_label(axis: ScrollbarAxis) -> &'static str {
    if axis.is_both() {
        "Both"
    } else if axis.is_vertical() {
        "Vertical"
    } else {
        "Horizontal"
    }
}

impl Render for VirtualListSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let columns_count = self.columns_count;

        fn render_item(cx: &App) -> Div {
            div()
                .flex()
                .h_full()
                .items_center()
                .justify_center()
                .text_sm()
                .w(ITEM_SIZE.width)
                .h(ITEM_SIZE.height)
                .bg(cx.theme().secondary)
        }

        v_flex()
            .on_action(cx.listener(|this, action: &VirtualListDataset, _, cx| {
                let mode = match action {
                    VirtualListDataset::Standard => 0,
                    VirtualListDataset::Wide => 1,
                    VirtualListDataset::Stress => 2,
                    VirtualListDataset::Short => 3,
                };
                this.change_dataset(mode, cx);
            }))
            .on_action(cx.listener(|this, action: &VirtualListAxis, _, cx| {
                this.change_axis(action.axis(), cx);
            }))
            .w_full()
            .items_center()
            .gap_6()
            .p_4()
            .child(self.render_buttons(cx))
            .child(
                section("virtual-list-grid", "Grid")
                    .description(
                        "Only the visible rows render, whatever the dataset size; the scrollbar \
                        axis follows the menu.",
                    )
                    .w_full()
                    .v_flex()
                    .child(
                        div().w_full().h(rems(30.)).child(
                            div().relative().size_full().child(
                                v_flex()
                                    .id("virtual-list-frame")
                                    .relative()
                                    .size_full()
                                    .child(
                                        v_virtual_list(
                                            cx.entity().clone(),
                                            "virtual-list-items",
                                            self.item_sizes.clone(),
                                            move |section, visible_range, _, cx| {
                                                section.visible_range = visible_range.clone();

                                                visible_range
                                                    .map(|ix| {
                                                        h_flex().gap_1().items_center().children(
                                                            (0..columns_count).map(|i| {
                                                                render_item(cx).child(if i == 0 {
                                                                    format!("row: {}", ix)
                                                                } else {
                                                                    format!("{}", i)
                                                                })
                                                            }),
                                                        )
                                                    })
                                                    .collect()
                                            },
                                        )
                                        .track_scroll(&self.scroll_handle)
                                        .p_4()
                                        .border_1()
                                        .border_color(cx.theme().border)
                                        .gap_1(),
                                    )
                                    .scrollbar(&self.scroll_handle, self.axis),
                            ),
                        ),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "virtual-list",
        "Virtual List",
        "A virtualized list rendering only visible items, with vertical, horizontal, or both \
        scrollbar axes.",
        VirtualListSection::view(window, cx),
    ));
}
