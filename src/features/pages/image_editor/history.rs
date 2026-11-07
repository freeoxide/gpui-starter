use std::sync::Arc;

use image::RgbaImage;

use super::document::FilterSettings;
use crate::ui::canvas::{CanvasShape, ShapeId};

pub(crate) type AnnotationSnapshot = Vec<(ShapeId, CanvasShape)>;

#[derive(Clone)]
pub(crate) struct DocSnapshot {
    pub(crate) base: Arc<RgbaImage>,
    pub(crate) filters: FilterSettings,
    pub(crate) annotations: AnnotationSnapshot,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum EditLabel {
    Stroke,
    Erase,
    Rotate,
    Crop,
    Filters,
}

#[derive(Clone)]
pub(crate) struct EditOp {
    pub(crate) label: EditLabel,
    pub(crate) before: DocSnapshot,
    pub(crate) after: DocSnapshot,
}

pub(crate) struct EditorHistory {
    past: Vec<EditOp>,
    future: Vec<EditOp>,
}

pub(crate) const HISTORY_ENTRY_LIMIT: usize = 40;
pub(crate) const HISTORY_PIXEL_BUDGET: usize = 256 * 1024 * 1024;

impl EditorHistory {
    pub(crate) fn new() -> Self {
        Self {
            past: Vec::new(),
            future: Vec::new(),
        }
    }

    pub(crate) fn record(&mut self, op: EditOp) {
        self.record_with_budget(op, HISTORY_ENTRY_LIMIT, HISTORY_PIXEL_BUDGET);
    }

    pub(crate) fn record_with_budget(
        &mut self,
        op: EditOp,
        entry_limit: usize,
        pixel_budget: usize,
    ) {
        self.future.clear();
        self.past.push(op);
        // Never evict the just-recorded op: an oversized base must not kill
        // undo for the whole session.
        while self.past.len() > 1
            && (self.past.len() > entry_limit || distinct_base_bytes(&self.past) > pixel_budget)
        {
            self.past.remove(0);
        }
    }

    pub(crate) fn undo(&mut self) -> Option<&EditOp> {
        let op = self.past.pop()?;
        self.future.push(op);
        self.future.last()
    }

    pub(crate) fn redo(&mut self) -> Option<&EditOp> {
        let op = self.future.pop()?;
        self.past.push(op);
        self.past.last()
    }

    pub(crate) fn clear(&mut self) {
        self.past.clear();
        self.future.clear();
    }

    pub(crate) fn can_undo(&self) -> bool {
        !self.past.is_empty()
    }

    pub(crate) fn can_redo(&self) -> bool {
        !self.future.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.past.len()
    }
}

fn distinct_base_bytes(entries: &[EditOp]) -> usize {
    // Identity is content, not address: an evicted entry's allocation can be
    // reused by a later image, and re-encoded copies must still dedup.
    let mut seen: Vec<&Arc<RgbaImage>> = Vec::new();
    let mut total = 0usize;
    for op in entries {
        for base in [&op.before.base, &op.after.base] {
            if seen
                .iter()
                .any(|seen| Arc::ptr_eq(seen, base) || ***seen == **base)
            {
                continue;
            }
            seen.push(base);
            let (w, h) = base.dimensions();
            total += w as usize * h as usize * 4;
        }
    }
    total
}

#[cfg(test)]
#[path = "history.test.rs"]
mod history_test;
