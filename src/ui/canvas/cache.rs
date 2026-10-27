use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use gpui_kit::*;

use super::scene::{CanvasShape, ShapeStyle, StrokeStyle};

#[derive(Default)]
pub(crate) struct ShapeCache {
    key: Option<u64>,
    path: Option<Path<Pixels>>,
}

impl ShapeCache {
    pub(crate) fn get(
        &mut self,
        key: u64,
        origin: Point<Pixels>,
        build: impl FnOnce() -> Option<Path<Pixels>>,
    ) -> Option<Path<Pixels>> {
        if self.key != Some(key) {
            self.path = build();
            self.key = Some(key);
        }
        self.path.as_ref().map(|path| translated(path, origin))
    }
}

fn translated(path: &Path<Pixels>, offset: Point<Pixels>) -> Path<Pixels> {
    let mut path = path.clone();
    path.bounds.origin += offset;
    for vertex in &mut path.vertices {
        vertex.xy_position += offset;
    }
    path
}

#[derive(Default)]
pub(crate) struct CanvasCaches {
    slots: Vec<ShapeCache>,
}

impl CanvasCaches {
    pub(crate) fn for_paint(
        key: impl Into<ElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<Self> {
        window.use_keyed_state(key, cx, |_, _| Self::default())
    }

    pub(crate) fn slot(&mut self, index: usize) -> &mut ShapeCache {
        if self.slots.len() <= index {
            self.slots.resize_with(index + 1, ShapeCache::default);
        }
        &mut self.slots[index]
    }
}

fn hash_f32(hasher: &mut DefaultHasher, value: f32) {
    value.to_bits().hash(hasher);
}

fn hash_pixels(hasher: &mut DefaultHasher, value: Pixels) {
    hash_f32(hasher, value.as_f32());
}

fn hash_point(hasher: &mut DefaultHasher, value: &Point<Pixels>) {
    hash_pixels(hasher, value.x);
    hash_pixels(hasher, value.y);
}

fn hash_bounds(hasher: &mut DefaultHasher, value: &Bounds<Pixels>) {
    hash_point(hasher, &value.origin);
    hash_pixels(hasher, value.size.width);
    hash_pixels(hasher, value.size.height);
}

fn hash_stroke(hasher: &mut DefaultHasher, stroke: &StrokeStyle) {
    hash_pixels(hasher, stroke.width);
    if let Some(dash) = &stroke.dash {
        for length in dash {
            hash_pixels(hasher, *length);
        }
    }
}

fn hash_style(hasher: &mut DefaultHasher, style: &ShapeStyle) {
    if let Some(stroke) = style.stroke() {
        hash_stroke(hasher, stroke);
    }
}

pub(crate) fn shape_key(shape: &CanvasShape, zoom: f32) -> u64 {
    let mut hasher = DefaultHasher::new();
    std::mem::discriminant(shape).hash(&mut hasher);
    hash_f32(&mut hasher, zoom);
    match shape {
        CanvasShape::Rect { bounds, style } | CanvasShape::Ellipse { bounds, style } => {
            hash_bounds(&mut hasher, bounds);
            hash_style(&mut hasher, style);
        }
        CanvasShape::Path { points, style } => {
            for point in points {
                hash_point(&mut hasher, point);
            }
            hash_stroke(&mut hasher, style);
        }
        CanvasShape::Text {
            position,
            font_size,
            ..
        } => {
            hash_point(&mut hasher, position);
            hash_pixels(&mut hasher, *font_size);
        }
        CanvasShape::Image { bounds, .. } => {
            hash_bounds(&mut hasher, bounds);
        }
    }
    hasher.finish()
}

#[cfg(test)]
#[path = "cache.test.rs"]
mod cache_test;
