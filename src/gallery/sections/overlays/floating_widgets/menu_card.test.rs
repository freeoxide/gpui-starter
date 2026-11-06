use gpui_kit::test::TestWindowExt as _;
use gpui_kit::{App, TestAppContext, VisualTestContext, px, size};

use crate::ui::floating::{FloatAnchor, floating_snapshot, init};

use super::super::{ID_PREFIX, dictation::DICTATION_ID, sketch::SKETCH_ID};
use super::{
    MENU_ID, MenuCard, ROW_CLOSE, ROW_DICTATION, ROW_SKETCH, menu_rows, menu_spec, spawn_menu_card,
};

fn example_count(cx: &App) -> usize {
    floating_snapshot(cx)
        .iter()
        .filter(|entry| entry.id.0.starts_with(ID_PREFIX))
        .count()
}

#[gpui_kit::test]
fn menu_rows_carry_unique_ids() {
    let rows = menu_rows();
    assert_eq!(rows.len(), 3);
    for (i, row) in rows.iter().enumerate() {
        assert!(
            rows[i + 1..].iter().all(|other| other.id != row.id),
            "row ids must stay distinct for dispatch"
        );
        assert!(!row.label.is_empty());
    }
}

#[gpui_kit::test]
fn menu_spec_is_chromeless_screen_anchored_and_focusing() {
    let spec = menu_spec();
    assert!(spec.is_chromeless());
    assert!(spec.is_focus(), "keyboard selection needs window focus");
    assert!(matches!(spec.float_anchor(), FloatAnchor::Screen { .. }));
}

#[gpui_kit::test]
fn spawn_menu_card_opens_an_idempotent_screen_float_without_a_main_window(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    cx.update(init);
    let _ = cx.update(|cx| spawn_menu_card(cx));
    let _ = cx.update(|cx| spawn_menu_card(cx));
    let count = cx.update(|cx| {
        floating_snapshot(cx)
            .iter()
            .filter(|entry| entry.id.0 == MENU_ID)
            .count()
    });
    assert_eq!(count, 1, "a second spawn must reuse the open menu card");
}

#[gpui_kit::test]
async fn row_clicks_dispatch_their_actions_and_close_all_clears_the_examples(
    cx: &mut TestAppContext,
) {
    cx.update(gpui_kit::init);
    cx.update(init);
    let window = cx.open_window(size(px(240.), px(190.)), |_, _| MenuCard);
    let mut cx = VisualTestContext::from_window(window.into(), cx);
    cx.update(|window, cx| window.draw(cx).clear(cx));

    cx.update(|window, cx| window.click(ROW_DICTATION, cx));
    let dictation = cx.update(|_, cx| {
        floating_snapshot(cx)
            .iter()
            .any(|entry| entry.id.0 == DICTATION_ID)
    });
    assert!(dictation, "row one must spawn the dictation HUD");

    cx.update(|window, cx| window.click(ROW_SKETCH, cx));
    let sketch = cx.update(|_, cx| {
        floating_snapshot(cx)
            .iter()
            .any(|entry| entry.id.0 == SKETCH_ID)
    });
    assert!(sketch, "row two must spawn the sketch pad");

    cx.update(|window, cx| window.click(ROW_CLOSE, cx));
    let open = cx.update(|_, cx| example_count(cx));
    assert_eq!(open, 0, "close-all must close every spawned example float");
}
