use std::cell::RefCell;
use std::io::Cursor;
use std::rc::Rc;

use gpui_kit::{AppContext, Bounds, Entity, Subscription, TestAppContext, point, px, size};
use image::{DynamicImage, ImageFormat, RgbaImage};

use super::{FilterSettings, ImageEditorDocument, ImageEditorEvent, rgba_to_render};
use crate::features::pages::image_editor::history::{AnnotationSnapshot, EditLabel};
use crate::ui::canvas::{CanvasShape, CanvasState, LayerId, ShapeId, ShapeStyle, StrokeStyle};

fn png_bytes(w: u32, h: u32) -> Vec<u8> {
    let img = RgbaImage::from_pixel(w, h, image::Rgba([120, 140, 160, 255]));
    let mut bytes = Vec::new();
    DynamicImage::ImageRgba8(img)
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .unwrap();
    bytes
}

fn rect(x: f32, y: f32, w: f32, h: f32) -> CanvasShape {
    CanvasShape::rect(
        Bounds {
            origin: point(px(x), px(y)),
            size: size(px(w), px(h)),
        },
        ShapeStyle::stroked(StrokeStyle::default()),
    )
}

fn opened_doc(cx: &mut TestAppContext) -> (Entity<CanvasState>, Entity<ImageEditorDocument>) {
    let canvas = cx.new(|_| CanvasState::new());
    let doc = cx.new(|_| ImageEditorDocument::new());
    doc.update(cx, |doc, cx| doc.open(png_bytes(16, 8), &canvas, cx));
    cx.executor().run_until_parked();
    (canvas, doc)
}

fn layer_shapes(
    canvas: &Entity<CanvasState>,
    layer: LayerId,
    cx: &TestAppContext,
) -> Vec<CanvasShape> {
    cx.read(|cx| {
        canvas
            .read(cx)
            .document()
            .layer(layer)
            .map(|l| l.shapes().iter().map(|(_, shape)| shape.clone()).collect())
            .unwrap_or_default()
    })
}

fn collect_events(
    cx: &TestAppContext,
    doc: &Entity<ImageEditorDocument>,
) -> (Rc<RefCell<Vec<ImageEditorEvent>>>, Subscription) {
    let events = Rc::new(RefCell::new(Vec::new()));
    let sink = events.clone();
    let subscription = cx.update(|cx| {
        cx.subscribe(doc, move |_, event: &ImageEditorEvent, _| {
            sink.borrow_mut().push(*event)
        })
    });
    (events, subscription)
}

#[gpui_kit::test]
fn rgba_to_render_round_trips_dimensions_and_bytes(cx: &mut TestAppContext) {
    cx.update(|_| {
        let img = RgbaImage::from_pixel(16, 8, image::Rgba([1, 2, 3, 255]));
        let render = rgba_to_render(&img);
        assert_eq!(render.size(0).width.0, 16);
        assert_eq!(render.size(0).height.0, 8);
        assert_eq!(render.as_bytes(0).map(<[u8]>::len), Some(16 * 8 * 4));
    });
}

#[gpui_kit::test]
fn set_display_for_test_emits_exactly_one_display_changed(cx: &mut TestAppContext) {
    let doc = cx.new(|_| ImageEditorDocument::new());
    let (events, _subscription) = collect_events(cx, &doc);
    doc.update(cx, |doc, cx| {
        let img = RgbaImage::from_pixel(2, 2, image::Rgba([0, 0, 0, 255]));
        doc.set_display_for_test(rgba_to_render(&img), cx)
    });
    assert_eq!(events.borrow().len(), 1);
    assert_eq!(events.borrow()[0], ImageEditorEvent::DisplayChanged);
    assert!(cx.read(|cx| doc.read(cx).display().is_some()));
}

#[gpui_kit::test]
fn open_builds_two_layers_and_resets_state(cx: &mut TestAppContext) {
    let (canvas, doc) = opened_doc(cx);

    let (has_image, loading, base_size, layer, base_shape, history_len, layers) = cx.read(|cx| {
        let doc = doc.read(cx);
        (
            doc.has_image(),
            doc.is_loading(),
            doc.base_size(),
            doc.annotation_layer(),
            doc.base_shape(),
            doc.history_len(),
            canvas.read(cx).document().layers().len(),
        )
    });
    assert!(has_image);
    assert!(!loading);
    assert_eq!(base_size, Some((16, 8)));
    assert!(layer.is_some());
    assert!(base_shape.is_some());
    assert_eq!(history_len, 0);
    assert_eq!(layers, 2);
}

#[gpui_kit::test]
fn open_emits_image_opened_once(cx: &mut TestAppContext) {
    let canvas = cx.new(|_| CanvasState::new());
    let doc = cx.new(|_| ImageEditorDocument::new());
    let (events, _subscription) = collect_events(cx, &doc);
    doc.update(cx, |doc, cx| doc.open(png_bytes(16, 8), &canvas, cx));
    cx.executor().run_until_parked();
    let opened = events
        .borrow()
        .iter()
        .filter(|event| **event == ImageEditorEvent::ImageOpened)
        .count();
    assert_eq!(opened, 1);
}

#[gpui_kit::test]
fn open_rejects_garbage_bytes_and_records_last_error(cx: &mut TestAppContext) {
    let canvas = cx.new(|_| CanvasState::new());
    let doc = cx.new(|_| ImageEditorDocument::new());
    doc.update(cx, |doc, cx| doc.open(vec![0_u8; 16], &canvas, cx));
    cx.executor().run_until_parked();
    let (has_image, loading, error) = cx.read(|cx| {
        let doc = doc.read(cx);
        (
            doc.has_image(),
            doc.is_loading(),
            doc.last_error().is_some(),
        )
    });
    assert!(!has_image);
    assert!(!loading);
    assert!(error);
}

#[gpui_kit::test]
fn set_filters_records_one_undoable_entry(cx: &mut TestAppContext) {
    let (canvas, doc) = opened_doc(cx);
    doc.update(cx, |doc, cx| {
        doc.set_filters(
            FilterSettings {
                grayscale: true,
                ..FilterSettings::default()
            },
            cx,
        )
    });
    assert!(cx.read(|cx| doc.read(cx).can_undo()));
    assert_eq!(cx.read(|cx| doc.read(cx).history_len()), 1);

    doc.update(cx, |doc, cx| doc.undo(&canvas, cx));
    assert_eq!(
        cx.read(|cx| doc.read(cx).filters()),
        FilterSettings::default()
    );
    assert!(!cx.read(|cx| doc.read(cx).can_undo()));
}

#[gpui_kit::test]
fn set_filters_with_identical_values_records_nothing(cx: &mut TestAppContext) {
    let (_canvas, doc) = opened_doc(cx);
    doc.update(cx, |doc, cx| doc.set_filters(FilterSettings::default(), cx));
    assert_eq!(cx.read(|cx| doc.read(cx).history_len()), 0);
}

fn seed_annotation(
    cx: &mut TestAppContext,
    canvas: &Entity<CanvasState>,
    doc: &Entity<ImageEditorDocument>,
    layer: LayerId,
    shape: CanvasShape,
) {
    canvas.update(cx, |state, cx| {
        state.add_shape(layer, shape, cx);
    });
    let after: AnnotationSnapshot = cx.read(|cx| {
        canvas
            .read(cx)
            .document()
            .layer(layer)
            .map(|l| l.shapes().to_vec())
            .unwrap_or_default()
    });
    doc.update(cx, |doc, cx| {
        doc.record_annotate(EditLabel::Stroke, Vec::new(), after, cx)
    });
}

#[gpui_kit::test]
fn rotate90_swaps_dimensions_and_rotates_annotations(cx: &mut TestAppContext) {
    let (canvas, doc) = opened_doc(cx);
    let layer = cx.read(|cx| doc.read(cx).annotation_layer()).unwrap();
    seed_annotation(cx, &canvas, &doc, layer, rect(0., 0., 2., 1.));

    doc.update(cx, |doc, cx| doc.rotate90(&canvas, cx));

    assert_eq!(cx.read(|cx| doc.read(cx).base_size()), Some((8, 16)));
    let shapes = layer_shapes(&canvas, layer, cx);
    assert_eq!(shapes.len(), 1);
    let Bounds { origin, size } = shapes[0].doc_bounds();
    assert!((origin.x.as_f32() - 7.).abs() < 0.01, "{origin:?}");
    assert!((origin.y.as_f32() - 0.).abs() < 0.01, "{origin:?}");
    assert!((size.width.as_f32() - 1.).abs() < 0.01, "{size:?}");
    assert!((size.height.as_f32() - 2.).abs() < 0.01, "{size:?}");
}

#[gpui_kit::test]
fn apply_crop_cuts_pixels_and_translates_annotations(cx: &mut TestAppContext) {
    let (canvas, doc) = opened_doc(cx);
    let layer = cx.read(|cx| doc.read(cx).annotation_layer()).unwrap();
    seed_annotation(cx, &canvas, &doc, layer, rect(6., 3., 4., 4.));

    let crop = Bounds {
        origin: point(px(4.), px(0.)),
        size: size(px(8.), px(8.)),
    };
    doc.update(cx, |doc, cx| doc.set_crop_rect(Some(crop), cx));
    assert!(cx.read(|cx| doc.read(cx).has_valid_crop()));

    let applied = doc.update(cx, |doc, cx| doc.apply_crop(&canvas, cx));
    assert!(applied);
    assert_eq!(cx.read(|cx| doc.read(cx).base_size()), Some((8, 8)));
    assert!(!cx.read(|cx| doc.read(cx).has_valid_crop()));

    let shapes = layer_shapes(&canvas, layer, cx);
    assert_eq!(shapes.len(), 1);
    let bounds = shapes[0].doc_bounds();
    assert!((bounds.origin.x.as_f32() - 2.).abs() < 0.01, "{bounds:?}");
    assert!((bounds.origin.y.as_f32() - 3.).abs() < 0.01, "{bounds:?}");
}

#[gpui_kit::test]
fn apply_crop_rejects_under_minimum_rect(cx: &mut TestAppContext) {
    let (canvas, doc) = opened_doc(cx);
    let crop = Bounds {
        origin: point(px(2.), px(1.)),
        size: size(px(4.), px(2.)),
    };
    doc.update(cx, |doc, cx| doc.set_crop_rect(Some(crop), cx));
    assert!(!cx.read(|cx| doc.read(cx).has_valid_crop()));
    let applied = doc.update(cx, |doc, cx| doc.apply_crop(&canvas, cx));
    assert!(!applied);
    assert_eq!(cx.read(|cx| doc.read(cx).base_size()), Some((16, 8)));
}

#[gpui_kit::test]
fn undo_after_crop_restores_the_full_image(cx: &mut TestAppContext) {
    let (canvas, doc) = opened_doc(cx);
    let crop = Bounds {
        origin: point(px(0.), px(0.)),
        size: size(px(10.), px(8.)),
    };
    doc.update(cx, |doc, cx| doc.set_crop_rect(Some(crop), cx));
    doc.update(cx, |doc, cx| doc.apply_crop(&canvas, cx));
    assert_eq!(cx.read(|cx| doc.read(cx).base_size()), Some((10, 8)));

    doc.update(cx, |doc, cx| doc.undo(&canvas, cx));
    assert_eq!(cx.read(|cx| doc.read(cx).base_size()), Some((16, 8)));

    doc.update(cx, |doc, cx| doc.redo(&canvas, cx));
    assert_eq!(cx.read(|cx| doc.read(cx).base_size()), Some((10, 8)));
}

#[gpui_kit::test]
fn eraser_annotate_undo_redo_round_trip(cx: &mut TestAppContext) {
    let (canvas, doc) = opened_doc(cx);
    let layer = cx.read(|cx| doc.read(cx).annotation_layer()).unwrap();
    seed_annotation(cx, &canvas, &doc, layer, rect(0., 0., 4., 4.));
    assert_eq!(layer_shapes(&canvas, layer, cx).len(), 1);
    let before: AnnotationSnapshot = cx.read(|cx| {
        canvas
            .read(cx)
            .document()
            .layer(layer)
            .map(|l| l.shapes().to_vec())
            .unwrap_or_default()
    });

    canvas.update(cx, |state, cx| {
        let ids: Vec<ShapeId> = state
            .document()
            .layer(layer)
            .map(|l| l.shapes().iter().map(|(id, _)| *id).collect())
            .unwrap_or_default();
        for id in ids {
            state.remove_shape(id, cx);
        }
    });
    doc.update(cx, |doc, cx| {
        doc.record_annotate(EditLabel::Erase, before, Vec::new(), cx)
    });
    assert_eq!(layer_shapes(&canvas, layer, cx).len(), 0);

    doc.update(cx, |doc, cx| doc.undo(&canvas, cx));
    assert!(cx.read(|cx| doc.read(cx).can_redo()));
    assert_eq!(layer_shapes(&canvas, layer, cx).len(), 1);

    doc.update(cx, |doc, cx| doc.redo(&canvas, cx));
    assert_eq!(layer_shapes(&canvas, layer, cx).len(), 0);
}

#[gpui_kit::test]
fn open_resets_history_from_a_previous_image(cx: &mut TestAppContext) {
    let (canvas, doc) = opened_doc(cx);
    doc.update(cx, |doc, cx| {
        doc.set_filters(
            FilterSettings {
                brightness: 40,
                ..FilterSettings::default()
            },
            cx,
        )
    });
    assert!(cx.read(|cx| doc.read(cx).can_undo()));

    doc.update(cx, |doc, cx| doc.open(png_bytes(4, 4), &canvas, cx));
    cx.executor().run_until_parked();
    assert_eq!(cx.read(|cx| doc.read(cx).history_len()), 0);
    assert_eq!(cx.read(|cx| doc.read(cx).base_size()), Some((4, 4)));
}
