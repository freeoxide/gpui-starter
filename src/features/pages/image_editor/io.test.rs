use std::io::Cursor;

use image::{DynamicImage, GenericImageView as _, ImageFormat, RgbaImage};

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
    let loaded = load_image(&path).unwrap();

    assert_eq!(loaded.dimensions(), (3, 2));
    assert_eq!(loaded.as_raw(), original.as_raw());
}

#[test]
fn jpg_extension_encodes_without_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("photo.jpg");

    save_image(&path, &sample(4, 4)).unwrap();

    assert!(path.exists());
    let decoded = ImageReader::open(&path).unwrap().decode().unwrap();
    assert_eq!(decoded.dimensions(), (4, 4));
}

#[test]
fn in_memory_png_bytes_decode_back() {
    let mut bytes = Vec::new();
    DynamicImage::ImageRgba8(sample(2, 2))
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .unwrap();
    let decoded = image::load_from_memory(&bytes).unwrap().to_rgba8();
    assert_eq!(decoded.dimensions(), (2, 2));
}
