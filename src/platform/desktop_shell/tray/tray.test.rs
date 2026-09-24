use super::*;

#[test]
fn icon_pixel_buffer_matches_dimensions() {
    for size in [16usize, 24, 36] {
        let px = icon_pixels(size);
        assert_eq!(px.len(), size * size * 4);
        assert!(px.chunks(4).any(|rgba| rgba[3] > 0));
    }
}

#[test]
fn icon_pixel_buffer_has_opaque_ring_and_transparent_corners() {
    let size = 36usize;
    let px = icon_pixels(size);
    let alpha = |x: usize, y: usize| px[(y * size + x) * 4 + 3];
    // Pixel centers on the lens ring and the handle are opaque; the corners
    // sit outside the glyph.
    assert_eq!(alpha(15, 23), 255);
    assert_eq!(alpha(26, 26), 255);
    assert_eq!(alpha(0, 0), 0);
    // The glyph is monochrome black.
    assert_eq!(px[0], 0);
    assert_eq!(px[1], 0);
    assert_eq!(px[2], 0);
}
