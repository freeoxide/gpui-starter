//! Marker section, ported from the upstream `MarkerStory`.

use std::time::Duration;

use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _,
    button::{Button, ButtonVariants as _},
    link::Link,
    marker::{
        Marker, MarkerAlignment, MarkerContent, MarkerIcon, MarkerLoadingStyle, MarkerVariant,
    },
    shimmer::{ShimmerStyle, ShimmerText},
    spinner::Spinner,
    v_flex,
};
use gpui_kit::*;

use crate::gallery::registry::GallerySection;

use super::demo::section;

pub struct MarkerSection;

impl MarkerSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self)
    }
}

impl Render for MarkerSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .p_4()
            .child(
                section("marker-variants", "Variants")
                    .description("Choose a plain row, a centered separator, or a bordered boundary.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_4()
                    .child(Marker::new().content(MarkerContent::new().child("Plain status update")))
                    .child(
                        Marker::new()
                            .with_variant(MarkerVariant::Separator)
                            .content(MarkerContent::new().child("Earlier messages")),
                    )
                    .child(
                        Marker::new()
                            .with_variant(MarkerVariant::Border)
                            .content(MarkerContent::new().child("Unread messages")),
                    ),
            )
            .child(
                section("marker-alignment", "Alignment")
                    .description(
                        "Center a system notice or trail a delivery state; Separator centers by default.",
                    )
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_3()
                    .child(Marker::new().content(MarkerContent::new().child("Leading by default")))
                    .child(
                        Marker::new()
                            .alignment(MarkerAlignment::Center)
                            .icon(MarkerIcon::new().child(Icon::new(IconName::Info)))
                            .content(MarkerContent::new().child("Messages are end-to-end encrypted")),
                    )
                    .child(
                        Marker::new()
                            .alignment(MarkerAlignment::Center)
                            .content(MarkerContent::new().child(
                                "The answer was stopped before it finished. Edit the question or ask again, and a long notice wraps around its center.",
                            )),
                    )
                    .child(
                        Marker::new()
                            .alignment(MarkerAlignment::Center)
                            .content(MarkerContent::new().child("The message could not be sent."))
                            .child(Button::new("marker-retry-send").text().small().label("Retry")),
                    )
                    .child(
                        Marker::new()
                            .alignment(MarkerAlignment::End)
                            .content(MarkerContent::new().child("Delivered")),
                    )
                    .child(
                        Marker::new()
                            .with_variant(MarkerVariant::Separator)
                            .alignment(MarkerAlignment::End)
                            .content(MarkerContent::new().child("Today")),
                    ),
            )
            .child(
                section("marker-status", "Status")
                    .description(
                        "Compose icons, spinners, and labels without a fixed status model.",
                    )
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_3()
                    .child(
                        Marker::new()
                            .text_color(cx.theme().success)
                            .icon(MarkerIcon::new().child(Icon::new(IconName::CircleCheck)))
                            .content(MarkerContent::new().child("Online")),
                    )
                    .child(
                        Marker::new()
                            .icon(MarkerIcon::new().child(Spinner::new().xsmall()))
                            .content(MarkerContent::new().child("Alice is typing…")),
                    )
                    .child(
                        Marker::new()
                            .icon(MarkerIcon::new().child(Icon::new(IconName::Bell)))
                            .content(MarkerContent::new().child("Unread notifications")),
                    )
                    .child(
                        Marker::new()
                            .text_color(cx.theme().danger)
                            .icon(MarkerIcon::new().child(Icon::new(IconName::Info)))
                            .content(MarkerContent::new().child("Message could not be delivered")),
                    ),
            )
            .child(
                section("marker-with-icon", "With icon")
                    .description("Icons can communicate sender activity, notices, and saved items.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_3()
                    .child(
                        Marker::new()
                            .icon(MarkerIcon::new().child(Icon::new(IconName::Info)))
                            .content(MarkerContent::new().child("Conversation details updated")),
                    )
                    .child(
                        Marker::new()
                            .icon(MarkerIcon::new().child(Icon::new(IconName::Star)))
                            .content(MarkerContent::new().child("Pinned for your team")),
                    )
                    .child(
                        Marker::new().content(MarkerContent::new().child("No icon is required")),
                    ),
            )
            .child(
                section("marker-loading-styles", "Loading styles")
                    .description("Choose a spinner or a sweeping, ChatGPT-style text shimmer.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_4()
                    .child(
                        Marker::new()
                            .loading(true)
                            .with_loading_style(MarkerLoadingStyle::Spinner)
                            .content(MarkerContent::new().text("shadcn/ui · Loading messages…")),
                    )
                    .child(
                        Marker::new()
                            .loading(true)
                            .with_loading_style(MarkerLoadingStyle::Shimmer)
                            .content(MarkerContent::new().text("ChatGPT · Thinking")),
                    )
                    .child(
                        Marker::new()
                            .loading(true)
                            .with_loading_style(MarkerLoadingStyle::Shimmer)
                            .icon(MarkerIcon::new().child(Icon::new(IconName::Info)))
                            .content(MarkerContent::new().text("正在探索 4 个文件…")),
                    )
                    .child(
                        Marker::new()
                            .loading(true)
                            .with_loading_style(MarkerLoadingStyle::Shimmer)
                            .with_shimmer_style(
                                ShimmerStyle::new()
                                    .duration(Duration::from_secs(3))
                                    .highlight_color(cx.theme().primary)
                                    .spread(0.45)
                                    .reverse(true),
                            )
                            .content(
                                MarkerContent::new().text("Custom color, width, and direction"),
                            ),
                    )
                    .child(
                        ShimmerText::new("Reusable shimmer without a Marker")
                            .text_color(cx.theme().muted_foreground),
                    ),
            )
            .child(
                section("marker-shimmer-settings", "Shimmer settings")
                    .description(
                        "Customize timing, highlight width, direction, and playback independently.",
                    )
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_3()
                    .child(
                        Marker::new()
                            .loading(true)
                            .with_loading_style(MarkerLoadingStyle::Shimmer)
                            .with_shimmer_style(
                                ShimmerStyle::new().duration(Duration::from_millis(900)),
                            )
                            .content(MarkerContent::new().text("Faster highlight sweep")),
                    )
                    .child(
                        Marker::new()
                            .loading(true)
                            .with_loading_style(MarkerLoadingStyle::Shimmer)
                            .with_shimmer_style(ShimmerStyle::new().spread(0.55))
                            .content(MarkerContent::new().text("Wider highlight band")),
                    )
                    .child(
                        Marker::new()
                            .loading(true)
                            .with_loading_style(MarkerLoadingStyle::Shimmer)
                            .with_shimmer_style(ShimmerStyle::new().reverse(true))
                            .content(MarkerContent::new().text("Right-to-left sweep")),
                    )
                    .child(
                        Marker::new()
                            .loading(true)
                            .with_loading_style(MarkerLoadingStyle::Shimmer)
                            .with_shimmer_style(
                                ShimmerStyle::new().highlight_color(cx.theme().primary),
                            )
                            .content(MarkerContent::new().text("Semantic primary highlight")),
                    )
                    .child(
                        Marker::new()
                            .loading(true)
                            .with_loading_style(MarkerLoadingStyle::Shimmer)
                            .with_shimmer_style(ShimmerStyle::new().once(true))
                            .content(MarkerContent::new().text("Play the highlight once")),
                    )
                    .child(
                        Marker::new()
                            .loading(false)
                            .with_loading_style(MarkerLoadingStyle::Shimmer)
                            .content(MarkerContent::new().text("Loading is disabled")),
                    ),
            )
            .child(
                section("marker-separator", "Separator")
                    .description("Place a conversation boundary between two semantic lines.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_4()
                    .child(
                        Marker::new()
                            .with_variant(MarkerVariant::Separator)
                            .content(MarkerContent::new().child("Today")),
                    )
                    .child(
                        Marker::new()
                            .with_variant(MarkerVariant::Separator)
                            .separator_style(
                                StyleRefinement::default().bg(cx.theme().primary.opacity(0.35)),
                            )
                            .icon(MarkerIcon::new().child(Icon::new(IconName::Star)))
                            .content(MarkerContent::new().child("Pinned messages")),
                    ),
            )
            .child(
                section("marker-border", "Border")
                    .description("Use a bottom edge for an unread or section boundary.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_3()
                    .child(
                        Marker::new()
                            .with_variant(MarkerVariant::Border)
                            .icon(MarkerIcon::new().child(Icon::new(IconName::Info)))
                            .content(MarkerContent::new().child("3 unread messages")),
                    )
                    .child(
                        Marker::new()
                            .with_variant(MarkerVariant::Border)
                            .border_color(cx.theme().primary.opacity(0.4))
                            .content(
                                MarkerContent::new().child("New replies since your last visit"),
                            ),
                    ),
            )
            .child(
                section("marker-links-and-buttons", "Links and buttons")
                    .description(
                        "Keep external destinations and in-app commands semantically distinct.",
                    )
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_3()
                    .child(
                        Marker::new()
                            .icon(MarkerIcon::new().child(Icon::new(IconName::Info)))
                            .content(
                                MarkerContent::new().child(
                                    Link::new("marker-documentation-link")
                                        .href("https://gpui-kit.com/")
                                        .child("Open the component documentation"),
                                ),
                            ),
                    )
                    .child(
                        Marker::new()
                            .icon(MarkerIcon::new().child(Icon::new(IconName::Star)))
                            .content(MarkerContent::new().child("A saved draft is ready"))
                            .child(
                                Button::new("marker-open-draft")
                                    .ghost()
                                    .small()
                                    .label("Open draft"),
                            ),
                    ),
            )
            .child(
                section("marker-custom-style", "Custom style")
                    .description("Caller refinements can replace spacing, color, and surface.")
                    .max_w(rems(42.5))
                    .child(
                        Marker::new()
                            .px_3()
                            .py_2()
                            .rounded(cx.theme().radius)
                            .bg(cx.theme().accent)
                            .text_color(cx.theme().accent_foreground)
                            .icon(MarkerIcon::new().child(Icon::new(IconName::Star)))
                            .content(MarkerContent::new().child("Pinned message")),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "marker",
        "Marker",
        "A compact row for conversation status, notifications, and separators.",
        MarkerSection::view(window, cx),
    ));
}
