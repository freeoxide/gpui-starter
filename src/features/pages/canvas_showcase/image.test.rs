use gpui_kit::{DevicePixels, TestAppContext, size};

use crate::ui::canvas::CanvasShape;

use super::{ImageDemo, SWATCH, checker_swatch, render_image};

#[test]
fn checker_swatch_writes_bgra_bytes_at_exact_size() {
    let swatch = checker_swatch();
    assert_eq!(swatch.as_raw().len(), (SWATCH * SWATCH * 4) as usize);
    assert_eq!(swatch.get_pixel(0, 0).0, [60, 120, 230, 255]);
    assert_eq!(swatch.get_pixel(12, 0).0, [170, 90, 40, 255]);
}

#[test]
fn render_image_reports_the_swatch_dimensions() {
    let image = render_image();
    assert_eq!(image.size(0), size(DevicePixels(96), DevicePixels(96)));
}

#[test]
fn each_render_image_call_gets_a_distinct_id() {
    let a = render_image();
    let b = render_image();
    assert_ne!(a.id, b.id);
    assert_ne!(a, b);
}

#[gpui_kit::test]
fn seed_places_two_placements_of_one_shared_image(cx: &mut TestAppContext) {
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
