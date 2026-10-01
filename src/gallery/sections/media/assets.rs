//! Assets section, ported from the app_assets example and the assets-crate
//! source recipes: an embedded rust-embed folder, selected catalog icons,
//! a composite source, and the full catalog.

use std::borrow::Cow;

use gpui_kit::assets::icon_assets;
use gpui_kit::component::{ActiveTheme as _, Icon, IconName, h_flex, v_flex};
use gpui_kit::*;
use gpui_kit::{AssetSource, Result, SharedString, prelude::FluentBuilder as _, rems};
use rust_embed::RustEmbed;

use crate::gallery::registry::GallerySection;

use super::demo::section;

const ACCESSIBILITY: &str = "icons/accessibility.svg";
const ALARM_CLOCK: &str = "icons/alarm-clock.svg";

icon_assets!(SelectedIcons, [Accessibility, AlarmClock]);

/// The section's own asset folder, embedded the way the app embeds themes/
/// (debug builds read the live folder; release embeds it).
#[derive(RustEmbed)]
#[folder = "src/gallery/sections/media/assets"]
struct GalleryAssets;

/// Composite lookup order: selected catalog icons first, then the embedded
/// folder. The app's own source composes project files over kit icons the
/// same way; nothing here is registered app-wide.
struct GallerySource;

impl AssetSource for GallerySource {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        // Debug rust-embed reads the live fs, so reject traversal before it.
        if path.is_empty() || path.split('/').any(|seg| seg == "..") {
            return Ok(None);
        }
        if let Some(bytes) = SelectedIcons.load(path)? {
            return Ok(Some(bytes));
        }
        Ok(GalleryAssets::get(path).map(|file| file.data))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut paths: Vec<SharedString> = GalleryAssets::iter()
            .filter(|p| p.starts_with(path))
            .map(Into::into)
            .collect();
        paths.extend(SelectedIcons.list(path)?);
        paths.sort();
        paths.dedup();
        Ok(paths)
    }
}

/// Bytes from the full catalog source: a unit value on native, an on-demand
/// client on the web build.
fn catalog_bytes(path: &str) -> Option<Cow<'static, [u8]>> {
    #[cfg(not(target_family = "wasm"))]
    return gpui_kit::assets::AllAssets.load(path).ok().flatten();
    #[cfg(target_family = "wasm")]
    return gpui_kit::assets::AllAssets::default()
        .load(path)
        .ok()
        .flatten();
}

pub struct AssetsSection;

impl AssetsSection {
    pub fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self)
    }

    fn folder_tile(&self, path: &'static str, cx: &App) -> impl IntoElement {
        let file = GalleryAssets::get(path).expect("key exists in the embedded folder");
        v_flex()
            .gap_1()
            .items_center()
            .child(Icon::default().data(&file.data).size_6())
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(path),
            )
    }
}

impl Render for AssetsSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let keys = GalleryAssets::iter()
            .map(|key| key.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        let selected_tile = |path: &'static str| {
            let bytes = SelectedIcons
                .load(path)
                .expect("selected source serves its own paths")
                .expect("path is one of the selected icons");
            v_flex()
                .gap_1()
                .items_center()
                .child(Icon::default().data(&bytes).size_6())
                .child(
                    div()
                        .text_xs()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("{path} · {} bytes", bytes.len())),
                )
        };
        let probe = |path: &'static str| {
            let answered = if SelectedIcons.load(path).ok().flatten().is_some() {
                "selected icons"
            } else if GalleryAssets::get(path).is_some() {
                "embedded folder"
            } else {
                "not served"
            };
            h_flex()
                .gap_2()
                .when_some(GallerySource.load(path).ok().flatten(), |this, bytes| {
                    this.child(Icon::default().data(&bytes).size_6())
                })
                .child(div().text_sm().child(path))
                .child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child(format!("→ {answered}")),
                )
        };

        v_flex()
            .w_full()
            .items_center()
            .gap_6()
            .p_4()
            .child(
                section("assets-embedded-folder", "Embedded folder")
                    .description(
                        "A rust-embed struct serves every file under the section's assets folder.",
                    )
                    .w(rems(30.))
                    .child(
                        v_flex()
                            .gap_3()
                            .child(
                                h_flex()
                                    .gap_6()
                                    .child(self.folder_tile("icons/bot.svg", cx))
                                    .child(self.folder_tile("icons/inbox.svg", cx)),
                            )
                            .child(
                                h_flex()
                                    .gap_6()
                                    .child(
                                        v_flex()
                                            .gap_1()
                                            .items_center()
                                            .child(Icon::new(IconName::Inbox))
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child("app's registered source"),
                                            ),
                                    )
                                    .child(
                                        v_flex()
                                            .gap_1()
                                            .items_center()
                                            .child(Icon::new(IconName::Bot))
                                            .child(
                                                div()
                                                    .text_xs()
                                                    .text_color(cx.theme().muted_foreground)
                                                    .child("app's registered source"),
                                            ),
                                    ),
                            )
                            .child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(format!("Folder keys: {keys}")),
                            ),
                    ),
            )
            .child(
                section("assets-selected-icons", "Selected catalog icons")
                    .description(
                        "The icon_assets! macro embeds only the named catalog icons; every other \
                         path returns None.",
                    )
                    .w(rems(30.))
                    .child(
                        h_flex()
                            .gap_6()
                            .child(selected_tile(ACCESSIBILITY))
                            .child(selected_tile(ALARM_CLOCK)),
                    ),
            )
            .child(
                section("assets-composite-source", "Composite source")
                    .description(
                        "The composite checks the selected icons first, then falls back to the \
                         embedded folder.",
                    )
                    .w(rems(30.))
                    .child(
                        v_flex()
                            .gap_2()
                            .child(probe(ACCESSIBILITY))
                            .child(probe("icons/bot.svg"))
                            .child(probe("icons/search.svg")),
                    ),
            )
            .child(
                section("assets-full-catalog", "Full catalog")
                    .description(
                        "AllAssets embeds every catalog icon on native; the web build fetches each \
                         icon on demand instead.",
                    )
                    .w(rems(30.))
                    .child(match catalog_bytes(ACCESSIBILITY) {
                        Some(bytes) => h_flex()
                            .gap_2()
                            .items_center()
                            .child(Icon::default().data(&bytes).size_6())
                            .child(
                                div()
                                    .text_sm()
                                    .child(format!("{ACCESSIBILITY} · {} bytes", bytes.len())),
                            )
                            .into_any_element(),
                        None => div()
                            .text_sm()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{ACCESSIBILITY} · not served on this platform yet"))
                            .into_any_element(),
                    }),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "assets",
        "Assets",
        "Embed icons and images through rust-embed, selected catalog icons, or the full catalog.",
        AssetsSection::view(window, cx),
    ));
}
