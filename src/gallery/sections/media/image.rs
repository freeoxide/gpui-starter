//! Image section, ported from the upstream `ImageStory`.

use gpui_kit::StyledImage as _;
use gpui_kit::component::{ActiveTheme as _, v_flex};
use gpui_kit::*;

use crate::gallery::registry::GallerySection;

use super::demo::{preview_source, section};

const REMOTE_SVG: &str = "https://pub.lbkrs.com/files/202503/vEnnmgUM6bo362ya/sdk.svg";

pub struct ImageSection;

impl ImageSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self)
    }
}

/// The story's bordered preview frame, shared by both boxes.
fn preview_frame(child: impl IntoElement, cx: &App) -> Div {
    div()
        .w_full()
        .h(rems(11.25))
        .flex()
        .items_center()
        .justify_center()
        .rounded(cx.theme().radius_lg)
        .border_1()
        .border_color(cx.theme().border)
        .child(child)
}

impl Render for ImageSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let muted = cx.theme().muted_foreground;

        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .p_4()
            .child(
                section("image-embedded-svg", "Embedded SVG")
                    .description("Loads and renders an SVG from bytes embedded in the binary.")
                    .w(rems(30.))
                    .child(preview_frame(img(preview_source()).h_24(), cx)),
            )
            .child(
                section("image-remote-url", "Remote URL")
                    .description(
                        "On native the default HTTP client fails every request, so the fallback \
                         view stays. The web build fetches through the browser instead.",
                    )
                    .w(rems(30.))
                    .child(preview_frame(
                        img(REMOTE_SVG)
                            .id("image-remote-svg")
                            .h_24()
                            .with_loading(move || {
                                div()
                                    .text_sm()
                                    .text_color(muted)
                                    .child("Loading image…")
                                    .into_any_element()
                            })
                            .with_fallback(move || {
                                div()
                                    .text_sm()
                                    .text_color(muted)
                                    .child("Image unavailable")
                                    .into_any_element()
                            }),
                        cx,
                    )),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "image",
        "Image",
        "Image and SVG image supported.",
        ImageSection::view(window, cx),
    ));
}
