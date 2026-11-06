//! Shared scaffolding for the messaging sections: the embedded cover preview
//! the message attachment demo shows.

use std::sync::Arc;

use gpui_kit::*;

pub(crate) use crate::gallery::demo::section;

/// The image the message attachment demo shows, decoded once through the
/// image cache. The app installs no HTTP client on native, so embedded bytes
/// are the only source that renders there.
pub(crate) fn cover_source() -> ImageSource {
    ImageSource::Image(Arc::new(Image::from_bytes(
        ImageFormat::Svg,
        include_bytes!("assets/cover-preview.svg").to_vec(),
    )))
}
