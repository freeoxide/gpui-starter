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
fn icon_pixel_buffer_draws_the_brand_g() {
    let size = 36usize;
    let px = icon_pixels(size);
    let alpha = |x: usize, y: usize| px[(y * size + x) * 4 + 3];
    // Opaque on the ring's left and top and on the crossbar; the mouth
    // above the crossbar and the corners sit outside the glyph.
    assert_eq!(alpha(6, 18), 255);
    assert_eq!(alpha(18, 6), 255);
    assert_eq!(alpha(29, 18), 255);
    assert_eq!(alpha(29, 10), 0);
    assert_eq!(alpha(0, 0), 0);
    // The glyph is monochrome black.
    assert_eq!(px[0], 0);
    assert_eq!(px[1], 0);
    assert_eq!(px[2], 0);
}
