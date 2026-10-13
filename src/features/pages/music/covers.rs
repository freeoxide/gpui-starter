use std::sync::Arc;

use gpui_kit::{Image, ImageFormat};

use super::model::AlbumId;

/// Deterministic SVG cover for one album; no RNG, no text elements, so the
/// bytes are pure ASCII and stable per id.
pub(crate) fn album_svg(id: AlbumId) -> String {
    let hue = (id.0 as i32 * 137) % 360;
    let background = format!("hsl({hue}, 45%, 22%)");
    let foreground = format!("hsl({}, 70%, 62%)", (hue + 40) % 360);
    let pattern = match id.0 % 4 {
        0 => format!(
            "<path d=\"M0 0 L512 0 L0 512 Z\" fill=\"{foreground}\"/>\
             <circle cx=\"192\" cy=\"192\" r=\"80\" fill=\"{background}\"/>"
        ),
        1 => format!(
            "<circle cx=\"256\" cy=\"256\" r=\"220\" fill=\"{foreground}\"/>\
             <circle cx=\"256\" cy=\"256\" r=\"160\" fill=\"{background}\"/>\
             <circle cx=\"256\" cy=\"256\" r=\"110\" fill=\"{foreground}\"/>\
             <circle cx=\"256\" cy=\"256\" r=\"60\" fill=\"{background}\"/>\
             <circle cx=\"256\" cy=\"256\" r=\"20\" fill=\"{foreground}\"/>"
        ),
        2 => (0..7)
            .map(|i| {
                let fill = if i % 2 == 0 { &foreground } else { &background };
                format!(
                    "<rect x=\"{}\" y=\"0\" width=\"40\" height=\"512\" fill=\"{fill}\"/>",
                    8 + i * 72
                )
            })
            .collect::<String>(),
        _ => format!(
            "<path d=\"M0 0 L256 0 A256 256 0 0 1 0 256 Z\" fill=\"{foreground}\"/>\
             <path d=\"M512 0 L512 256 A256 256 0 0 1 256 0 Z\" fill=\"{foreground}\"/>\
             <path d=\"M512 512 L256 512 A256 256 0 0 1 512 256 Z\" fill=\"{foreground}\"/>\
             <path d=\"M0 512 L0 256 A256 256 0 0 1 256 512 Z\" fill=\"{foreground}\"/>"
        ),
    };
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"512\" height=\"512\" viewBox=\"0 0 512 512\">\
         <rect width=\"512\" height=\"512\" fill=\"{background}\"/>{pattern}</svg>"
    )
}

/// Image cache key is the hash of the bytes, so identical covers decode once.
pub(crate) fn album_image(id: AlbumId) -> Arc<Image> {
    Arc::new(Image::from_bytes(
        ImageFormat::Svg,
        album_svg(id).into_bytes(),
    ))
}

#[cfg(test)]
#[path = "covers.test.rs"]
mod covers_test;
