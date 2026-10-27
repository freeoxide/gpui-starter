use std::io::Cursor;

use gpui_kit::component::color_picker::ColorPickerEvent;
use gpui_kit::component::slider::{SliderEvent, SliderValue};
use gpui_kit::{TestAppContext, hsla};
use image::{DynamicImage, ImageFormat, RgbaImage};

use super::{EditorTool, ImageEditorPage};

fn png_bytes(w: u32, h: u32) -> Vec<u8> {
    let img = RgbaImage::from_pixel(w, h, image::Rgba([120, 140, 160, 255]));
    let mut bytes = Vec::new();
    DynamicImage::ImageRgba8(img)
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .unwrap();
    bytes
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
