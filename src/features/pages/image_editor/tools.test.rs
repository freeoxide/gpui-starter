use std::io::Cursor;

use gpui_kit::{
    AppContext, Bounds, Context, CursorStyle, Entity, Modifiers, MouseButton, Pixels,
    TestAppContext, black, point, px, size,
};
use image::{DynamicImage, ImageFormat, RgbaImage};

use super::{
    ArrowTool, BrushTool, CropTool, EllipseTool, EraserTool, RectTool, arrow_segments,
    bounds_from_corners,
};
use crate::features::pages::image_editor::document::{ImageEditorDocument, capture_annotations};
use crate::features::pages::image_editor::history::EditLabel;
use crate::ui::canvas::{
    CanvasPointerEvent, CanvasShape, CanvasState, CanvasTool, CanvasTransform, LayerId,
    StrokeStyle, ToolContext,
};

fn stroke() -> StrokeStyle {
    StrokeStyle {
        width: px(2.),
        color: black(),
        dash: None,
    }
}

fn png_bytes(w: u32, h: u32) -> Vec<u8> {
    let img = RgbaImage::from_pixel(w, h, image::Rgba([120, 140, 160, 255]));
    let mut bytes = Vec::new();
    DynamicImage::ImageRgba8(img)
        .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
        .unwrap();
    bytes
}

fn pointer(doc: (f32, f32), button: MouseButton) -> CanvasPointerEvent {
    let position = point(px(doc.0), px(doc.1));
    CanvasPointerEvent {
        screen_position: position,
        doc_position: position,
        button,
        click_count: 1,
        modifiers: Modifiers::default(),
    }
}

fn tool_ctx() -> ToolContext {
    ToolContext {
        viewport: Bounds {
            origin: point(px(0.), px(0.)),
            size: size(px(1000.), px(1000.)),
        },
        transform: CanvasTransform::IDENTITY,
        hit: None,
    }
}

type Phase = fn(
    &mut dyn CanvasTool,
    &CanvasPointerEvent,
    &ToolContext,
    &mut CanvasState,
    &mut Context<CanvasState>,
);

fn dispatch(canvas: &Entity<CanvasState>, cx: &mut TestAppContext, phase: Phase, at: (f32, f32)) {
    let event = pointer(at, MouseButton::Left);
    let tool_ctx = tool_ctx();
    canvas.update(cx, |state, cx| {
        state.dispatch_tool_event(cx, |tool, state, cx| {
            phase(tool, &event, &tool_ctx, state, cx)
        });
    });
}

fn phase_down(
    tool: &mut dyn CanvasTool,
    event: &CanvasPointerEvent,
    tool_ctx: &ToolContext,
    state: &mut CanvasState,
    cx: &mut Context<CanvasState>,
) {
    tool.pointer_down(event, tool_ctx, state, cx);
}

fn phase_move(
    tool: &mut dyn CanvasTool,
    event: &CanvasPointerEvent,
    tool_ctx: &ToolContext,
    state: &mut CanvasState,
    cx: &mut Context<CanvasState>,
) {
    tool.pointer_move(event, tool_ctx, state, cx);
}

fn phase_up(
    tool: &mut dyn CanvasTool,
    event: &CanvasPointerEvent,
    tool_ctx: &ToolContext,
    state: &mut CanvasState,
    cx: &mut Context<CanvasState>,
) {
    tool.pointer_up(event, tool_ctx, state, cx);
}

fn harness(
    cx: &mut TestAppContext,
    build: impl FnOnce(Entity<ImageEditorDocument>, LayerId) -> Box<dyn CanvasTool>,
) -> (Entity<CanvasState>, Entity<ImageEditorDocument>, LayerId) {
    let canvas = cx.new(|_| CanvasState::new());
    let doc = cx.new(|_| ImageEditorDocument::new());
    doc.update(cx, |doc, cx| doc.open(png_bytes(16, 8), &canvas, cx));
    cx.executor().run_until_parked();
    let layer = cx.read(|cx| doc.read(cx).annotation_layer()).unwrap();
    let tool = build(doc.clone(), layer);
    canvas.update(cx, |state, cx| state.set_tool(tool, cx));
    (canvas, doc, layer)
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

fn assert_px_eq(actual: Pixels, expected: f32) {
    assert!(
        (actual.as_f32() - expected).abs() < 0.01,
        "expected ~{expected}, got {}",
        actual.as_f32()
    );
}

#[gpui_kit::test]
fn brush_drag_commits_one_undoable_path(cx: &mut TestAppContext) {
    let (canvas, doc, layer) = harness(cx, |doc, layer| {
        Box::new(BrushTool::new(doc, layer, stroke()))
    });

    dispatch(&canvas, cx, phase_down, (2., 2.));
    dispatch(&canvas, cx, phase_move, (6., 2.));
    dispatch(&canvas, cx, phase_move, (10., 4.));
    dispatch(&canvas, cx, phase_up, (10., 4.));

    assert!(cx.read(|cx| canvas.read(cx).tool_overlay().is_none()));
    let shapes = layer_shapes(&canvas, layer, cx);
    assert_eq!(shapes.len(), 1);
    match &shapes[0] {
        CanvasShape::Path { points, .. } => assert_eq!(points.len(), 3),
        other => panic!("expected a path, got {other:?}"),
    }
    assert_eq!(cx.read(|cx| doc.read(cx).history_len()), 1);

    doc.update(cx, |doc, cx| doc.undo(&canvas, cx));
    assert_eq!(layer_shapes(&canvas, layer, cx).len(), 0);
    doc.update(cx, |doc, cx| doc.redo(&canvas, cx));
    assert_eq!(layer_shapes(&canvas, layer, cx).len(), 1);
}

#[gpui_kit::test]
fn brush_tap_commits_a_single_point_dot(cx: &mut TestAppContext) {
    let (canvas, _doc, layer) = harness(cx, |doc, layer| {
        Box::new(BrushTool::new(doc, layer, stroke()))
    });

    dispatch(&canvas, cx, phase_down, (5., 5.));
    dispatch(&canvas, cx, phase_up, (5., 5.));

    let shapes = layer_shapes(&canvas, layer, cx);
    assert_eq!(shapes.len(), 1);
    match &shapes[0] {
        CanvasShape::Path { points, .. } => assert_eq!(points.len(), 1),
        other => panic!("expected a path, got {other:?}"),
    }
}

#[gpui_kit::test]
fn rect_drag_commits_normalized_bounds(cx: &mut TestAppContext) {
    let (canvas, doc, layer) = harness(cx, |doc, layer| {
        Box::new(RectTool::new(doc, layer, stroke()))
    });

    dispatch(&canvas, cx, phase_down, (7., 5.));
    dispatch(&canvas, cx, phase_move, (2., 2.));
    dispatch(&canvas, cx, phase_up, (2., 2.));

    assert!(cx.read(|cx| canvas.read(cx).tool_overlay().is_none()));
    let shapes = layer_shapes(&canvas, layer, cx);
    assert_eq!(shapes.len(), 1);
    match &shapes[0] {
        CanvasShape::Rect { bounds, .. } => {
            assert_px_eq(bounds.origin.x, 2.);
            assert_px_eq(bounds.origin.y, 2.);
            assert_px_eq(bounds.size.width, 5.);
            assert_px_eq(bounds.size.height, 3.);
        }
        other => panic!("expected a rect, got {other:?}"),
    }
    assert_eq!(cx.read(|cx| doc.read(cx).history_len()), 1);
}

#[gpui_kit::test]
fn ellipse_drag_commits_corner_bounds(cx: &mut TestAppContext) {
    let (canvas, doc, layer) = harness(cx, |doc, layer| {
        Box::new(EllipseTool::new(doc, layer, stroke()))
    });

    dispatch(&canvas, cx, phase_down, (1., 1.));
    dispatch(&canvas, cx, phase_move, (9., 5.));
    dispatch(&canvas, cx, phase_up, (9., 5.));

    let shapes = layer_shapes(&canvas, layer, cx);
    assert_eq!(shapes.len(), 1);
    match &shapes[0] {
        CanvasShape::Ellipse { bounds, .. } => {
            assert_px_eq(bounds.origin.x, 1.);
            assert_px_eq(bounds.origin.y, 1.);
            assert_px_eq(bounds.size.width, 8.);
            assert_px_eq(bounds.size.height, 4.);
        }
        other => panic!("expected an ellipse, got {other:?}"),
    }
    assert_eq!(cx.read(|cx| doc.read(cx).history_len()), 1);
}

#[gpui_kit::test]
fn arrow_drag_commits_three_matching_paths(cx: &mut TestAppContext) {
    let (canvas, doc, layer) = harness(cx, |doc, layer| {
        Box::new(ArrowTool::new(doc, layer, stroke()))
    });

    dispatch(&canvas, cx, phase_down, (0., 0.));
    dispatch(&canvas, cx, phase_move, (10., 0.));
    dispatch(&canvas, cx, phase_up, (10., 0.));

    let shapes = layer_shapes(&canvas, layer, cx);
    assert_eq!(shapes.len(), 3);
    let expected = arrow_segments(point(px(0.), px(0.)), point(px(10.), px(0.)));
    for (shape, (a, b)) in shapes.iter().zip(expected) {
        match shape {
            CanvasShape::Path { points, .. } => {
                assert_eq!(points.len(), 2);
                assert_px_eq(points[0].x, a.x.as_f32());
                assert_px_eq(points[0].y, a.y.as_f32());
                assert_px_eq(points[1].x, b.x.as_f32());
                assert_px_eq(points[1].y, b.y.as_f32());
            }
            other => panic!("expected a path, got {other:?}"),
        }
    }
    assert_eq!(cx.read(|cx| doc.read(cx).history_len()), 1);
}

#[gpui_kit::test]
fn eraser_hit_removes_and_records_erase(cx: &mut TestAppContext) {
    let (canvas, doc, layer) = harness(cx, |doc, layer| Box::new(EraserTool::new(doc, layer, 4.)));
    canvas.update(cx, |state, cx| {
        state.add_shape(
            layer,
            CanvasShape::path(
                vec![point(px(0.), px(5.)), point(px(16.), px(5.))],
                stroke(),
            ),
            cx,
        );
    });
    doc.update(cx, |doc, cx| {
        let before = capture_annotations(canvas.read(cx), layer);
        doc.record_annotate(EditLabel::Stroke, Vec::new(), before, cx)
    });
    assert_eq!(layer_shapes(&canvas, layer, cx).len(), 1);

    dispatch(&canvas, cx, phase_down, (5., 5.));
    dispatch(&canvas, cx, phase_up, (5., 5.));

    assert_eq!(layer_shapes(&canvas, layer, cx).len(), 0);
    assert_eq!(cx.read(|cx| doc.read(cx).history_len()), 2);

    doc.update(cx, |doc, cx| doc.undo(&canvas, cx));
    assert_eq!(layer_shapes(&canvas, layer, cx).len(), 1);
}

#[gpui_kit::test]
fn eraser_miss_records_nothing(cx: &mut TestAppContext) {
    let (canvas, doc, layer) = harness(cx, |doc, layer| Box::new(EraserTool::new(doc, layer, 2.)));
    canvas.update(cx, |state, cx| {
        state.add_shape(
            layer,
            CanvasShape::path(
                vec![point(px(0.), px(5.)), point(px(16.), px(5.))],
                stroke(),
            ),
            cx,
        );
    });
    doc.update(cx, |doc, cx| {
        let before = capture_annotations(canvas.read(cx), layer);
        doc.record_annotate(EditLabel::Stroke, Vec::new(), before, cx)
    });

    dispatch(&canvas, cx, phase_down, (5., 1.));
    dispatch(&canvas, cx, phase_up, (5., 1.));

    assert_eq!(layer_shapes(&canvas, layer, cx).len(), 1);
    assert_eq!(cx.read(|cx| doc.read(cx).history_len()), 1);
}

#[gpui_kit::test]
fn crop_drag_sets_crop_rect_without_history(cx: &mut TestAppContext) {
    let (canvas, doc, _layer) = harness(cx, |doc, _| Box::new(CropTool::new(doc, black())));

    dispatch(&canvas, cx, phase_down, (9., 9.));
    dispatch(&canvas, cx, phase_move, (3., 1.));
    dispatch(&canvas, cx, phase_up, (3., 1.));

    assert!(cx.read(|cx| canvas.read(cx).tool_overlay().is_none()));
    let rect = cx
        .read(|cx| doc.read(cx).crop_rect())
        .expect("crop rect stored");
    assert_px_eq(rect.origin.x, 3.);
    assert_px_eq(rect.origin.y, 1.);
    assert_px_eq(rect.size.width, 6.);
    assert_px_eq(rect.size.height, 8.);
    assert_eq!(cx.read(|cx| doc.read(cx).history_len()), 0);
}

#[gpui_kit::test]
fn every_editor_tool_has_crosshair_cursor(cx: &mut TestAppContext) {
    let (canvas, doc, layer) = harness(cx, |doc, layer| {
        Box::new(BrushTool::new(doc, layer, stroke()))
    });
    assert_eq!(
        cx.read(|cx| canvas.read(cx).active_tool_id().to_string()),
        "image-brush"
    );
    assert_eq!(
        cx.read(|cx| canvas.read(cx).active_tool_cursor()),
        CursorStyle::Crosshair
    );

    let eraser = EraserTool::new(doc.clone(), layer, 12.);
    assert_eq!(eraser.id(), "image-eraser");
    assert_eq!(eraser.cursor(), CursorStyle::Crosshair);
    let rect = RectTool::new(doc.clone(), layer, stroke());
    assert_eq!(rect.id(), "image-rect");
    assert_eq!(rect.cursor(), CursorStyle::Crosshair);
    let ellipse = EllipseTool::new(doc.clone(), layer, stroke());
    assert_eq!(ellipse.id(), "image-ellipse");
    assert_eq!(ellipse.cursor(), CursorStyle::Crosshair);
    let arrow = ArrowTool::new(doc.clone(), layer, stroke());
    assert_eq!(arrow.id(), "image-arrow");
    assert_eq!(arrow.cursor(), CursorStyle::Crosshair);
    let crop = CropTool::new(doc.clone(), black());
    assert_eq!(crop.id(), "image-crop");
    assert_eq!(crop.cursor(), CursorStyle::Crosshair);
}

#[gpui_kit::test]
fn arrow_head_geometry_rotates_with_the_shaft() {
    let horizontal = arrow_segments(point(px(0.), px(0.)), point(px(10.), px(0.)));
    assert_px_eq(horizontal[0].0.x, 0.);
    assert_px_eq(horizontal[0].1.x, 10.);
    for segment in &horizontal[1..] {
        assert_px_eq(segment.0.x, 10.);
        assert_px_eq(segment.0.y, 0.);
        assert!(segment.1.x.as_f32() < 10., "head sweeps backward");
    }

    let vertical = arrow_segments(point(px(0.), px(0.)), point(px(0.), px(10.)));
    for segment in &vertical[1..] {
        assert_px_eq(segment.0.x, 0.);
        assert_px_eq(segment.0.y, 10.);
        assert!(
            segment.1.y.as_f32() < 10.,
            "head sweeps backward along the shaft"
        );
    }
    let head_x: Vec<f32> = vertical[1..].iter().map(|s| s.1.x.as_f32()).collect();
    assert!(
        head_x.iter().any(|x| *x <= 0.) && head_x.iter().any(|x| *x >= 0.),
        "heads straddle the shaft axis: {head_x:?}"
    );
}

#[gpui_kit::test]
fn bounds_from_corners_normalizes_reversed_corners() {
    let bounds = bounds_from_corners(point(px(8.), px(6.)), point(px(2.), px(0.)));
    assert_px_eq(bounds.origin.x, 2.);
    assert_px_eq(bounds.origin.y, 0.);
    assert_px_eq(bounds.size.width, 6.);
    assert_px_eq(bounds.size.height, 6.);
}
