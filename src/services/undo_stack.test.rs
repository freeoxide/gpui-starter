use super::*;
use gpui_kit::{TestAppContext, component::ActiveTheme as _};

fn sample_entry() -> UndoEntry {
    UndoEntry {
        label: "Switch Theme".to_string(),
        undo_label: "Undo Theme Switch".to_string(),
        redo_label: "Redo Theme Switch".to_string(),
        created_at: AppTimestamp::now(),
        kind: UndoKind::ThemeMode {
            before: ThemeMode::Light,
            after: ThemeMode::Dark,
        },
    }
}

#[test]
fn record_clears_redo_history() {
    let mut model = UndoModel {
        future: vec![sample_entry()],
        ..UndoModel::default()
    };
    model.record(sample_entry());
    assert_eq!(model.past.len(), 1);
    assert!(model.future.is_empty());
}

/// `record` during apply is the reentrancy hazard the `applying` guard exists
/// for: an inverse action must reach neither stack, and nothing may be reset.
#[test]
fn record_while_applying_is_ignored() {
    let mut model = UndoModel {
        past: vec![sample_entry()],
        future: vec![sample_entry()],
        applying: true,
        last_rejected: Some("nothing to redo".to_string()),
    };
    model.record(sample_entry());
    assert_eq!(
        model.past.len(),
        1,
        "a record during apply must not grow the undo stack"
    );
    assert_eq!(
        model.future.len(),
        1,
        "a record during apply must not clear the redo stack"
    );
    assert_eq!(model.last_rejected.as_deref(), Some("nothing to redo"));
}

#[test]
fn pop_undo_sets_rejected_reason_when_empty() {
    let mut model = UndoModel::default();
    assert!(model.pop_undo().is_none());
    assert_eq!(model.last_rejected.as_deref(), Some("nothing to undo"));
}

#[test]
fn pop_redo_sets_rejected_reason_when_empty() {
    let mut model = UndoModel::default();
    assert!(model.pop_redo().is_none());
    assert_eq!(model.last_rejected.as_deref(), Some("nothing to redo"));
}

#[test]
fn undo_redo_round_trip_restores_the_entry() {
    let mut model = UndoModel {
        past: vec![sample_entry()],
        last_rejected: Some("nothing to redo".to_string()),
        ..UndoModel::default()
    };

    let undone = model.pop_undo().expect("seeded entry on the undo stack");
    model.push_redo(undone);
    assert!(model.past.is_empty());
    assert_eq!(
        model.future.len(),
        1,
        "undo must park the entry on the redo stack"
    );

    let redone = model.pop_redo().expect("entry parked on the redo stack");
    model.push_undo(redone);
    assert_eq!(
        model.past.len(),
        1,
        "redo must return the entry to the undo stack"
    );
    assert!(model.future.is_empty());
    assert_eq!(model.past[0].label, "Switch Theme");
    assert!(
        model.last_rejected.is_none(),
        "a completed round trip clears the rejection"
    );
}

// undo()/redo() re-enter set_theme_mode, which reads the global theme kit init installs.
#[gpui_kit::test]
async fn undo_and_redo_replay_a_recorded_theme_change(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(|cx| crate::app::set_theme_mode(ThemeMode::Dark, cx));
    assert_eq!(cx.read(|cx| cx.theme().mode), ThemeMode::Dark);
    let recorded = cx.read(|cx| snapshot(cx));
    assert_eq!(recorded.past.len(), 1);
    assert!(recorded.future.is_empty());

    assert!(cx.update(undo));
    assert_eq!(cx.read(|cx| cx.theme().mode), ThemeMode::Light);
    assert_eq!(
        cx.read(|cx| can_redo(cx)).as_deref(),
        Some("Redo Theme Switch")
    );
    let undone = cx.read(|cx| snapshot(cx));
    assert!(undone.past.is_empty());
    assert_eq!(undone.future.len(), 1);
    assert!(
        !undone.applying,
        "undo must clear the applying flag after apply"
    );

    assert!(cx.update(redo));
    assert_eq!(cx.read(|cx| cx.theme().mode), ThemeMode::Dark);
    assert_eq!(
        cx.read(|cx| can_undo(cx)).as_deref(),
        Some("Undo Theme Switch")
    );
    let redone = cx.read(|cx| snapshot(cx));
    assert_eq!(redone.past.len(), 1);
    assert!(redone.future.is_empty());
}
