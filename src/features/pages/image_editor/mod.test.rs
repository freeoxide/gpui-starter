use std::io::Cursor;

use gpui_kit::component::color_picker::ColorPickerEvent;
use gpui_kit::component::slider::{SliderEvent, SliderValue};
use gpui_kit::{DevicePixels, TestAppContext, VisualTestContext, hsla, px, size};
use image::{DynamicImage, ImageFormat, RgbaImage};

use super::{EditorTool, ImageEditorPage};
use crate::ui::canvas::CanvasShape;

fn png_bytes(w: u32, h: u32) -> Vec<u8> {
    let img = RgbaImage::from_pixel(w, h, image::Rgba([120, 140, 160, 255]));
    let mut bytes = Vec::new();
    DynamicImage::ImageRgba8(img)
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .unwrap();
    bytes
}

fn open_page(cx: &mut TestAppContext) -> (gpui_kit::Entity<ImageEditorPage>, VisualTestContext) {
    cx.update(gpui_kit::component::theme::init);
    let handle = cx.open_window(size(px(800.), px(600.)), |window, cx| {
        ImageEditorPage::new(window, cx)
    });
    let page = handle.root(cx).unwrap();
    let cx = VisualTestContext::from_window(handle.into(), cx);
    (page, cx)
}

/// Drives the full open flow: the button task's prompt, the disk read, the
/// decode, the canvas shape, the filtered display, and the fit-to-viewport.
#[gpui_kit::test]
fn on_open_mounts_the_picked_image_into_the_canvas(cx: &mut TestAppContext) {
    let (page, mut cx) = open_page(cx);
    cx.run_until_parked();
    // A paint pass first: on_prepaint must store the canvas-area bounds so the
    // post-open fit has a real viewport instead of staying deferred.
    cx.update(|window, cx| window.draw(cx).clear(cx));

    let file = std::env::temp_dir().join(format!(
        "gpui-starter-image-editor-open-{}.png",
        std::process::id()
    ));
    std::fs::write(&file, png_bytes(24, 12)).unwrap();

    cx.update(|window, cx| {
        page.update(cx, |page, cx| page.on_open(window, cx));
    });
    cx.run_until_parked();
    let picked = file.clone();
    cx.simulate_path_prompt_response(move |_options| Some(vec![picked]));
    cx.run_until_parked();
    let _ = std::fs::remove_file(&file);

    cx.read(|cx| {
        let view = page.read(cx);
        let doc = view.doc.read(cx);
        assert!(doc.has_image(), "decoded image became the document base");
        assert_eq!(doc.base_size(), Some((24, 12)));
        let display = doc.display().expect("filtered display derived after open");
        assert_eq!(
            (display.size(0).width, display.size(0).height),
            (DevicePixels(24), DevicePixels(12))
        );
        let shape_id = doc
            .base_shape()
            .expect("base image shape mounted on the canvas");
        match view.canvas.read(cx).document().shape(shape_id) {
            Some(CanvasShape::Image { bounds, .. }) => {
                assert_eq!((bounds.size.width, bounds.size.height), (px(24.), px(12.)));
            }
            other => panic!("expected an image shape, got {other:?}"),
        }
        assert!(!view.pending_fit, "fit ran once the viewport was known");
        assert!(
            view.canvas.read(cx).zoom() > 1.,
            "the 24x12 doc is scaled up to fit the window; viewport={:?}",
            view.viewport
        );
        assert_eq!(view.canvas.read(cx).active_tool_id(), "image-brush");
    });
}

/// A cancelled prompt must leave the editor untouched: no document, no error.
#[gpui_kit::test]
fn on_open_cancel_keeps_the_document_empty(cx: &mut TestAppContext) {
    let (page, mut cx) = open_page(cx);
    cx.run_until_parked();

    cx.update(|window, cx| {
        page.update(cx, |page, cx| page.on_open(window, cx));
    });
    cx.run_until_parked();
    cx.simulate_path_prompt_response(|_options| None);
    cx.run_until_parked();

    cx.read(|cx| {
        let view = page.read(cx);
        assert!(!view.doc.read(cx).has_image());
        assert!(view.doc.read(cx).last_error().is_none());
        assert!(view.reported_error.is_none(), "cancel is not an error");
    });
}

#[gpui_kit::test]
fn image_opened_mounts_the_default_brush_tool(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let (page, cx) = cx.add_window_view(|window, cx| ImageEditorPage::new(window, cx));
    let canvas = page.update(cx, |page, _| page.canvas.clone());
    page.update(cx, |page, cx| {
        page.doc
            .update(cx, |doc, cx| doc.open(png_bytes(16, 8), &canvas, cx));
    });
    cx.run_until_parked();

    page.update(cx, |page, cx| {
        assert!(page.doc.read(cx).has_image());
        assert_eq!(page.canvas.read(cx).active_tool_id(), "image-brush");
    });
}

#[gpui_kit::test]
fn select_tool_swaps_the_active_editor_tool(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let (page, cx) = cx.add_window_view(|window, cx| ImageEditorPage::new(window, cx));
    let canvas = page.update(cx, |page, _| page.canvas.clone());
    page.update(cx, |page, cx| {
        page.doc
            .update(cx, |doc, cx| doc.open(png_bytes(16, 8), &canvas, cx));
    });
    cx.run_until_parked();

    page.update(cx, |page, cx| {
        page.select_tool(EditorTool::Eraser, cx);
        assert_eq!(page.canvas.read(cx).active_tool_id(), "image-eraser");
        page.select_tool(EditorTool::Crop, cx);
        assert_eq!(page.canvas.read(cx).active_tool_id(), "image-crop");
        page.select_tool(EditorTool::Arrow, cx);
        assert_eq!(page.canvas.read(cx).active_tool_id(), "image-arrow");
        assert_eq!(page.tool, EditorTool::Arrow);
    });
}

#[gpui_kit::test]
fn brush_size_release_updates_tool_params(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let (page, cx) = cx.add_window_view(|window, cx| ImageEditorPage::new(window, cx));
    let canvas = page.update(cx, |page, _| page.canvas.clone());
    page.update(cx, |page, cx| {
        page.doc
            .update(cx, |doc, cx| doc.open(png_bytes(16, 8), &canvas, cx));
    });
    cx.run_until_parked();

    page.update(cx, |page, cx| {
        page.brush_size.update(cx, |_, cx| {
            cx.emit(SliderEvent::Release(SliderValue::Single(9.)))
        });
    });
    cx.run_until_parked();
    page.update(cx, |page, cx| {
        assert_eq!(page.canvas.read(cx).active_tool_id(), "image-brush");
        assert_eq!(page.brush_value, 9.);
    });
}

#[gpui_kit::test]
fn color_change_updates_tool_params(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let (page, cx) = cx.add_window_view(|window, cx| ImageEditorPage::new(window, cx));
    let canvas = page.update(cx, |page, _| page.canvas.clone());
    page.update(cx, |page, cx| {
        page.doc
            .update(cx, |doc, cx| doc.open(png_bytes(16, 8), &canvas, cx));
    });
    cx.run_until_parked();

    let color = hsla(210., 0.8, 0.4, 1.);
    page.update(cx, |page, cx| {
        page.color
            .update(cx, |_, cx| cx.emit(ColorPickerEvent::Change(Some(color))));
    });
    cx.run_until_parked();
    page.update(cx, |page, cx| {
        assert_eq!(page.color_value, color);
        assert_eq!(page.canvas.read(cx).active_tool_id(), "image-brush");
    });
}

#[gpui_kit::test]
fn reopening_an_image_resets_filter_controls(cx: &mut TestAppContext) {
    use gpui_kit::component::slider::SliderValue;

    cx.update(gpui_kit::component::theme::init);
    let (page, cx) = cx.add_window_view(|window, cx| ImageEditorPage::new(window, cx));
    let canvas = page.update(cx, |page, _| page.canvas.clone());
    for _ in 0..2 {
        page.update(cx, |page, cx| {
            page.doc
                .update(cx, |doc, cx| doc.open(png_bytes(16, 8), &canvas, cx));
        });
        cx.run_until_parked();
    }

    page.update(cx, |page, cx| {
        page.gray = true;
        page.doc.update(cx, |doc, cx| {
            doc.set_filters(
                crate::features::pages::image_editor::document::FilterSettings {
                    grayscale: true,
                    blur_sigma: 3.,
                    brightness: 20,
                    ..Default::default()
                },
                cx,
            )
        });
    });

    page.update(cx, |page, cx| {
        page.doc
            .update(cx, |doc, cx| doc.open(png_bytes(8, 8), &canvas, cx));
    });
    cx.run_until_parked();

    page.update(cx, |page, cx| {
        assert!(!page.gray, "switch resets with the document");
        assert_eq!(page.blur.read(cx).value().start(), 0.);
        assert_eq!(page.brightness.read(cx).value().start(), 0.);
        assert_eq!(
            page.doc.read(cx).filters(),
            Default::default(),
            "document filters reset on open"
        );
    });

    cx.update(|window, cx| {
        page.update(cx, |page, cx| {
            page.blur.update(cx, |state, cx| {
                state.set_value(5., window, cx);
                cx.emit(SliderEvent::Release(SliderValue::Single(5.)))
            });
        });
    });
    cx.run_until_parked();
    page.update(cx, |page, cx| {
        assert_eq!(
            page.doc.read(cx).filters().blur_sigma,
            5.,
            "rebuilt slider subscription still applies filters"
        );
    });
    page.update(cx, |page, cx| {
        assert_eq!(
            page.doc.read(cx).filters().blur_sigma,
            5.,
            "rebuilt slider subscription still applies filters"
        );
    });
}
