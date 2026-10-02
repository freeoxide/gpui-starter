//! Media and asset sections, ported from the upstream stories and asset
//! examples: Attachment, Assets, Avatar, Clipboard, Image, and WebView.

mod assets;
mod attachment;
mod avatar;
mod clipboard;
mod demo;
mod image;
mod webview;

use gpui_kit::{App, Window};

use crate::gallery::registry::GallerySection;

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    attachment::register(sections, window, cx);
    assets::register(sections, window, cx);
    avatar::register(sections, window, cx);
    clipboard::register(sections, window, cx);
    image::register(sections, window, cx);
    webview::register(sections, window, cx);
}
