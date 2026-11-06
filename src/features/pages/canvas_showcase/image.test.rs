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

#[cfg(target_os = "linux")]
#[test]
fn painted_swatch_cells_keep_their_rgba_channel_order() {
    use gpui_kit::{point, px, size};

    use crate::ui::canvas::raster_test::{RasterCanvas, skip_without_headless_gpu};
    use crate::ui::canvas::{Bounds, CanvasGrid};

    if skip_without_headless_gpu() {
        return;
    }
    let warm = [230, 120, 60, 255];
    let cool = [40, 90, 170, 255];
    // A channel swap anywhere in the RGBA->BGRA upload -> renderer -> readback
    // chain flips warm and cool, so assert the painted pixels, not the buffer.
    for zoom in [1.0f32, 2.] {
        let mut canvas = RasterCanvas::open(size(px(208.), px(208.)), 2., |s, cx| {
            s.set_grid(
                CanvasGrid {
                    visible: false,
                    spacing: px(16.),
                },
                cx,
            );
            let layer = s.add_layer(cx);
            s.add_shape(
                layer,
                CanvasShape::image(
                    Bounds {
                        origin: point(px(8.), px(8.)),
                        size: size(px(96.), px(96.)),
                    },
                    render_image(),
                ),
                cx,
            )
            .unwrap();
        });
        canvas.edit(|s, cx| s.set_zoom(zoom, cx));
        let image = canvas.draw();
        assert_eq!(image.dimensions(), (416, 416));

        let painted = |doc_x: f32, doc_y: f32| {
            image
                .get_pixel((doc_x * zoom * 2.) as u32, (doc_y * zoom * 2.) as u32)
                .0
        };
        for (doc, expected) in [
            ((14., 14.), warm),
            ((26., 14.), cool),
            ((14., 26.), cool),
            ((26., 26.), warm),
        ] {
            let actual = painted(doc.0, doc.1);
            assert!(
                actual
                    .iter()
                    .zip(expected)
                    .all(|(a, e)| a.abs_diff(*e) <= 3),
                "zoom {zoom}: cell at {doc:?} painted {actual:?}, expected {expected:?}"
            );
        }

        // The placement starts 8 doc px in, so the pixels just outside it are
        // pure background at every zoom.
        let bg = image.get_pixel(4, 4).0;
        let near_edge = (8. * zoom * 2.) as u32 - 4;
        let mid_image = (56. * zoom * 2.) as u32;
        for (x, y) in [(near_edge, mid_image), (mid_image, near_edge)] {
            let actual = image.get_pixel(x, y).0;
            assert!(
                actual.iter().zip(bg).all(|(a, e)| a.abs_diff(*e) <= 3),
                "zoom {zoom}: ({x},{y}) is {actual:?}, background is {bg:?}"
            );
        }
    }
}
