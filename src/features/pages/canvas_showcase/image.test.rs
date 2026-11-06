use gpui_kit::TestAppContext;

use crate::ui::canvas::CanvasShape;

use super::{CELL, ImageDemo, SWATCH, checker_swatch, render_image};

#[test]
fn checker_swatch_writes_rgba_bytes_at_exact_size() {
    let swatch = checker_swatch();
    assert_eq!(swatch.as_raw().len(), (SWATCH * SWATCH * 4) as usize);
    assert_eq!(swatch.get_pixel(0, 0).0, [230, 120, 60, 255]);
    assert_eq!(swatch.get_pixel(12, 0).0, [40, 90, 170, 255]);
}

#[test]
fn render_image_hands_bgra_bytes_to_the_renderer() {
    let image = render_image();
    let bytes = image.as_bytes(0).expect("one frame");
    assert_eq!(bytes.len(), (SWATCH * SWATCH * 4) as usize);
    // RenderImage samples BGRA, so the swatch's RGBA cells arrive swapped.
    assert_eq!(&bytes[0..4], &[60, 120, 230, 255][..]);
    let cell = (CELL * 4) as usize;
    assert_eq!(&bytes[cell..cell + 4], &[170, 90, 40, 255][..]);
}

#[gpui_kit::test]
async fn seed_places_two_placements_of_one_shared_image(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let handle = cx.add_window(|_, cx| ImageDemo::new(cx));
    let demo = handle.root(cx).expect("demo root entity");
    let state = cx.read(|cx| demo.read(cx).state.clone());
    let shapes = cx.read(|cx| state.read(cx).document().layers()[0].shapes().to_vec());
    assert_eq!(shapes.len(), 4);
    let images: Vec<_> = shapes
        .iter()
        .filter_map(|(_, shape)| match shape {
            CanvasShape::Image { image, .. } => Some(image.clone()),
            _ => None,
        })
        .collect();
    let frames: Vec<_> = shapes
        .iter()
        .filter(|(_, shape)| matches!(shape, CanvasShape::Rect { .. }))
        .collect();
    assert_eq!(images.len(), 2);
    assert_eq!(frames.len(), 2);
    assert_eq!(images[0].id, images[1].id);
}
