//! Attachment section, ported from the upstream `AttachmentStory`.

use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, Sizable as _, StyledExt as _, WindowExt as _,
    attachment::{
        Attachment, AttachmentActions, AttachmentContent, AttachmentDescription, AttachmentGroup,
        AttachmentMedia, AttachmentStatus, AttachmentTitle,
    },
    button::{Button, ButtonVariants as _},
    shimmer::ShimmerStyle,
    v_flex,
};
use gpui_kit::*;

use crate::gallery::registry::GallerySection;

use super::demo::{preview_source, section};

pub struct AttachmentSection;

impl AttachmentSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self)
    }
}

impl Render for AttachmentSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let image_tile = |id: &'static str, status: AttachmentStatus| {
            Attachment::new()
                .id(id)
                .axis(Axis::Vertical)
                .status(status)
                .on_remove(|_, _, _| {})
                .media(AttachmentMedia::new().src(preview_source()))
        };
        let file_chip = |id: &'static str,
                         status: AttachmentStatus,
                         name: &'static str,
                         description: &'static str| {
            Attachment::new()
                .id(id)
                .status(status)
                .on_remove(|_, _, _| {})
                .media(AttachmentMedia::new().child(Icon::new(IconName::FileText)))
                .content(
                    AttachmentContent::new()
                        .title(AttachmentTitle::new(name))
                        .description(AttachmentDescription::new(description)),
                )
        };

        v_flex()
            .gap_4()
            .w_full()
            .items_center()
            .p_4()
            .child(
                section("attachment-composer", "Composer")
                    .description(
                        "Image tiles and file chips in a scrolling row, with the built-in remove, \
                         retry, progress and tooltip controls. Hover a card for its remove control.",
                    )
                    .max_w(rems(42.5))
                    .child(
                        AttachmentGroup::new("attachment-composer-row")
                            .with_edge_fade(cx.theme().background)
                            .child(image_tile(
                                "attachment-composer-image",
                                AttachmentStatus::Complete,
                            ))
                            .child(
                                image_tile(
                                    "attachment-composer-image-uploading",
                                    AttachmentStatus::Uploading,
                                )
                                .progress(62.),
                            )
                            .child(
                                image_tile(
                                    "attachment-composer-image-failed",
                                    AttachmentStatus::Failed,
                                )
                                .tooltip("Network error · Click to retry")
                                .on_retry(|_, window, cx| {
                                    window.push_notification("Retrying photo.png…", cx);
                                }),
                            )
                            .child(
                                image_tile(
                                    "attachment-composer-image-rejected",
                                    AttachmentStatus::Failed,
                                )
                                .tooltip("Image exceeds 20 MB limit · Remove to send"),
                            )
                            .child(file_chip(
                                "attachment-composer-file",
                                AttachmentStatus::Complete,
                                "Q3 statement.pdf",
                                "PDF · 1.2 MB",
                            ))
                            .child(
                                file_chip(
                                    "attachment-composer-file-uploading",
                                    AttachmentStatus::Uploading,
                                    "Q3 statement.pdf",
                                    "Uploading",
                                )
                                .progress(62.),
                            )
                            .child(
                                file_chip(
                                    "attachment-composer-file-failed",
                                    AttachmentStatus::Failed,
                                    "Q3 statement.pdf",
                                    "Upload failed",
                                )
                                .tooltip("Network error · Click to retry")
                                .on_retry(|_, window, cx| {
                                    window.push_notification("Retrying Q3 statement.pdf…", cx);
                                }),
                            )
                            .child(
                                file_chip(
                                    "attachment-composer-file-rejected",
                                    AttachmentStatus::Failed,
                                    "accessibility-review-and-keyboard-navigation-findings.xlsx",
                                    "Exceeds 20 MB limit",
                                )
                                .tooltip("Max file size is 20 MB · Remove to send"),
                            ),
                    ),
            )
            .child(
                section("attachment-lifecycle", "Lifecycle")
                    .description(
                        "The states the composer row does not show: pending, an upload without a \
                         known percentage, processing with a custom shimmer, and a completed file.",
                    )
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_3()
                    .child(file_chip(
                        "attachment-lifecycle-pending",
                        AttachmentStatus::Pending,
                        "meeting-notes.pdf",
                        "Ready to upload",
                    ))
                    .child(file_chip(
                        "attachment-lifecycle-uploading",
                        AttachmentStatus::Uploading,
                        "design-assets.zip",
                        "Uploading",
                    ))
                    .child(
                        Attachment::new()
                            .id("attachment-lifecycle-processing")
                            .status(AttachmentStatus::Processing)
                            .media(AttachmentMedia::new().child(Icon::new(IconName::FileText)))
                            .content(
                                AttachmentContent::new().title(
                                    AttachmentTitle::new("transcript.pdf").with_shimmer_style(
                                        ShimmerStyle::new()
                                            .highlight_color(cx.theme().primary)
                                            .spread(0.45)
                                            .reverse(true),
                                    ),
                                ),
                            ),
                    )
                    .child(
                        Attachment::new()
                            .id("attachment-lifecycle-complete")
                            .media(
                                AttachmentMedia::new()
                                    .text_color(cx.theme().success)
                                    .child(Icon::new(IconName::CircleCheck)),
                            )
                            .content(
                                AttachmentContent::new()
                                    .title(AttachmentTitle::new("published-report.pdf"))
                                    .description(AttachmentDescription::new("Uploaded · 1.8 MB")),
                            ),
                    ),
            )
            .child(
                section("attachment-preview-card", "Preview card")
                    .description(
                        "A vertical card puts the preview above the metadata; actions sit over its \
                         upper trailing corner and stay clickable above the whole-card click.",
                    )
                    .max_w(rems(42.5))
                    .child(
                        Attachment::new()
                            .id("attachment-preview-card")
                            .axis(Axis::Vertical)
                            .on_click(|_, window, cx| {
                                window.push_notification("Opening sdk-preview.svg…", cx);
                            })
                            .media(AttachmentMedia::new().src(preview_source()))
                            .content(
                                AttachmentContent::new()
                                    .title(AttachmentTitle::new("sdk-preview.svg"))
                                    .description(AttachmentDescription::new("SVG · 1280 × 720")),
                            )
                            .actions(
                                AttachmentActions::new().child(
                                    Button::new("attachment-preview-card-open")
                                        .ghost()
                                        .xsmall()
                                        .label("Open")
                                        .on_click(|_, window, cx| {
                                            window.push_notification("Opened sdk-preview.svg", cx);
                                        }),
                                ),
                            ),
                    ),
            )
            .child(
                section("attachment-sizes", "Sizes")
                    .description(
                        "One geometry scale per named size: card, media box and type move together.",
                    )
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_3()
                    .child(
                        file_chip(
                            "attachment-size-large",
                            AttachmentStatus::Complete,
                            "large.pdf",
                            "Large · PDF · 3.1 MB",
                        )
                        .large(),
                    )
                    .child(file_chip(
                        "attachment-size-medium",
                        AttachmentStatus::Complete,
                        "medium.pdf",
                        "Medium · PDF · 2.4 MB",
                    ))
                    .child(
                        file_chip(
                            "attachment-size-small",
                            AttachmentStatus::Complete,
                            "small.csv",
                            "Small · CSV · 840 KB",
                        )
                        .small(),
                    )
                    .child(
                        file_chip(
                            "attachment-size-xsmall",
                            AttachmentStatus::Complete,
                            "xsmall.txt",
                            "XSmall · TXT · 4 KB",
                        )
                        .xsmall(),
                    ),
            )
            .child(
                section("attachment-custom-style", "Custom style")
                    .description(
                        "Every public part takes style refinements, and an overlay is painted above \
                         the preview and any status scrim.",
                    )
                    .max_w(rems(42.5))
                    .v_flex()
                    .gap_3()
                    .child(
                        Attachment::new()
                            .w_full()
                            .rounded(cx.theme().radius)
                            .bg(cx.theme().accent)
                            .border_color(cx.theme().accent.opacity(0.5))
                            .media(
                                AttachmentMedia::new()
                                    .rounded(cx.theme().radius)
                                    .bg(cx.theme().primary.opacity(0.12))
                                    .text_color(cx.theme().primary)
                                    .child(Icon::new(IconName::FileText)),
                            )
                            .content(
                                AttachmentContent::new()
                                    .title(
                                        AttachmentTitle::new("custom-theme.json")
                                            .text_color(cx.theme().primary),
                                    )
                                    .description(AttachmentDescription::new("JSON · 16 KB")),
                            ),
                    )
                    .child(
                        Attachment::new()
                            .axis(Axis::Vertical)
                            .media(AttachmentMedia::new().src(preview_source()).overlay(
                                Icon::new(IconName::Play).text_color(cx.theme().foreground),
                            )),
                    ),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "attachment",
        "Attachment",
        "Composable file and media attachments with lifecycle states and actions.",
        AttachmentSection::view(window, cx),
    ));
}
