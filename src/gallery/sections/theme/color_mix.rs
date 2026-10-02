//! Oklab Color Mix section, ported from the upstream `color_mix_oklab`
//! example: Oklab and HSL mixing of one theme color with transparent black,
//! compared side by side.

use gpui_kit::component::{ActiveTheme as _, Colorize as _, StyledExt as _, h_flex, v_flex};
use gpui_kit::{
    App, AppContext, Context, Entity, FontWeight, Hsla, IntoElement, ParentElement, Render, Styled,
    Window, div, transparent_black,
};

use crate::gallery::registry::GallerySection;

use super::demo::section;

pub struct ColorMixSection;

impl ColorMixSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self)
    }
}

impl Render for ColorMixSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let danger = cx.theme().danger;
        let danger_foreground = cx.theme().danger_foreground;
        let muted_foreground = cx.theme().muted_foreground;
        let transparent = transparent_black();

        // Mirrors CSS color-mix(in oklab, var(--danger) 20%, transparent).
        let mixed_20 = danger.mix_oklab(transparent, 0.2);
        let mixed_50 = danger.mix_oklab(transparent, 0.5);
        let mixed_80 = danger.mix_oklab(transparent, 0.8);

        let swatch = |label: &'static str, color: Hsla| {
            div()
                .size_20()
                .bg(color)
                .child(label)
                .text_color(danger_foreground)
        };

        v_flex()
            .gap_4()
            .w_full()
            .p_4()
            .child(
                section("color-mix-oklab", "Oklab mix")
                    .description("Oklab mixing keeps the source hue and fades only alpha.")
                    .v_flex()
                    .gap_2()
                    .child("Oklab mix with transparent (premultiplied alpha):")
                    .child(
                        h_flex()
                            .gap_2()
                            .child(swatch("100%", danger))
                            .child(swatch("80%", mixed_80))
                            .child(swatch("50%", mixed_50))
                            .child(swatch("20%", mixed_20)),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .text_color(muted_foreground)
                            .child(format!(
                                "Source color: {} (alpha: {:.2})",
                                danger.to_hex(),
                                danger.a
                            ))
                            .child(format!(
                                "80% mix: {} (alpha: {:.2})",
                                mixed_80.to_hex(),
                                mixed_80.a
                            ))
                            .child(format!(
                                "50% mix: {} (alpha: {:.2})",
                                mixed_50.to_hex(),
                                mixed_50.a
                            ))
                            .child(format!(
                                "20% mix: {} (alpha: {:.2})",
                                mixed_20.to_hex(),
                                mixed_20.a
                            )),
                    ),
            )
            .child(
                section("color-mix-hsl", "HSL comparison")
                    .description("The two models disagree once the color carries alpha.")
                    .h_flex()
                    .gap_4()
                    .child(
                        v_flex()
                            .gap_1()
                            .child(div().font_weight(FontWeight::MEDIUM).child("HSL mix:"))
                            .child(
                                div()
                                    .size_16()
                                    .bg(danger.mix(transparent, 0.5))
                                    .text_color(danger_foreground)
                                    .child("HSL"),
                            )
                            .child(danger.mix(transparent, 0.5).to_hex()),
                    )
                    .child(
                        v_flex()
                            .gap_1()
                            .child(div().font_weight(FontWeight::MEDIUM).child("Oklab mix:"))
                            .child(
                                div()
                                    .size_16()
                                    .bg(mixed_50)
                                    .text_color(danger_foreground)
                                    .child("Oklab"),
                            )
                            .child(mixed_50.to_hex()),
                    ),
            )
            .child(
                section("color-mix-notes", "How they differ")
                    .v_flex()
                    .gap_1()
                    .child("Oklab mixing keeps the source color's hue and changes only alpha")
                    .child(
                        "HSL mixing darkens the color, because the transparent endpoint is black",
                    )
                    .child("Oklab uses premultiplied alpha, matching the CSS color-mix() rule"),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "color-mix",
        "Oklab Color Mix",
        "Oklab and HSL mixing of one theme color with transparent black.",
        ColorMixSection::view(window, cx),
    ));
}
