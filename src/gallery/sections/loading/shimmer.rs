//! Shimmer section, ported from the upstream `ShimmerStory`.

use std::time::Duration;

use gpui_kit::component::{
    ActiveTheme as _, StyledExt as _,
    attachment::{
        Attachment, AttachmentContent, AttachmentDescription, AttachmentStatus, AttachmentTitle,
    },
    button::Button,
    h_flex,
    marker::{Marker, MarkerContent, MarkerLoadingStyle},
    shimmer::{ShimmerStyle, ShimmerText},
    v_flex,
};
use gpui_kit::{
    App, AppContext as _, Context, Entity, IntoElement, ParentElement as _, Render, Styled as _,
    Window, px, rems,
};

use crate::gallery::registry::GallerySection;

use super::demo::section;

pub struct ShimmerSection {
    replay_count: usize,
}

impl ShimmerSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self { replay_count: 0 })
    }
}

impl Render for ShimmerSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let replay_count = self.replay_count;
        let shared_style = ShimmerStyle::new()
            .duration(Duration::from_secs(3))
            .highlight_color(cx.theme().primary)
            .spread(0.42);

        v_flex()
            .gap_4()
            .w_full()
            .items_center()
            .p_4()
            .child(
                section("shimmer-default", "Default")
                    .description("A readable, theme-aware highlight crosses the existing text.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_2()
                    .child(ShimmerText::new("Thinking…"))
                    .child(
                        ShimmerText::new("Searching the current project…")
                            .text_color(cx.theme().muted_foreground),
                    ),
            )
            .child(
                section("shimmer-color", "Color")
                    .description("Highlight colors come from semantic theme roles.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_2()
                    .child(ShimmerText::new("Automatic theme-aware highlight"))
                    .child(
                        ShimmerText::new("Primary highlight")
                            .highlight_color(cx.theme().primary),
                    )
                    .child(
                        ShimmerText::new("Success highlight")
                            .highlight_color(cx.theme().success),
                    ),
            )
            .child(
                section("shimmer-duration", "Duration")
                    .description("Each duration controls one complete sweep.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_2()
                    .child(
                        ShimmerText::new("Quick sweep · 1 second")
                            .duration(Duration::from_secs(1)),
                    )
                    .child(
                        ShimmerText::new("Default sweep · 2 seconds")
                            .duration(Duration::from_secs(2)),
                    )
                    .child(
                        ShimmerText::new("Relaxed sweep · 4 seconds")
                            .duration(Duration::from_secs(4)),
                    ),
            )
            .child(
                section("shimmer-spread", "Spread")
                    .description("Spread is a relative or absolute highlight half-width.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_2()
                    .child(ShimmerText::new("Narrow highlight · 0.12").spread(0.12))
                    .child(ShimmerText::new("Default highlight · 0.30").spread(0.30))
                    .child(ShimmerText::new("Wide highlight · 0.60").spread(0.60))
                    .child(ShimmerText::new("Fixed highlight · 48px").spread(px(48.))),
            )
            .child(
                section("shimmer-direction", "Direction")
                    .description("Reverse changes movement without replacing text or layout.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_2()
                    .child(ShimmerText::new("Forward · left to right"))
                    .child(ShimmerText::new("Reverse · right to left").reverse(true)),
            )
            .child(
                section("shimmer-play-once", "Play once")
                    .description("A stable explicit identity controls one-shot playback and replay.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_3()
                    .child(
                        ShimmerText::new("A single sweep completes and then stops")
                            .id(("shimmer-single-sweep", replay_count))
                            .once(true),
                    )
                    .child(
                        Button::new("shimmer-replay").label("Replay").on_click(
                            cx.listener(|this, _, _, cx| {
                                this.replay_count += 1;
                                cx.notify();
                            }),
                        ),
                    ),
            )
            .child(
                section("shimmer-reusable-style", "Reusable style")
                    .description("One ShimmerStyle can be shared by independent status labels.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_2()
                    .child(
                        ShimmerText::new("Analyzing source files…")
                            .with_shimmer_style(shared_style),
                    )
                    .child(
                        ShimmerText::new("Preparing a response…").with_shimmer_style(shared_style),
                    ),
            )
            .child(
                section("shimmer-typography", "Typography and wrapping")
                    .description("Text inherits typography, color, and the surrounding layout.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_3()
                    .child(
                        ShimmerText::new("Compact supporting status")
                            .text_sm()
                            .text_color(cx.theme().muted_foreground),
                    )
                    .child(
                        ShimmerText::new("Prominent status")
                            .text_lg()
                            .font_semibold(),
                    )
                    .child(
                        h_flex()
                            .w(rems(24.))
                            .max_w_full()
                            .min_w_0()
                            .child(ShimmerText::new(
                                "Long loading messages remain readable as the surrounding region becomes narrower.",
                            )),
                    ),
            )
            .child(
                section("shimmer-marker", "Marker")
                    .description("Marker applies the same reusable style to its text content.")
                    .max_w(rems(42.5))
                    .v_flex()
                    .child(
                        Marker::new()
                            .loading(true)
                            .with_loading_style(MarkerLoadingStyle::Shimmer)
                            .with_shimmer_style(shared_style)
                            .content(MarkerContent::new().text("Generating an answer…")),
                    ),
            )
            .child(
                section("shimmer-attachment", "Attachment")
                    .description("An uploading or processing title can share its loading style.")
                    .max_w(rems(42.5))
                    .child(
                        Attachment::new()
                            .status(AttachmentStatus::Processing)
                            .content(
                                AttachmentContent::new()
                                    .title(
                                        AttachmentTitle::new("meeting-transcript.pdf")
                                            .with_shimmer_style(shared_style),
                                    )
                                    .description(AttachmentDescription::new(
                                        "Extracting key discussion points",
                                    )),
                            ),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "shimmer",
        "Shimmer",
        "Reusable, theme-aware text loading effects with composable timing and appearance.",
        ShimmerSection::view(window, cx),
    ));
}
