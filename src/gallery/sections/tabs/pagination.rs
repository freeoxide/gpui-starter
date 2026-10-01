//! Pagination section, ported from the upstream `PaginationStory`.

use gpui_kit::component::{Disableable as _, Sizable as _, Size, pagination::Pagination, v_flex};
use gpui_kit::*;

use crate::gallery::registry::GallerySection;

use super::demo::{DemoToggle, demo_toolbar, section, size_dropdown};

pub struct PaginationSection {
    basic_page: usize,
    many_pages_page: usize,
    compact_page: usize,
    size: Size,
}

impl PaginationSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self {
            basic_page: 5,
            many_pages_page: 1,
            compact_page: 3,
            size: Size::default(),
        })
    }
}

impl Render for PaginationSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let size = self.size;

        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .p_4()
            .on_action(cx.listener(|this, action: &DemoToggle, _, cx| {
                if let Some(size) = action.size() {
                    this.size = size;
                    cx.notify();
                }
            }))
            .child(demo_toolbar(vec![
                size_dropdown("pagination-size", size).into_any_element(),
            ]))
            .child(
                section("pagination-basic-box", "Default").child(
                    Pagination::new("pagination-basic")
                        .current_page(self.basic_page)
                        .total_pages(10)
                        .with_size(size)
                        .on_click(cx.listener(|this, page: &usize, _, cx| {
                            this.basic_page = *page;
                            cx.notify();
                        })),
                ),
            )
            .child(
                section("pagination-many-pages-box", "Visible Pages")
                    .description(
                        "Control how many page links remain visible in a larger result set.",
                    )
                    .child(
                        Pagination::new("pagination-many-pages")
                            .current_page(self.many_pages_page)
                            .total_pages(50)
                            .visible_pages(10)
                            .with_size(size)
                            .on_click(cx.listener(|this, page: &usize, _, cx| {
                                this.many_pages_page = *page;
                                cx.notify();
                            })),
                    ),
            )
            .child(
                section("pagination-compact-box", "Compact Style").child(
                    Pagination::new("pagination-compact")
                        .compact()
                        .current_page(self.compact_page)
                        .total_pages(10)
                        .with_size(size)
                        .on_click(cx.listener(|this, page: &usize, _, cx| {
                            this.compact_page = *page;
                            cx.notify();
                        })),
                ),
            )
            .child(
                section("pagination-disabled-box", "Disabled").child(
                    Pagination::new("pagination-disabled")
                        .current_page(4)
                        .total_pages(10)
                        .with_size(size)
                        .disabled(true)
                        .on_click(|_, _, _| {}),
                ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "pagination",
        "Pagination",
        "Pagination with page navigation, next and previous links.",
        PaginationSection::view(window, cx),
    ));
}
