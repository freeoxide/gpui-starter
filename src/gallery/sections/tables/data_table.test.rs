// No glob `use gpui_kit::*` here: it would import gpui's `test` proc-macro and
// shadow `#[gpui_kit::test]`. Import only what is needed.
use gpui_kit::component::table::TableSelection;
use gpui_kit::{Modifiers, TestAppContext, VisualTestContext, point, px, size};

use super::{DataTableSection, TableGoTo};

/// Columns `StockTableDelegate::new` pins to the left; the table's visible
/// column range counts only the columns that scroll.
const PINNED_LEFT_COLUMNS: usize = 4;

/// `TableState::set_selected_cell` stopped scrolling in kit 0.11, so the
/// Go-to actions scroll on their own; without that they select a cell the
/// user cannot see. Row 10 is inside the stock table's 30rem viewport from
/// the start, so the column scroll is the observable half here.
#[gpui_kit::test]
fn go_to_cell_scrolls_the_table_to_the_selected_cell(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let volume_column = 7 - PINNED_LEFT_COLUMNS;
    // Narrow enough that column 7 starts off-screen.
    let handle = cx.open_window(size(px(480.), px(420.)), |window, cx| {
        DataTableSection::new(window, cx)
    });
    let section = handle.root(cx).unwrap();
    let mut cx = VisualTestContext::from_window(handle.into(), cx);
    let selection =
        |cx: &mut VisualTestContext| cx.update(|_, cx| section.read(cx).table.read(cx).selection());
    let visible_cols = |cx: &mut VisualTestContext| {
        cx.update(|window, cx| {
            window.draw(cx).clear(cx);
            section
                .read(cx)
                .table
                .read(cx)
                .visible_range()
                .cols()
                .clone()
        })
    };

    // The action bubbles from the focused element, so click into the table.
    cx.simulate_click(point(px(240.), px(300.)), Modifiers::none());
    assert!(
        matches!(selection(&mut cx), TableSelection::Row(_)),
        "the focusing click must land on a stock table row; selected {:?}",
        selection(&mut cx)
    );
    let before = visible_cols(&mut cx);
    assert!(
        !before.contains(&volume_column),
        "column 7 must start off-screen; visible scrollable cols {before:?}"
    );

    cx.dispatch_action(TableGoTo::Cell107);

    let cols = visible_cols(&mut cx);
    assert_eq!(
        selection(&mut cx),
        TableSelection::Cell(10, 7),
        "Go To Cell 10:7 must select the cell"
    );
    assert!(
        cols.contains(&volume_column),
        "Go To Cell 10:7 must reveal its column; visible scrollable cols {cols:?}"
    );
}
