//! Rating section, ported from the upstream `RatingStory`.

use gpui_kit::component::{ActiveTheme as _, Size, rating::Rating, v_flex};
use gpui_kit::{
    App, AppContext, Context, Entity, InteractiveElement, IntoElement, ParentElement, Render,
    Styled, Window, rems,
};

use crate::gallery::registry::GallerySection;

use super::demo::{DemoToggle, demo_toolbar, section, size_dropdown};

pub struct RatingSection {
    size: Size,
    value: usize,
}

impl RatingSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self {
            size: Size::default(),
            value: 3,
        })
    }
}

impl Render for RatingSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let size = self.size;

        v_flex()
            .w_full()
            .gap_6()
            .p_4()
            .on_action(cx.listener(|this, action: &DemoToggle, _, cx| {
                if let Some(size) = action.size() {
                    this.size = size;
                    cx.notify();
                }
            }))
            .child(demo_toolbar(vec![
                size_dropdown("rating-size", size).into_any_element(),
            ]))
            .child(
                section("rating-default", "Default")
                    .description("Select a value directly from the rating.")
                    .w_128()
                    .child(
                        v_flex()
                            .w_full()
                            .gap_3()
                            .justify_center()
                            .items_center()
                            .child(
                                Rating::new("rating-default-stars")
                                    .with_size(size)
                                    .value(self.value)
                                    .max(5)
                                    .on_click(cx.listener(|this, value: &usize, _, cx| {
                                        this.value = *value;
                                        cx.notify();
                                    })),
                            ),
                    ),
            )
            .child(
                section("rating-disabled", "Disabled").w(rems(30.)).child(
                    Rating::new("rating-disabled-stars")
                        .with_size(size)
                        .value(2)
                        .color(cx.theme().green)
                        .max(5)
                        .disabled(true),
                ),
            )
            .child(
                section("rating-color", "Color").w(rems(30.)).child(
                    Rating::new("rating-color-stars")
                        .with_size(size)
                        .value(self.value)
                        .color(cx.theme().green)
                        .max(5),
                ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "rating",
        "Rating",
        "A simple interactive star rating component.",
        RatingSection::view(window, cx),
    ));
}
