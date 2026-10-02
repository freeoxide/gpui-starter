//! HoverCard section, ported from the upstream `HoverCardStory`.

use std::time::Duration;

use gpui_kit::component::{
    ActiveTheme as _, StyledExt as _, avatar::Avatar, button::Button, h_flex,
    hover_card::HoverCard, link::Link, v_flex,
};
use gpui_kit::*;

use crate::gallery::registry::GallerySection;

use super::demo::section;

pub struct HoverCardSection;

impl HoverCardSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self)
    }
}

impl Render for HoverCardSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .p_4()
            .child(
                section("hover-card-default", "Default")
                    .description("Shows supporting information without changing the current view.")
                    .w(rems(32.5))
                    .child(
                        HoverCard::new("hover-card-basic")
                            .trigger(
                                div()
                                    .child("Hover over me")
                                    .text_color(cx.theme().primary)
                                    .cursor_pointer()
                                    .text_sm(),
                            )
                            .child(
                                v_flex()
                                    .gap_1()
                                    .w(rems(28.125))
                                    .child(
                                        div()
                                            .child("This is a hover card")
                                            .font_semibold()
                                            .text_sm(),
                                    )
                                    .child(
                                        div()
                                            .child(
                                                "You can display rich content when hovering over a trigger element.",
                                            )
                                            .text_color(cx.theme().muted_foreground)
                                            .text_sm(),
                                    ),
                            ),
                    ),
            )
            .child(
                section("hover-card-rich-content", "Rich Content")
                    .description("Cards can contain avatars, typography, and structured details.")
                    .w(rems(32.5))
                    .child(
                        h_flex()
                            .child("Hover over ")
                            .child(
                                HoverCard::new("hover-card-user-profile")
                                    .trigger(
                                        Link::new("hover-card-user-profile-link").child("@huacnlee"),
                                    )
                                    .content(|_, _, cx| {
                                        h_flex()
                                            .w_80()
                                            .gap_3()
                                            .items_start()
                                            .child(
                                                Avatar::new().src(
                                                    "https://avatars.githubusercontent.com/u/5518?s=64",
                                                ),
                                            )
                                            .child(
                                                v_flex()
                                                    .gap_1()
                                                    .line_height(relative(1.))
                                                    .child(
                                                        div().child("Jason Lee").font_semibold(),
                                                    )
                                                    .child(
                                                        div()
                                                            .child("@huacnlee")
                                                            .text_color(cx.theme().link)
                                                            .text_sm(),
                                                    )
                                                    .child(
                                                        div()
                                                            .mt_1()
                                                            .child("The author of GPUI Kit."),
                                                    ),
                                            )
                                    }),
                            )
                            .child(" to see their profile"),
                    ),
            )
            .child(
                section("hover-card-timing", "Timing")
                    .description("Open and close delays can match the interaction context.")
                    .w(rems(32.5))
                    .child(
                        h_flex()
                            .gap_4()
                            .child(
                                HoverCard::new("hover-card-fast-open")
                                    .open_delay(Duration::from_millis(200))
                                    .close_delay(Duration::from_millis(100))
                                    .trigger(
                                        Button::new("hover-card-fast")
                                            .label("Fast Open (200ms)")
                                            .outline(),
                                    )
                                    .child(
                                        div().child("This hover card opens after 200ms").text_sm(),
                                    ),
                            )
                            .child(
                                HoverCard::new("hover-card-slow-open")
                                    .open_delay(Duration::from_secs(1))
                                    .close_delay(Duration::from_secs_f32(0.5))
                                    .trigger(
                                        Button::new("hover-card-slow")
                                            .label("Slow Open (1000ms)")
                                            .outline(),
                                    )
                                    .child(
                                        div().child("This hover card opens after 1000ms").text_sm(),
                                    ),
                            ),
                    ),
            )
            .child(
                section("hover-card-position", "Position")
                    .description("Content can anchor to each side of its trigger.")
                    .w(rems(40.))
                    .child(
                        v_flex()
                            .gap_4()
                            .items_center()
                            .justify_center()
                            .child(
                                h_flex()
                                    .gap_4()
                                    .child(
                                        HoverCard::new("hover-card-anchor-top-left")
                                            .anchor(Anchor::TopLeft)
                                            .trigger(
                                                Button::new("hover-card-tl")
                                                    .label("Top Left")
                                                    .outline(),
                                            )
                                            .child(
                                                div().child("Positioned at Top Left").text_sm(),
                                            ),
                                    )
                                    .child(
                                        HoverCard::new("hover-card-anchor-top-center")
                                            .anchor(Anchor::TopCenter)
                                            .trigger(
                                                Button::new("hover-card-tc")
                                                    .label("Top Center")
                                                    .outline(),
                                            )
                                            .child(
                                                div().child("Positioned at Top Center").text_sm(),
                                            ),
                                    )
                                    .child(
                                        HoverCard::new("hover-card-anchor-top-right")
                                            .anchor(Anchor::TopRight)
                                            .trigger(
                                                Button::new("hover-card-tr")
                                                    .label("Top Right")
                                                    .outline(),
                                            )
                                            .child(
                                                div().child("Positioned at Top Right").text_sm(),
                                            ),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .gap_4()
                                    .child(
                                        HoverCard::new("hover-card-anchor-bottom-left")
                                            .anchor(Anchor::BottomLeft)
                                            .trigger(
                                                Button::new("hover-card-bl")
                                                    .label("Bottom Left")
                                                    .outline(),
                                            )
                                            .child(
                                                div().child("Positioned at Bottom Left").text_sm(),
                                            ),
                                    )
                                    .child(
                                        HoverCard::new("hover-card-anchor-bottom-center")
                                            .anchor(Anchor::BottomCenter)
                                            .trigger(
                                                Button::new("hover-card-bc")
                                                    .label("Bottom Center")
                                                    .outline(),
                                            )
                                            .child(
                                                div()
                                                    .child("Positioned at Bottom Center")
                                                    .text_sm(),
                                            ),
                                    )
                                    .child(
                                        HoverCard::new("hover-card-anchor-bottom-right")
                                            .anchor(Anchor::BottomRight)
                                            .trigger(
                                                Button::new("hover-card-br")
                                                    .label("Bottom Right")
                                                    .outline(),
                                            )
                                            .child(
                                                div()
                                                    .child("Positioned at Bottom Right")
                                                    .text_sm(),
                                            ),
                                    ),
                            ),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "hover-card",
        "Hover Card",
        "Display content when hovering over a trigger element, with configurable delays.",
        HoverCardSection::view(window, cx),
    ));
}
