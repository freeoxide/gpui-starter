use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use gpui_kit::*;

use image::RgbaImage;

use super::bake::apply_filters_until;
use super::history::{AnnotationSnapshot, DocSnapshot, EditLabel, EditOp, EditorHistory};
use crate::ui::canvas::{CanvasShape, CanvasState, LayerId, ShapeId, rgba_to_render};

/// Minimum crop size in doc px per axis.
pub(crate) const MIN_CROP_PX: Pixels = px(8.);

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub(crate) struct FilterSettings {
    pub(crate) grayscale: bool,
    pub(crate) blur_sigma: f32,
    pub(crate) brightness: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ImageEditorEvent {
    ImageOpened,
    DisplayChanged,
    HistoryChanged,
    CropRectChanged,
}

pub(crate) fn capture_annotations(state: &CanvasState, layer: LayerId) -> AnnotationSnapshot {
    state
        .document()
        .layer(layer)
        .map(|l| {
            l.shapes()
                .iter()
                .map(|(id, shape)| (*id, shape.clone()))
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) struct ImageEditorDocument {
    base: Option<Arc<RgbaImage>>,
    filters: FilterSettings,
    display: Option<Arc<RenderImage>>,
    display_request: Arc<AtomicU64>,
    crop_rect: Option<Bounds<Pixels>>,
    history: EditorHistory,
    loading: bool,
    last_error: Option<String>,
    annotation_layer: Option<LayerId>,
    base_shape: Option<ShapeId>,
    annotations: AnnotationSnapshot,
    _derive: Option<Task<()>>,
    _open: Option<Task<()>>,
    _transform: Option<Task<()>>,
}

impl EventEmitter<ImageEditorEvent> for ImageEditorDocument {}

impl ImageEditorDocument {
    pub(crate) fn new() -> Self {
        Self {
            base: None,
            filters: FilterSettings::default(),
            display: None,
            display_request: Arc::new(AtomicU64::new(0)),
            crop_rect: None,
            history: EditorHistory::new(),
            loading: false,
            last_error: None,
            annotation_layer: None,
            base_shape: None,
            annotations: Vec::new(),
            _derive: None,
            _open: None,
            _transform: None,
        }
    }

    pub(crate) fn is_loading(&self) -> bool {
        self.loading
    }

    pub(crate) fn has_image(&self) -> bool {
        self.base.is_some()
    }

    pub(crate) fn filters(&self) -> FilterSettings {
        self.filters
    }

    #[cfg(test)]
    pub(crate) fn crop_rect(&self) -> Option<Bounds<Pixels>> {
        self.crop_rect
    }

    pub(crate) fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub(crate) fn can_redo(&self) -> bool {
        self.history.can_redo()
    }

    #[cfg(test)]
    pub(crate) fn history_len(&self) -> usize {
        self.history.len()
    }

    pub(crate) fn display(&self) -> Option<&Arc<RenderImage>> {
        self.display.as_ref()
    }

    pub(crate) fn base_size(&self) -> Option<(u32, u32)> {
        self.base.as_ref().map(|base| base.dimensions())
    }

    #[cfg(not(target_family = "wasm"))]
    pub(crate) fn base(&self) -> Option<&Arc<RgbaImage>> {
        self.base.as_ref()
    }

    pub(crate) fn annotation_layer(&self) -> Option<LayerId> {
        self.annotation_layer
    }

    pub(crate) fn base_shape(&self) -> Option<ShapeId> {
        self.base_shape
    }

    #[cfg(not(target_family = "wasm"))]
    pub(crate) fn annotation_shapes(&self) -> Vec<CanvasShape> {
        self.annotations
            .iter()
            .map(|(_, shape)| shape.clone())
            .collect()
    }

    pub(crate) fn last_error(&self) -> Option<&str> {
        self.last_error.as_deref()
    }

    pub(crate) fn has_valid_crop(&self) -> bool {
        self.base_size()
            .zip(self.crop_rect)
            .is_some_and(|(dims, rect)| clipped_crop(dims, rect).is_some())
    }

    #[cfg(not(target_family = "wasm"))]
    pub(crate) fn open(
        &mut self,
        bytes: Vec<u8>,
        canvas: &Entity<CanvasState>,
        cx: &mut Context<Self>,
    ) {
        let canvas = canvas.clone();
        self.loading = true;
        self.last_error = None;
        cx.notify();
        self._open = Some(cx.spawn(async move |this, cx| {
            let decoded = cx
                .background_spawn(async move {
                    image::load_from_memory(&bytes).map(|img| img.to_rgba8())
                })
                .await;
            this.update(cx, |doc, cx| {
                doc.loading = false;
                match decoded {
                    Ok(img) => doc.apply_opened(img, &canvas, cx),
                    Err(err) => {
                        doc.last_error = Some(err.to_string());
                        cx.notify();
                    }
                }
            })
            .ok();
        }));
    }

    #[cfg(not(target_family = "wasm"))]
    fn apply_opened(
        &mut self,
        img: RgbaImage,
        canvas: &Entity<CanvasState>,
        cx: &mut Context<Self>,
    ) {
        let (w, h) = img.dimensions();
        // One conversion seeds the base shape and the display; a default-
        // filter derive would rebuild pixel-identical output.
        let display = rgba_to_render(img.clone());
        self.filters = FilterSettings::default();
        self.history.clear();
        self.crop_rect = None;
        self.last_error = None;
        self.annotations = Vec::new();
        self.base = Some(Arc::new(img));
        let (annotation_layer, base_shape) = canvas.update(cx, |state, cx| {
            state.edit_document(cx, |doc| {
                doc.clear();
                let base_layer = doc.add_layer();
                let annotation_layer = doc.add_layer();
                let bounds = Bounds {
                    origin: Point::default(),
                    size: size(px(w as f32), px(h as f32)),
                };
                let base_shape =
                    doc.add_shape(base_layer, CanvasShape::image(bounds, display.clone()));
                (annotation_layer, base_shape)
            })
        });
        self.annotation_layer = Some(annotation_layer);
        self.base_shape = base_shape;
        self.display = Some(display);
        cx.emit(ImageEditorEvent::ImageOpened);
        cx.notify();
    }

    pub(crate) fn set_filters(&mut self, filters: FilterSettings, cx: &mut Context<Self>) {
        if filters == self.filters || !self.has_image() {
            return;
        }
        let before = self.snapshot();
        self.filters = filters;
        let after = self.snapshot();
        self.history.record(EditOp {
            label: EditLabel::Filters,
            before,
            after,
        });
        self.request_display(cx);
        cx.emit(ImageEditorEvent::HistoryChanged);
        cx.notify();
    }

    pub(crate) fn rotate90(&mut self, canvas: &Entity<CanvasState>, cx: &mut Context<Self>) {
        let Some(base) = self.base.clone() else {
            return;
        };
        let before = self.snapshot();
        let old_height = base.dimensions().1 as f32;
        let rotated: AnnotationSnapshot = before
            .annotations
            .iter()
            .map(|(id, shape)| (*id, rotate_shape(shape, old_height)))
            .collect();
        self.crop_rect = None;
        let canvas = canvas.clone();
        self._transform = Some(cx.spawn(async move |this, cx| {
            let after_base = cx
                .background_spawn(async move { image::imageops::rotate90(&*base) })
                .await;
            this.update(cx, |doc, cx| {
                let after_base = Arc::new(after_base);
                doc.base = Some(after_base.clone());
                doc.rebuild_annotations(&canvas, &rotated, cx);
                doc.annotations = rotated.clone();
                doc.history.record(EditOp {
                    label: EditLabel::Rotate,
                    before,
                    after: DocSnapshot {
                        base: after_base,
                        filters: doc.filters,
                        annotations: rotated,
                    },
                });
                doc.request_display(cx);
                cx.emit(ImageEditorEvent::HistoryChanged);
                cx.notify();
            })
            .ok();
        }));
    }

    pub(crate) fn apply_crop(
        &mut self,
        canvas: &Entity<CanvasState>,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(base) = self.base.clone() else {
            return false;
        };
        let Some(rect) = self.crop_rect else {
            return false;
        };
        let Some((x0, y0, cw, ch)) = clipped_crop(base.dimensions(), rect) else {
            return false;
        };
        let before = self.snapshot();
        let crop_bounds = Bounds {
            origin: point(px(x0 as f32), px(y0 as f32)),
            size: size(px(cw as f32), px(ch as f32)),
        };
        let cropped_bounds = Bounds {
            origin: Point::default(),
            size: crop_bounds.size,
        };
        let survivors: AnnotationSnapshot = before
            .annotations
            .iter()
            .filter(|(_, shape)| {
                let overlap = shape.doc_bounds().intersect(&crop_bounds);
                overlap.size.width > px(0.) && overlap.size.height > px(0.)
            })
            .map(|(id, shape)| {
                let mut shape = shape.clone();
                shape.translate(point(-crop_bounds.origin.x, -crop_bounds.origin.y));
                (*id, clamp_shape(&shape, cropped_bounds))
            })
            .collect();
        self.crop_rect = None;
        cx.emit(ImageEditorEvent::CropRectChanged);
        let canvas = canvas.clone();
        self._transform = Some(cx.spawn(async move |this, cx| {
            let after_base = cx
                .background_spawn(async move {
                    image::imageops::crop_imm(&*base, x0, y0, cw, ch).to_image()
                })
                .await;
            this.update(cx, |doc, cx| {
                let after_base = Arc::new(after_base);
                doc.base = Some(after_base.clone());
                doc.rebuild_annotations(&canvas, &survivors, cx);
                doc.annotations = survivors.clone();
                doc.history.record(EditOp {
                    label: EditLabel::Crop,
                    before,
                    after: DocSnapshot {
                        base: after_base,
                        filters: doc.filters,
                        annotations: survivors,
                    },
                });
                doc.request_display(cx);
                cx.emit(ImageEditorEvent::HistoryChanged);
                cx.notify();
            })
            .ok();
        }));
        true
    }

    pub(crate) fn set_crop_rect(&mut self, rect: Option<Bounds<Pixels>>, cx: &mut Context<Self>) {
        self.crop_rect = rect;
        cx.emit(ImageEditorEvent::CropRectChanged);
        cx.notify();
    }

    pub(crate) fn record_annotate(
        &mut self,
        label: EditLabel,
        before: AnnotationSnapshot,
        after: AnnotationSnapshot,
        cx: &mut Context<Self>,
    ) {
        self.annotations = after.clone();
        self.history.record(EditOp {
            label,
            before: DocSnapshot {
                base: self.current_base(),
                filters: self.filters,
                annotations: before,
            },
            after: DocSnapshot {
                base: self.current_base(),
                filters: self.filters,
                annotations: after,
            },
        });
        cx.emit(ImageEditorEvent::HistoryChanged);
        cx.notify();
    }

    pub(crate) fn undo(&mut self, canvas: &Entity<CanvasState>, cx: &mut Context<Self>) -> bool {
        let Some(op) = self.history.undo() else {
            return false;
        };
        let (label, before) = (op.label, op.before.clone());
        self.apply_snapshot(&before, canvas, cx);
        if matches!(label, EditLabel::Rotate | EditLabel::Crop) {
            self.crop_rect = None;
            cx.emit(ImageEditorEvent::CropRectChanged);
        }
        cx.emit(ImageEditorEvent::HistoryChanged);
        cx.notify();
        true
    }

    pub(crate) fn redo(&mut self, canvas: &Entity<CanvasState>, cx: &mut Context<Self>) -> bool {
        let Some(op) = self.history.redo() else {
            return false;
        };
        let (label, after) = (op.label, op.after.clone());
        self.apply_snapshot(&after, canvas, cx);
        if matches!(label, EditLabel::Rotate | EditLabel::Crop) {
            self.crop_rect = None;
            cx.emit(ImageEditorEvent::CropRectChanged);
        }
        cx.emit(ImageEditorEvent::HistoryChanged);
        cx.notify();
        true
    }

    #[cfg(test)]
    pub(crate) fn set_display_for_test(
        &mut self,
        display: Arc<RenderImage>,
        cx: &mut Context<Self>,
    ) {
        self.display = Some(display);
        cx.emit(ImageEditorEvent::DisplayChanged);
        cx.notify();
    }

    fn current_base(&self) -> Arc<RgbaImage> {
        self.base
            .clone()
            .unwrap_or_else(|| Arc::new(RgbaImage::new(0, 0)))
    }

    fn snapshot(&self) -> DocSnapshot {
        DocSnapshot {
            base: self.current_base(),
            filters: self.filters,
            annotations: self.annotations.clone(),
        }
    }

    fn apply_snapshot(
        &mut self,
        snap: &DocSnapshot,
        canvas: &Entity<CanvasState>,
        cx: &mut Context<Self>,
    ) {
        // Snapshots only ever carry Arcs cloned from self.base, so pointer
        // identity is sound and skips a whole-image content compare.
        let display_stale = self.base.as_ref().map(Arc::as_ptr) != Some(Arc::as_ptr(&snap.base))
            || self.filters != snap.filters;
        self.base = Some(snap.base.clone());
        self.filters = snap.filters;
        self.rebuild_annotations(canvas, &snap.annotations, cx);
        self.annotations = snap.annotations.clone();
        if display_stale {
            self.request_display(cx);
        }
    }

    fn rebuild_annotations(
        &mut self,
        canvas: &Entity<CanvasState>,
        annotations: &AnnotationSnapshot,
        cx: &mut Context<Self>,
    ) {
        let Some(layer) = self.annotation_layer else {
            return;
        };
        canvas.update(cx, |state, cx| {
            state.edit_document(cx, |doc| {
                let stale: Vec<ShapeId> = doc
                    .layer(layer)
                    .map(|l| l.shapes().iter().map(|(id, _)| *id).collect())
                    .unwrap_or_default();
                for id in stale {
                    doc.remove_shape(id);
                }
                for (_, shape) in annotations {
                    doc.add_shape(layer, shape.clone());
                }
            });
        });
    }

    fn request_display(&mut self, cx: &mut Context<Self>) {
        let Some(base) = self.base.clone() else {
            return;
        };
        let request = self.display_request.fetch_add(1, Ordering::SeqCst) + 1;
        let generation = self.display_request.clone();
        let filters = self.filters;
        self._derive = Some(cx.spawn(async move |this, cx| {
            let derived = cx
                .background_spawn(async move {
                    apply_filters_until(&base, &filters, &mut || {
                        generation.load(Ordering::SeqCst) != request
                    })
                })
                .await;
            this.update(cx, |doc, cx| {
                if doc.display_request.load(Ordering::SeqCst) != request {
                    return;
                }
                if let Some(derived) = derived {
                    doc.display = Some(rgba_to_render(derived));
                    cx.emit(ImageEditorEvent::DisplayChanged);
                    cx.notify();
                }
            })
            .ok();
        }));
    }
}

fn rotate_point(p: Point<Pixels>, old_height: f32) -> Point<Pixels> {
    point(px(old_height - p.y.as_f32()), p.x)
}

/// The crop rect clipped to the image, as integer pixel bounds, when the
/// intersection still clears MIN_CROP_PX on both axes.
fn clipped_crop((bw, bh): (u32, u32), rect: Bounds<Pixels>) -> Option<(u32, u32, u32, u32)> {
    let image_bounds = Bounds {
        origin: Point::default(),
        size: size(px(bw as f32), px(bh as f32)),
    };
    let clipped = rect.intersect(&image_bounds);
    let x0 = clipped.origin.x.as_f32().floor().max(0.) as u32;
    let y0 = clipped.origin.y.as_f32().floor().max(0.) as u32;
    let x1 = clipped.bottom_right().x.as_f32().ceil().min(bw as f32) as u32;
    let y1 = clipped.bottom_right().y.as_f32().ceil().min(bh as f32) as u32;
    let (cw, ch) = (x1.saturating_sub(x0), y1.saturating_sub(y0));
    let min = MIN_CROP_PX.as_f32() as u32;
    (cw >= min && ch >= min).then_some((x0, y0, cw, ch))
}

fn clamp_point(p: Point<Pixels>, bounds: Bounds<Pixels>) -> Point<Pixels> {
    let max_x = bounds.origin.x + bounds.size.width;
    let max_y = bounds.origin.y + bounds.size.height;
    point(
        p.x.max(bounds.origin.x).min(max_x),
        p.y.max(bounds.origin.y).min(max_y),
    )
}

/// Keeps crop survivors inside the cropped image so the canvas preview and
/// the rasterized export render the same pixels.
fn clamp_shape(shape: &CanvasShape, bounds: Bounds<Pixels>) -> CanvasShape {
    match shape {
        CanvasShape::Path { points, style } => CanvasShape::Path {
            points: points.iter().map(|p| clamp_point(*p, bounds)).collect(),
            style: style.clone(),
        },
        CanvasShape::Rect { bounds: b, style } => CanvasShape::Rect {
            bounds: b.intersect(&bounds),
            style: style.clone(),
        },
        CanvasShape::Ellipse { bounds: b, style } => CanvasShape::Ellipse {
            bounds: b.intersect(&bounds),
            style: style.clone(),
        },
        CanvasShape::Triangle { bounds: b, style } => CanvasShape::Triangle {
            bounds: b.intersect(&bounds),
            style: style.clone(),
        },
        CanvasShape::Text { .. } | CanvasShape::Image { .. } => shape.clone(),
    }
}

fn rotate_shape(shape: &CanvasShape, old_height: f32) -> CanvasShape {
    match shape {
        CanvasShape::Path { points, style } => CanvasShape::Path {
            points: points
                .iter()
                .map(|p| rotate_point(*p, old_height))
                .collect(),
            style: style.clone(),
        },
        CanvasShape::Rect { bounds, style } => CanvasShape::Rect {
            bounds: rotate_bounds(*bounds, old_height),
            style: style.clone(),
        },
        CanvasShape::Ellipse { bounds, style } => CanvasShape::Ellipse {
            bounds: rotate_bounds(*bounds, old_height),
            style: style.clone(),
        },
        CanvasShape::Triangle { bounds, style } => CanvasShape::Triangle {
            bounds: rotate_bounds(*bounds, old_height),
            style: style.clone(),
        },
        CanvasShape::Text { .. } | CanvasShape::Image { .. } => shape.clone(),
    }
}

fn rotate_bounds(bounds: Bounds<Pixels>, old_height: f32) -> Bounds<Pixels> {
    super::tools::bounds_from_corners(
        rotate_point(bounds.origin, old_height),
        rotate_point(bounds.bottom_right(), old_height),
    )
}

#[cfg(test)]
#[path = "document.test.rs"]
mod document_test;
