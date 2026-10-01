//! Skeleton section, ported from the upstream `SkeletonStory`.

use gpui_kit::component::{ActiveTheme as _, ThemeStyled as _, h_flex, skeleton::Skeleton, v_flex};
use gpui_kit::{
    App, AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, Styled as _,
    Window, rems,
};

use crate::gallery::registry::GallerySection;

use super::demo::section;

pub struct SkeletonSection;

impl SkeletonSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self)
    }
}

impl Render for SkeletonSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_6()
            .w_full()
            .items_center()
            .p_4()
            .child(
                section("skeleton-text", "Text")
                    .description("Represents an avatar and text while profile content loads.")
                    .w(rems(22.5))
                    .child(
                        h_flex()
                            .w_full()
                            .gap_3()
                            .child(Skeleton::new().size_12().rounded_full_style(cx))
                            .child(
                                v_flex()
                                    .flex_1()
                                    .gap_2()
                                    .child(
                                        Skeleton::new().w_full().h_4().rounded(cx.theme().radius),
                                    )
                                    .child(
                                        Skeleton::new().w_2_3().h_4().rounded(cx.theme().radius),
                                    ),
                            ),
                    ),
            )
            .child(
                section("skeleton-card", "Card")
                    .description("Combines media and text placeholders in a content card.")
                    .w(rems(22.5))
                    .child(
                        v_flex()
                            .gap_2()
                            .child(
                                Skeleton::new()
                                    .w_full()
                                    .h(rems(11.25))
                                    .rounded(cx.theme().radius),
                            )
                            .child(
                                v_flex()
                                    .gap_2()
                                    .child(
                                        Skeleton::new().w_full().h_4().rounded(cx.theme().radius),
                                    )
                                    .child(
                                        Skeleton::new()
                                            .w(rems(12.5))
                                            .h_4()
                                            .rounded(cx.theme().radius),
                                    ),
                            ),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "skeleton",
        "Skeleton",
        "Use to show a placeholder while content is loading.",
        SkeletonSection::view(window, cx),
    ));
}
