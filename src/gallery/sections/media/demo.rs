//! Shared scaffolding for the media sections: the embedded SVG preview every
//! media card shows.

use std::sync::Arc;

use gpui_kit::*;

pub(crate) use crate::gallery::demo::{section, size_label};

/// The preview every media card shares, decoded once through the image cache
/// (keyed by the bytes' hash). The app installs no HTTP client, so an
/// embedded source is the only one that renders.
pub(crate) fn preview_source() -> ImageSource {
    ImageSource::Image(Arc::new(Image::from_bytes(
        ImageFormat::Svg,
        include_bytes!("assets/sdk-preview.svg").to_vec(),
    )))
}
