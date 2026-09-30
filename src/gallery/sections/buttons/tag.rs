//! Tag section, ported from the upstream `TagStory`.

use gpui_kit::component::{ColorName, Sizable as _, Size, h_flex, tag::Tag, v_flex};
use gpui_kit::{
    App, AppContext, Context, Entity, InteractiveElement, IntoElement, ParentElement, Render,
    Styled, Window, px, rems,
};

use crate::gallery::registry::GallerySection;

use super::demo::{DemoToggle, demo_toolbar, section, size_dropdown};

pub struct TagSection {
    size: Size,
}

impl TagSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self { size: Size::Medium })
    }
}

impl Render for TagSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let size = self.size;
        let row = || h_flex().gap_2();

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
                size_dropdown("tag-size", size).into_any_element(),
            ]))
            .child(
                section("tag-default", "Default").child(
                    row().children([
                        Tag::primary().with_size(size).child("Tag"),
                        Tag::secondary().with_size(size).child("Secondary"),
                        Tag::danger().with_size(size).child("Danger"),
                        Tag::success().with_size(size).child("Success"),
                        Tag::warning().with_size(size).child("Warning"),
                        Tag::info().with_size(size).child("Info"),
                        Tag::custom(
                            ColorName::Indigo.scale(500),
                            ColorName::Indigo.scale(50),
                            ColorName::Indigo.scale(500),
                        )
                        .with_size(size)
                        .child("Custom"),
                    ]),
                ),
            )
            .child(
                section("tag-outline", "Outline").child(
                    row().children([
                        Tag::primary().with_size(size).outline().child("Tag"),
                        Tag::secondary()
                            .with_size(size)
                            .outline()
                            .child("Secondary"),
                        Tag::danger().with_size(size).outline().child("Danger"),
                        Tag::success().with_size(size).outline().child("Success"),
                        Tag::warning().with_size(size).outline().child("Warning"),
                        Tag::info().with_size(size).outline().child("Info"),
                        Tag::custom(
                            ColorName::Indigo.scale(500),
                            ColorName::Indigo.scale(500),
                            ColorName::Indigo.scale(500),
                        )
                        .with_size(size)
                        .outline()
                        .child("Custom"),
                    ]),
                ),
            )
            .child(
                section("tag-rounded", "Rounded").child(
                    row().children([
                        Tag::primary().with_size(size).rounded_full().child("Tag"),
                        Tag::secondary()
                            .with_size(size)
                            .rounded_full()
                            .child("Secondary"),
                        Tag::danger().with_size(size).rounded_full().child("Danger"),
                        Tag::success()
                            .with_size(size)
                            .rounded_full()
                            .child("Success"),
                        Tag::warning()
                            .with_size(size)
                            .rounded_full()
                            .child("Warning"),
                        Tag::info().with_size(size).rounded_full().child("Info"),
                    ]),
                ),
            )
            .child(
                section("tag-square", "Square").child(
                    row().children([
                        Tag::primary().with_size(size).rounded(px(0.)).child("Tag"),
                        Tag::secondary()
                            .with_size(size)
                            .rounded(px(0.))
                            .child("Secondary"),
                        Tag::danger()
                            .with_size(size)
                            .rounded(px(0.))
                            .child("Danger"),
                        Tag::success()
                            .with_size(size)
                            .rounded(px(0.))
                            .child("Success"),
                        Tag::warning()
                            .with_size(size)
                            .rounded(px(0.))
                            .child("Warning"),
                        Tag::info().with_size(size).rounded(px(0.)).child("Info"),
                    ]),
                ),
            )
            .child(
                section("tag-colors", "Colors").w(rems(40.)).child(
                    h_flex().w_full().gap_2().flex_wrap().children(
                        ColorName::all()
                            .into_iter()
                            .filter(|color| *color != ColorName::Gray)
                            .map(|color| {
                                Tag::color(color).with_size(size).child(color.to_string())
                            }),
                    ),
                ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "tag",
        "Tag",
        "A short item that can be used to categorize or label content.",
        TagSection::view(window, cx),
    ));
}
