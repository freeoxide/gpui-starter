use std::sync::Arc;

use gpui_kit::{Bounds, Pixels, point, px};
use image::RgbaImage;

use super::{
    DocSnapshot, EditLabel, EditOp, EditorHistory, HISTORY_ENTRY_LIMIT, HISTORY_PIXEL_BUDGET,
};
use crate::features::pages::image_editor::document::FilterSettings;
use crate::ui::canvas::ShapeId;
use crate::ui::canvas::{CanvasShape, ShapeStyle};

fn snapshot(base: Arc<RgbaImage>, annotations: usize) -> DocSnapshot {
    DocSnapshot {
        base,
        filters: FilterSettings::default(),
        annotations: (0..annotations)
            .map(|i| {
                (
                    ShapeId(i as u64),
                    CanvasShape::rect(
                        Bounds::<Pixels>::from_corners(
                            point(px(0.), px(0.)),
                            point(px(1.), px(1.)),
                        ),
                        ShapeStyle::default(),
                    ),
                )
            })
            .collect(),
    }
}

fn op(before: DocSnapshot, after: DocSnapshot) -> EditOp {
    EditOp {
        label: EditLabel::Stroke,
        before,
        after,
    }
}

fn solid(w: u32, h: u32) -> Arc<RgbaImage> {
    Arc::new(RgbaImage::from_pixel(w, h, image::Rgba([255, 0, 0, 255])))
}

#[test]
fn undo_redo_round_trip_restores_entries() {
    let mut history = EditorHistory::new();
    let a = snapshot(solid(2, 2), 1);
    let b = snapshot(solid(2, 2), 2);
    history.record(op(a, b.clone()));

    let undone = history.undo().cloned();
    assert_eq!(undone.as_ref().map(|e| e.after.annotations.len()), Some(2));
    assert!(history.can_redo());

    let redone = history.redo().cloned();
    assert_eq!(redone.as_ref().map(|e| e.after.annotations.len()), Some(2));
    assert!(!history.can_redo());
    assert!(history.can_undo());
}

#[test]
fn recording_after_undo_clears_the_future() {
    let mut history = EditorHistory::new();
    history.record(op(snapshot(solid(2, 2), 1), snapshot(solid(2, 2), 2)));
    history.undo();
    assert!(history.can_redo());

    history.record(op(snapshot(solid(2, 2), 3), snapshot(solid(2, 2), 4)));
    assert!(!history.can_redo());
    assert_eq!(history.len(), 1);
}

#[test]
fn entry_limit_drops_the_oldest_entry() {
    let mut history = EditorHistory::new();
    for i in 0..5 {
        history.record_with_budget(
            op(snapshot(solid(1, 1), i), snapshot(solid(1, 1), i + 1)),
            3,
            HISTORY_PIXEL_BUDGET,
        );
    }
    assert_eq!(history.len(), 3);
}

#[test]
fn pixel_budget_drops_entries_until_within_budget() {
    let mut history = EditorHistory::new();
    for i in 0..4 {
        history.record_with_budget(
            op(snapshot(solid(8, 8), i), snapshot(solid(8, 8), i + 1)),
            HISTORY_ENTRY_LIMIT,
            8 * 8 * 4 * 2,
        );
    }
    assert!(history.len() < 4, "budget should have pruned entries");
    assert!(!history.past.is_empty());
}

#[test]
fn shared_arc_bases_count_once() {
    let shared = solid(8, 8);
    let mut history = EditorHistory::new();
    history.record_with_budget(
        op(snapshot(shared.clone(), 1), snapshot(shared.clone(), 2)),
        HISTORY_ENTRY_LIMIT,
        8 * 8 * 4,
    );
    assert_eq!(history.len(), 1, "one distinct buffer stays within budget");
}

#[test]
fn clear_empties_both_directions() {
    let mut history = EditorHistory::new();
    history.record(op(snapshot(solid(1, 1), 0), snapshot(solid(1, 1), 1)));
    history.undo();
    history.clear();
    assert!(!history.can_undo());
    assert!(!history.can_redo());
    assert_eq!(history.len(), 0);
}
