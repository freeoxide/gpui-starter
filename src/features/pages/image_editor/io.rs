use std::fs::File;
use std::path::Path;

use anyhow::Context as _;
use image::{DynamicImage, ImageFormat, RgbaImage};

pub(crate) fn save_image(path: &Path, img: &RgbaImage) -> anyhow::Result<()> {
    let format = match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .as_deref()
    {
        Some("jpg" | "jpeg") => ImageFormat::Jpeg,
        _ => ImageFormat::Png,
    };
    DynamicImage::ImageRgba8(img.clone())
        .write_to(&mut File::create(path)?, format)
        .with_context(|| format!("encode {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
#[path = "io.test.rs"]
mod io_test;
