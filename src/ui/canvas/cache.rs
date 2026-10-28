use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use gpui_kit::*;

use super::scene::{CanvasShape, ShapeStyle, StrokeStyle};

/// One slot per painted artifact: a matching paint generation reuses the
/// entry without rehashing; a changed one rebuilds only if content changed.
/// The shaped line stays boxed: ShapedLine carries a SmallVec<[DecorationRun;
/// 32]> inline (~3 KB) that would otherwise sit in every slot.
#[derive(Default)]
pub(crate) struct ShapeCache {
    generation: Option<u64>,
    content_key: Option<u64>,
    path: Option<Path<Pixels>>,
    line: Option<Box<ShapedLine>>,
}

impl ShapeCache {
    /// Returns true when the artifact must be rebuilt; records the
    /// generation either way so unmodified entries skip the key closure on
    /// every subsequent same-generation paint.
    fn invalidated(&mut self, generation: u64, content_key: impl FnOnce() -> u64) -> bool {
        if self.generation == Some(generation) {
            return false;
        }
        let key = content_key();
        let changed = self.content_key != Some(key);
        self.generation = Some(generation);
        if changed {
            self.content_key = Some(key);
        }
        changed
    }

    pub(crate) fn get_path(
        &mut self,
        generation: u64,
        content_key: impl FnOnce() -> u64,
        origin: Point<Pixels>,
        build: impl FnOnce() -> Option<Path<Pixels>>,
    ) -> Option<Path<Pixels>> {
        if self.invalidated(generation, content_key) {
            #[cfg(test)]
            super::perf_probe::add_path_build();
            self.path = None;
            self.line = None;
            self.path = build();
        }
        self.path.as_ref().map(|path| translated(path, origin))
    }

    pub(crate) fn get_line(
        &mut self,
        generation: u64,
        content_key: impl FnOnce() -> u64,
        build: impl FnOnce() -> ShapedLine,
    ) -> ShapedLine {
        if self.invalidated(generation, content_key) || self.line.is_none() {
            #[cfg(test)]
            super::perf_probe::add_text_shaping();
            self.path = None;
            self.line = None;
            self.line = Some(Box::new(build()));
        }
        self.line.as_deref().cloned().expect("line built above")
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

fn hash_hsla(hasher: &mut DefaultHasher, color: Hsla) {
    hash_f32(hasher, color.h);
    hash_f32(hasher, color.s);
    hash_f32(hasher, color.l);
    hash_f32(hasher, color.a);
}

pub(crate) fn shape_key(shape: &CanvasShape, zoom: f32) -> u64 {
    #[cfg(test)]
    super::perf_probe::add_key_hash();
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
            text,
            font_size,
            color,
        } => {
            hash_point(&mut hasher, position);
            hash_pixels(&mut hasher, *font_size);
            text.hash(&mut hasher);
            hash_hsla(&mut hasher, *color);
        }
        // Image paints uncached; nothing else contributes to its key.
        _ => {}
    }
    hasher.finish()
}

#[cfg(test)]
#[path = "cache.test.rs"]
mod cache_test;
