//! Separator section, ported from the upstream `SeparatorStory`.

use gpui_kit::component::{ActiveTheme as _, h_flex, label::Label, separator::Separator, v_flex};
use gpui_kit::*;

use crate::gallery::registry::GallerySection;

use super::demo::section;

const DESCRIPTION: &str = "GPUI Kit is a collection of Rust GUI components for building \
    fantastic cross-platform desktop applications with GPUI.";

pub struct SeparatorSection;

impl SeparatorSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self)
    }
}

impl Render for SeparatorSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .gap_6()
            .p_4()
            .child(
                section("separator-horizontal", "Horizontal")
                    .description(
                        "Separates stacked content, with optional labels and dashed rules.",
                    )
                    .w(rems(32.5))
                    .child(
                        v_flex()
                            .gap_4()
                            .w_full()
                            .mt_4()
                            .child(Separator::horizontal())
                            .child(Separator::horizontal().label("With Label"))
                            .child(Separator::horizontal_dashed())
                            .child(Separator::horizontal_dashed().label("Dashed With Label")),
                    ),
            )
            .child(
                section("separator-vertical", "Vertical")
                    .description("Separates actions or values arranged in a row.")
                    .w(rems(32.5))
                    .child(
                        h_flex()
                            .gap_4()
                            .h(rems(6.25))
                            .child(Separator::vertical())
                            .child(Separator::vertical().label("Solid"))
                            .child(Separator::vertical_dashed())
                            .child(Separator::vertical_dashed().label("Dashed")),
                    ),
            )
            .child(
                section("separator-in-context", "In Context")
                    .description("Horizontal and vertical rules can structure compact content.")
                    .w(rems(32.5))
                    .child(
                        v_flex()
                            .gap_y_4()
                            .child(
                                v_flex().gap_y_2().child("Hello GPUI Kit").child(
                                    Label::new(DESCRIPTION)
                                        .text_color(cx.theme().muted_foreground)
                                        .text_sm(),
                                ),
                            )
                            .child(Separator::horizontal())
                            .child(
                                h_flex()
                                    .gap_x_4()
                                    .child("Docs")
                                    .child(Separator::vertical().dashed())
                                    .child("GitHub")
                                    .child(Separator::vertical().dashed())
                                    .child("Source"),
                            ),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "separator",
        "Separator",
        "A separator that can be either vertical or horizontal.",
        SeparatorSection::view(window, cx),
    ));
}
