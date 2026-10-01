//! Scrollbar section, ported from the upstream `ScrollbarStory`.

use std::rc::Rc;

use gpui_kit::component::{
    ActiveTheme as _,
    button::{Button, DropdownButton},
    menu::PopupMenu,
    scroll::ScrollableElement as _,
    v_flex,
};
use gpui_kit::*;

use crate::gallery::registry::GallerySection;

use super::demo::{DemoToggle, demo_toolbar};

const DATASETS: [&str; 4] = ["Standard", "Wide", "Stress", "Short"];

/// Fixed row height drives the uniform list's item measurement.
const ITEM_HEIGHT: Pixels = px(50.);

pub struct ScrollbarSection {
    items: Rc<Vec<String>>,
    size_mode: usize,
    scroll_handle: UniformListScrollHandle,
}

impl ScrollbarSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self {
            items: Rc::new((0..5000).map(|i| format!("Item {i}")).collect()),
            size_mode: 0,
            scroll_handle: UniformListScrollHandle::new(),
        })
    }

    fn change_dataset(&mut self, mode: usize, cx: &mut Context<Self>) {
        self.size_mode = mode;
        let count = match mode {
            0 => 5000,
            1 => 100,
            2 => 500_000,
            _ => 5,
        };
        self.items = Rc::new((0..count).map(|i| format!("Item {i}")).collect());
        cx.notify();
    }
}

fn dataset_action(mode: usize) -> Box<dyn Action> {
    match mode {
        0 => Box::new(DemoToggle::ScrollbarDatasetStandard),
        1 => Box::new(DemoToggle::ScrollbarDatasetWide),
        2 => Box::new(DemoToggle::ScrollbarDatasetStress),
        _ => Box::new(DemoToggle::ScrollbarDatasetShort),
    }
}

impl Render for ScrollbarSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let size_mode = self.size_mode;

        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .p_4()
            .on_action(cx.listener(|this, action: &DemoToggle, _, cx| {
                let mode = match action {
                    DemoToggle::ScrollbarDatasetStandard => 0,
                    DemoToggle::ScrollbarDatasetWide => 1,
                    DemoToggle::ScrollbarDatasetStress => 2,
                    DemoToggle::ScrollbarDatasetShort => 3,
                    _ => return,
                };
                this.change_dataset(mode, cx);
            }))
            .child(demo_toolbar(vec![
                DropdownButton::new("scrollbar-dataset")
                    .button(
                        Button::new("scrollbar-dataset-trigger")
                            .label(format!("Dataset: {}", DATASETS[size_mode])),
                    )
                    .dropdown_menu(move |menu: PopupMenu, _, _| {
                        DATASETS
                            .into_iter()
                            .enumerate()
                            .fold(menu, |menu, (mode, label)| {
                                menu.menu_with_check(label, mode == size_mode, dataset_action(mode))
                            })
                    })
                    .into_any_element(),
            ]))
            .child(
                // Fixed height: the pane scrolls, so the list region needs a
                // bounded frame for the scrollbar to measure against.
                div()
                    .relative()
                    .w_full()
                    .h(rems(30.))
                    .border_1()
                    .border_color(cx.theme().border)
                    .child(
                        uniform_list("scrollbar-list", self.items.len(), {
                            let items = self.items.clone();
                            move |visible_range, _, cx| {
                                let mut elements = Vec::with_capacity(visible_range.len());
                                for ix in visible_range {
                                    let item = &items[ix];
                                    elements.push(
                                        div()
                                            .h(ITEM_HEIGHT)
                                            .pt_1()
                                            .items_center()
                                            .justify_center()
                                            .text_sm()
                                            .child(
                                                div()
                                                    .p_2()
                                                    .bg(cx.theme().secondary)
                                                    .child(item.to_string()),
                                            ),
                                    );
                                }
                                elements
                            }
                        })
                        .py_1()
                        .px_3()
                        .size_full()
                        .track_scroll(&self.scroll_handle),
                    )
                    .vertical_scrollbar(&self.scroll_handle),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "scrollbar",
        "Scrollbar",
        "Add scrollbar to a scrollable element.",
        ScrollbarSection::view(window, cx),
    ));
}
