use std::io::Cursor;

use gpui_kit::component::color_picker::ColorPickerEvent;
use gpui_kit::component::slider::{SliderEvent, SliderValue};
use gpui_kit::{
    Bounds, DevicePixels, Modifiers, MouseButton, TestAppContext, VisualTestContext, hsla, point,
    px, size,
};
use image::{DynamicImage, ImageFormat, ImageReader, RgbaImage};

use super::document::FilterSettings;
use super::{EditorTool, ImageEditorPage, SaveState};
use crate::ui::canvas::{
    CanvasPointerEvent, CanvasShape, CanvasTransform, StrokeStyle, ToolContext,
};

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
    let (page, cx) = cx.add_window_view(ImageEditorPage::new);
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
    let (page, cx) = cx.add_window_view(ImageEditorPage::new);
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

/// Commits a brush tap through the mounted tool so tests can assert the
/// StrokeStyle the page actually wired into it.
fn commit_brush_tap(
    page: &gpui_kit::Entity<ImageEditorPage>,
    cx: &mut TestAppContext,
    at: (f32, f32),
) {
    let canvas = cx.read(|cx| page.read(cx).canvas.clone());
    let position = point(px(at.0), px(at.1));
    let event = CanvasPointerEvent {
        screen_position: position,
        doc_position: position,
        button: MouseButton::Left,
        click_count: 1,
        modifiers: Modifiers::default(),
    };
    let tool_ctx = ToolContext {
        viewport: Bounds {
            origin: point(px(0.), px(0.)),
            size: size(px(64.), px(64.)),
        },
        transform: CanvasTransform::IDENTITY,
        hit: None,
    };
    canvas.update(cx, |state, cx| {
        state.dispatch_tool_event(cx, |tool, state, cx| {
            tool.pointer_down(&event, &tool_ctx, state, cx);
            tool.pointer_up(&event, &tool_ctx, state, cx);
        });
    });
}

fn committed_stroke(page: &gpui_kit::Entity<ImageEditorPage>, cx: &gpui_kit::App) -> StrokeStyle {
    let view = page.read(cx);
    let layer = view
        .doc
        .read(cx)
        .annotation_layer()
        .expect("annotation layer mounted");
    view.canvas
        .read(cx)
        .document()
        .layer(layer)
        .and_then(|layer| layer.shapes().last())
        .map(|(_, shape)| match shape {
            CanvasShape::Path { style, .. } => style.clone(),
            other => panic!("expected a committed path, got {other:?}"),
        })
        .expect("the brush tap committed a path")
}

#[gpui_kit::test]
fn brush_size_release_updates_tool_params(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let (page, cx) = cx.add_window_view(ImageEditorPage::new);
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
    commit_brush_tap(&page, cx, (4., 4.));
    cx.run_until_parked();
    cx.read(|cx| {
        assert_eq!(
            page.read(cx).canvas.read(cx).active_tool_id(),
            "image-brush"
        );
        assert_eq!(page.read(cx).brush_value, 9.);
        let stroke = committed_stroke(&page, cx);
        assert_eq!(
            stroke.width,
            px(9.),
            "the remounted brush must stroke with the released width"
        );
    });
}

#[gpui_kit::test]
fn color_change_updates_tool_params(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let (page, cx) = cx.add_window_view(ImageEditorPage::new);
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
    commit_brush_tap(&page, cx, (4., 4.));
    cx.run_until_parked();
    cx.read(|cx| {
        let view = page.read(cx);
        assert_eq!(view.color_value, color);
        assert_eq!(view.canvas.read(cx).active_tool_id(), "image-brush");
        assert_eq!(
            committed_stroke(&page, cx).color,
            color,
            "the remounted brush must stroke with the picked color"
        );
    });
}

#[gpui_kit::test]
fn undo_resyncs_filter_controls_with_the_document(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let (page, cx) = cx.add_window_view(ImageEditorPage::new);
    let canvas = page.update(cx, |page, _| page.canvas.clone());
    page.update(cx, |page, cx| {
        page.doc
            .update(cx, |doc, cx| doc.open(png_bytes(16, 8), &canvas, cx));
    });
    cx.run_until_parked();

    page.update(cx, |page, cx| {
        page.doc.update(cx, |doc, cx| {
            doc.set_filters(
                FilterSettings {
                    grayscale: true,
                    blur_sigma: 4.,
                    brightness: 12,
                },
                cx,
            )
        });
    });
    cx.run_until_parked();
    page.update(cx, |page, cx| {
        assert!(page.gray);
        assert_eq!(page.blur.read(cx).value().start(), 4.);
        assert_eq!(page.brightness.read(cx).value().start(), 12.);
        page.doc.update(cx, |doc, cx| doc.undo(&canvas, cx));
    });
    cx.run_until_parked();
    page.update(cx, |page, cx| {
        assert!(!page.gray, "undo reverts the grayscale switch");
        assert_eq!(page.blur.read(cx).value().start(), 0.);
        assert_eq!(page.brightness.read(cx).value().start(), 0.);
        page.gray = true;
        page.apply_filters(cx);
        let filters = page.doc.read(cx).filters();
        assert_eq!(
            filters.blur_sigma, 0.,
            "the undone blur must not resurrect as a new op"
        );
        assert_eq!(filters.brightness, 0);
        assert!(filters.grayscale);
        assert_eq!(page.doc.read(cx).history_len(), 1);
    });
}

#[gpui_kit::test]
fn leaving_the_crop_tool_clears_the_crop_rect(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let (page, cx) = cx.add_window_view(ImageEditorPage::new);
    let canvas = page.update(cx, |page, _| page.canvas.clone());
    page.update(cx, |page, cx| {
        page.doc
            .update(cx, |doc, cx| doc.open(png_bytes(16, 8), &canvas, cx));
    });
    cx.run_until_parked();

    page.update(cx, |page, cx| {
        page.select_tool(EditorTool::Crop, cx);
        page.doc.update(cx, |doc, cx| {
            doc.set_crop_rect(
                Some(Bounds::from_corners(
                    point(px(1.), px(1.)),
                    point(px(9.), px(7.)),
                )),
                cx,
            )
        });
        assert!(page.doc.read(cx).crop_rect().is_some());
        page.select_tool(EditorTool::Brush, cx);
        assert!(
            page.doc.read(cx).crop_rect().is_none(),
            "switching tools must not keep an invisible crop rect"
        );
    });
}

#[gpui_kit::test]
fn failed_apply_crop_drops_the_invalid_rect(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let (page, cx) = cx.add_window_view(ImageEditorPage::new);
    let canvas = page.update(cx, |page, _| page.canvas.clone());
    page.update(cx, |page, cx| {
        page.doc
            .update(cx, |doc, cx| doc.open(png_bytes(16, 8), &canvas, cx));
    });
    cx.run_until_parked();

    cx.update(|window, cx| {
        page.update(cx, |page, cx| {
            page.select_tool(EditorTool::Crop, cx);
            page.doc.update(cx, |doc, cx| {
                doc.set_crop_rect(
                    // Entirely outside the 16x8 image: apply must reject it.
                    Some(Bounds::from_corners(
                        point(px(40.), px(40.)),
                        point(px(60.), px(60.)),
                    )),
                    cx,
                )
            });
            page.on_apply_crop(window, cx);
            assert!(
                page.doc.read(cx).crop_rect().is_none(),
                "a rejected rect is dropped, not kept for a later apply"
            );
        });
    });
}

#[gpui_kit::test]
fn save_captures_edits_made_while_the_dialog_is_open(cx: &mut TestAppContext) {
    let (page, mut cx) = open_page(cx);
    cx.run_until_parked();

    let file = std::env::temp_dir().join(format!(
        "gpui-starter-image-editor-save-{}.png",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&file);
    page.update(&mut cx, |page, cx| {
        let canvas = page.canvas.clone();
        page.doc
            .update(cx, |doc, cx| doc.open(png_bytes(24, 12), &canvas, cx));
    });
    cx.run_until_parked();

    cx.update(|window, cx| {
        page.update(cx, |page, cx| page.on_save(window, cx));
    });
    cx.run_until_parked();
    assert!(
        cx.read(|cx| matches!(page.read(cx).save_state, SaveState::Busy)),
        "the guard is raised before the dialog resolves"
    );

    page.update(&mut cx, |page, cx| {
        page.doc.update(cx, |doc, cx| {
            doc.set_filters(
                FilterSettings {
                    grayscale: true,
                    ..Default::default()
                },
                cx,
            )
        });
    });
    cx.run_until_parked();

    let picked = file.clone();
    cx.simulate_new_path_selection(move |_| Some(picked));
    cx.run_until_parked();

    let saved = ImageReader::open(&file)
        .unwrap()
        .decode()
        .unwrap()
        .to_rgba8();
    let _ = std::fs::remove_file(&file);
    page.update(&mut cx, |page, cx| {
        let doc = page.doc.read(cx);
        let expected = super::bake::compose_export(
            doc.base().unwrap(),
            &doc.filters(),
            &doc.annotation_shapes(),
        );
        assert_eq!(saved.dimensions(), expected.dimensions());
        assert_eq!(
            saved.as_raw(),
            expected.as_raw(),
            "the grayscale edit made during the dialog is exported"
        );
    });
    assert!(cx.read(|cx| matches!(page.read(cx).save_state, SaveState::Done)));
}

/// A prompt path the encoder cannot write must land in Failed, not Done.
#[gpui_kit::test]
fn save_to_an_unwritable_path_reports_failed(cx: &mut TestAppContext) {
    let (page, mut cx) = open_page(cx);
    cx.run_until_parked();
    page.update(&mut cx, |page, cx| {
        let canvas = page.canvas.clone();
        page.doc
            .update(cx, |doc, cx| doc.open(png_bytes(24, 12), &canvas, cx));
    });
    cx.run_until_parked();

    cx.update(|window, cx| {
        page.update(cx, |page, cx| page.on_save(window, cx));
    });
    cx.run_until_parked();
    // No parent directory: File::create fails inside save_image.
    let dir = std::env::temp_dir().join(format!(
        "gpui-starter-image-editor-missing-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let missing = dir.join("saved.png");
    cx.simulate_new_path_selection(move |_| Some(missing));
    cx.run_until_parked();

    cx.read(|cx| match &page.read(cx).save_state {
        SaveState::Failed(err) => {
            assert!(!err.is_empty(), "the io error message must surface");
        }
        other => panic!("an unwritable save target must fail, got {other:?}"),
    });
}

#[gpui_kit::test]
fn reopening_an_image_resets_filter_controls(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let (page, cx) = cx.add_window_view(ImageEditorPage::new);
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
                FilterSettings {
                    grayscale: true,
                    blur_sigma: 3.,
                    brightness: 20,
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
}
