use std::sync::Arc;

use gpui_kit::*;

use super::geometry::segment_distance;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct LayerId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct ShapeId(pub u64);

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct StrokeStyle {
    /// Document-space stroke width; scales with zoom.
    pub width: Pixels,
    pub color: Hsla,
    pub dash: Option<Vec<Pixels>>,
}

impl Default for StrokeStyle {
    fn default() -> Self {
        Self {
            width: px(1.),
            color: black(),
            dash: None,
        }
    }
}

/// Fill and/or stroke. Private fields; use the constructors.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct ShapeStyle {
    fill: Option<Hsla>,
    stroke: Option<StrokeStyle>,
}

impl ShapeStyle {
    pub fn filled(fill: Hsla) -> Self {
        Self {
            fill: Some(fill),
            stroke: None,
        }
    }

    pub fn stroked(stroke: StrokeStyle) -> Self {
        Self {
            fill: None,
            stroke: Some(stroke),
        }
    }

    pub fn both(fill: Hsla, stroke: StrokeStyle) -> Self {
        Self {
            fill: Some(fill),
            stroke: Some(stroke),
        }
    }

    pub fn fill(&self) -> Option<Hsla> {
        self.fill
    }

    pub fn stroke(&self) -> Option<&StrokeStyle> {
        self.stroke.as_ref()
    }
}

#[derive(Clone, Debug)]
#[non_exhaustive]
pub enum CanvasShape {
    Rect {
        bounds: Bounds<Pixels>,
        style: ShapeStyle,
    },
    Ellipse {
        bounds: Bounds<Pixels>,
        style: ShapeStyle,
    },
    /// Isosceles triangle inside `bounds`: one apex at top-center, two
    /// corners at the bottom-left/bottom-right.
    Triangle {
        bounds: Bounds<Pixels>,
        style: ShapeStyle,
    },
    Path {
        points: Vec<Point<Pixels>>,
        style: StrokeStyle,
    },
    Text {
        position: Point<Pixels>,
        text: SharedString,
        font_size: Pixels,
        color: Hsla,
        /// Doc-space size measured by [`measure_text`] at creation; bounds
        /// and hit tests derive from it.
        measured: Size<Pixels>,
    },
    Image {
        bounds: Bounds<Pixels>,
        image: Arc<RenderImage>,
    },
}

/// Measures `text` in the default-font run text shapes paint with, so bounds match
/// the glyphs. Any `WindowTextSystem` over the app's `TextSystem` measures alike.
pub fn measure_text(text_system: &WindowTextSystem, text: &str, font_size: Pixels) -> Size<Pixels> {
    let run = TextRun {
        len: text.len(),
        font: TextStyle::default().font(),
        color: black(),
        background_color: None,
        underline: None,
        strikethrough: None,
    };
    let shaped = text_system.shape_line(text.into(), font_size, &[run], None);
    size(shaped.width, shaped.ascent + shaped.descent)
}

/// Wraps RGBA pixels for [`CanvasShape::Image`]. RenderImage data is BGRA,
/// so the swizzle happens here and nowhere else.
pub fn rgba_to_render(img: image::RgbaImage) -> Arc<RenderImage> {
    let (width, height) = img.dimensions();
    let bgra = img.into_raw_bgra();
    // into_raw_bgra swaps channels in place, so the container length is a
    // from_raw invariant that cannot fail here.
    let buffer = image::RgbaImage::from_raw(width, height, bgra)
        .expect("into_raw_bgra preserves the container length");
    Arc::new(RenderImage::new(vec![image::Frame::new(buffer)]))
}

fn expanded(bounds: &Bounds<Pixels>, by: Pixels) -> Bounds<Pixels> {
    Bounds {
        origin: bounds.origin - point(by, by),
        size: size(bounds.size.width + by + by, bounds.size.height + by + by),
    }
}

/// Document-space corners of a triangle: apex, bottom-left, bottom-right.
fn triangle_vertices(bounds: &Bounds<Pixels>) -> [Point<Pixels>; 3] {
    let left = bounds.origin.x;
    let right = left + bounds.size.width;
    let top = bounds.origin.y;
    let bottom = top + bounds.size.height;
    [
        point(left + bounds.size.width / 2., top),
        point(left, bottom),
        point(right, bottom),
    ]
}

impl CanvasShape {
    pub fn rect(bounds: Bounds<Pixels>, style: ShapeStyle) -> Self {
        Self::Rect { bounds, style }
    }

    pub fn ellipse(bounds: Bounds<Pixels>, style: ShapeStyle) -> Self {
        Self::Ellipse { bounds, style }
    }

    pub fn triangle(bounds: Bounds<Pixels>, style: ShapeStyle) -> Self {
        Self::Triangle { bounds, style }
    }

    pub fn path(points: Vec<Point<Pixels>>, style: StrokeStyle) -> Self {
        Self::Path { points, style }
    }

    /// `measured` is the doc-space size from [`measure_text`] with this text
    /// and font size; doc_bounds and hit_test read it.
    pub fn text(
        position: Point<Pixels>,
        text: impl Into<SharedString>,
        font_size: Pixels,
        color: Hsla,
        measured: Size<Pixels>,
    ) -> Self {
        Self::Text {
            position,
            text: text.into(),
            font_size,
            color,
            measured,
        }
    }

    pub fn image(bounds: Bounds<Pixels>, image: Arc<RenderImage>) -> Self {
        Self::Image { bounds, image }
    }

    /// Conservative document-space bounds, including half the stroke width on
    /// stroked shapes; text uses the size measured at creation.
    pub fn doc_bounds(&self) -> Bounds<Pixels> {
        match self {
            Self::Rect { bounds, style }
            | Self::Ellipse { bounds, style }
            | Self::Triangle { bounds, style } => {
                let pad = style
                    .stroke()
                    .map(|stroke| stroke.width * 0.5)
                    .unwrap_or(px(0.));
                expanded(bounds, pad)
            }
            Self::Image { bounds, .. } => *bounds,
            Self::Path { points, style } => {
                let half_width = style.width * 0.5;
                let mut min = points.first().copied().unwrap_or_default();
                let mut max = min;
                for p in points.iter().skip(1) {
                    min = point(min.x.min(p.x), min.y.min(p.y));
                    max = point(max.x.max(p.x), max.y.max(p.y));
                }
                Bounds::from_corners(
                    min - point(half_width, half_width),
                    max + point(half_width, half_width),
                )
            }
            Self::Text {
                position, measured, ..
            } => Bounds {
                origin: *position,
                size: *measured,
            },
        }
    }

    pub fn translate(&mut self, delta: Point<Pixels>) {
        match self {
            Self::Rect { bounds, .. }
            | Self::Ellipse { bounds, .. }
            | Self::Triangle { bounds, .. }
            | Self::Image { bounds, .. } => bounds.origin += delta,
            Self::Path { points, .. } => {
                for p in points {
                    *p += delta;
                }
            }
            Self::Text { position, .. } => *position += delta,
        }
    }

    pub fn hit_test(&self, doc_point: Point<Pixels>, tolerance: Pixels) -> bool {
        match self {
            Self::Rect { bounds, .. } | Self::Image { bounds, .. } => {
                expanded(bounds, tolerance).contains(&doc_point)
            }
            Self::Ellipse { bounds, .. } => {
                let center = bounds.center();
                let rx = bounds.size.width.as_f32() / 2. + tolerance.as_f32();
                let ry = bounds.size.height.as_f32() / 2. + tolerance.as_f32();
                let dx = (doc_point.x - center.x).as_f32() / rx;
                let dy = (doc_point.y - center.y).as_f32() / ry;
                dx * dx + dy * dy <= 1.
            }
            Self::Triangle { bounds, .. } => {
                if !expanded(bounds, tolerance).contains(&doc_point) {
                    return false;
                }
                let [a, b, c] = triangle_vertices(bounds);
                let cross = |o: Point<Pixels>, m: Point<Pixels>, n: Point<Pixels>| {
                    (m.x - o.x).as_f32() * (n.y - o.y).as_f32()
                        - (m.y - o.y).as_f32() * (n.x - o.x).as_f32()
                };
                let d1 = cross(doc_point, a, b);
                let d2 = cross(doc_point, b, c);
                let d3 = cross(doc_point, c, a);
                let has_neg = d1 < 0. || d2 < 0. || d3 < 0.;
                let has_pos = d1 > 0. || d2 > 0. || d3 > 0.;
                !(has_neg && has_pos)
                    || segment_distance(doc_point, a, b) <= tolerance
                    || segment_distance(doc_point, b, c) <= tolerance
                    || segment_distance(doc_point, c, a) <= tolerance
            }
            Self::Path { points, style } => {
                if !expanded(&self.doc_bounds(), tolerance).contains(&doc_point) {
                    return false;
                }
                if points.len() == 1 {
                    let radius = style.width.as_f32() / 2. + tolerance.as_f32();
                    let delta = doc_point - points[0];
                    let dx = delta.x.as_f32() / radius;
                    let dy = delta.y.as_f32() / radius;
                    dx * dx + dy * dy <= 1.
                } else {
                    // The hit corridor must cover the painted stroke, not
                    // just its centerline, like the single-point branch above.
                    let corridor = tolerance + style.width * 0.5;
                    points
                        .windows(2)
                        .any(|w| segment_distance(doc_point, w[0], w[1]) <= corridor)
                }
            }
            Self::Text {
                position, measured, ..
            } => expanded(
                &Bounds {
                    origin: *position,
                    size: *measured,
                },
                tolerance,
            )
            .contains(&doc_point),
        }
    }
}

/// A paint order: layers[0] paints first (bottom). Private fields.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct CanvasLayer {
    id: LayerId,
    visible: bool,
    shapes: Vec<(ShapeId, CanvasShape)>,
}

impl CanvasLayer {
    pub fn id(&self) -> LayerId {
        self.id
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn shapes(&self) -> &[(ShapeId, CanvasShape)] {
        &self.shapes
    }
}

#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct CanvasDocument {
    layers: Vec<CanvasLayer>,
    next_id: u64,
}

impl CanvasDocument {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn layers(&self) -> &[CanvasLayer] {
        &self.layers
    }

    pub fn layer(&self, id: LayerId) -> Option<&CanvasLayer> {
        self.layers.iter().find(|layer| layer.id == id)
    }

    pub fn top_layer(&self) -> Option<LayerId> {
        self.layers.last().map(|layer| layer.id)
    }

    pub fn shape(&self, id: ShapeId) -> Option<&CanvasShape> {
        self.layers
            .iter()
            .find_map(|layer| layer.shapes.iter().find(|(sid, _)| *sid == id))
            .map(|(_, shape)| shape)
    }

    /// Bottom-to-top iterator across visible and hidden layers.
    pub fn shapes(&self) -> impl Iterator<Item = (LayerId, ShapeId, &CanvasShape)> + '_ {
        self.layers.iter().flat_map(|layer| {
            layer
                .shapes
                .iter()
                .map(move |(shape_id, shape)| (layer.id, *shape_id, shape))
        })
    }

    pub fn add_layer(&mut self) -> LayerId {
        let id = LayerId(self.alloc_id());
        self.layers.push(CanvasLayer {
            id,
            visible: true,
            shapes: Vec::new(),
        });
        id
    }

    pub fn add_shape(&mut self, layer: LayerId, shape: CanvasShape) -> Option<ShapeId> {
        let id = ShapeId(self.alloc_id());
        self.layers
            .iter_mut()
            .find(|l| l.id == layer)?
            .shapes
            .push((id, shape));
        Some(id)
    }

    pub fn remove_shape(&mut self, id: ShapeId) -> Option<CanvasShape> {
        for layer in &mut self.layers {
            if let Some(ix) = layer.shapes.iter().position(|(sid, _)| *sid == id) {
                return Some(layer.shapes.remove(ix).1);
            }
        }
        None
    }

    pub fn set_layer_visible(&mut self, id: LayerId, visible: bool) {
        if let Some(layer) = self.layers.iter_mut().find(|layer| layer.id == id) {
            layer.visible = visible;
        }
    }

    pub fn update_shape<R>(
        &mut self,
        id: ShapeId,
        f: impl FnOnce(&mut CanvasShape) -> R,
    ) -> Option<R> {
        let shape = self
            .layers
            .iter_mut()
            .find_map(|layer| layer.shapes.iter_mut().find(|(sid, _)| *sid == id))
            .map(|(_, shape)| shape);
        shape.map(f)
    }

    pub fn clear(&mut self) {
        self.layers.clear();
    }

    fn alloc_id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }
}

#[cfg(test)]
#[path = "scene.test.rs"]
mod scene_test;
