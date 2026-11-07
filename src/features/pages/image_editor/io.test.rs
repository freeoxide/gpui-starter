use image::{GenericImageView as _, ImageFormat, ImageReader, RgbaImage};

use super::*;

fn sample(w: u32, h: u32) -> RgbaImage {
    RgbaImage::from_fn(w, h, |x, y| {
        image::Rgba([(x % 256) as u8, (y % 256) as u8, 128, 255])
    })
}

#[test]
fn save_then_load_png_round_trips_pixels() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("round-trip.png");
    let original = sample(3, 2);

    save_image(&path, &original).unwrap();
    let loaded = ImageReader::open(&path)
        .unwrap()
        .decode()
        .unwrap()
        .to_rgba8();

    assert_eq!(loaded.dimensions(), (3, 2));
    assert_eq!(loaded.as_raw(), original.as_raw());
}

#[test]
fn jpg_extension_encodes_jpeg_bytes_not_png() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("photo.jpg");

    save_image(&path, &sample(4, 4)).unwrap();

    let reader = ImageReader::open(&path)
        .unwrap()
        .with_guessed_format()
        .unwrap();
    assert_eq!(
        reader.format(),
        Some(ImageFormat::Jpeg),
        "the jpg extension must select the JPEG encoder, not the PNG fallback"
    );
    let decoded = reader.decode().unwrap();
    assert_eq!(decoded.dimensions(), (4, 4));
}
