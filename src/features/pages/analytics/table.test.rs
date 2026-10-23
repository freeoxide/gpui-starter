use gpui_kit::component::Size;
use gpui_kit::component::table::{ColumnSort, TableDelegate, TableState};
use gpui_kit::{Hsla, TestAppContext, TextRun, WindowHandle, px, size};

use super::super::model::{ChannelRow, Segment, WorkspaceData};
use super::ChannelTableDelegate;

type Handle = WindowHandle<TableState<ChannelTableDelegate>>;

fn table_for(cx: &mut TestAppContext, rows: Vec<ChannelRow>) -> Handle {
    cx.update(gpui_kit::component::theme::init);
    cx.open_window(size(px(800.), px(600.)), |window, cx| {
        TableState::new(ChannelTableDelegate::new(rows), window, cx)
    })
}

fn load_table(cx: &mut TestAppContext) -> Handle {
    table_for(cx, WorkspaceData::load().channels(Segment::All))
}

#[gpui_kit::test]
async fn delegate_exposes_seven_columns_and_all_48_rows(cx: &mut TestAppContext) {
    let table = load_table(cx);
    let entity = table.root(cx).unwrap();
    cx.read(|cx| {
        let delegate = entity.read(cx).delegate();
        assert_eq!(delegate.columns_count(cx), 7);
        assert_eq!(delegate.rows_count(cx), 48);
    });
}

#[gpui_kit::test]
async fn revenue_sorts_descending_by_default_and_ascending_on_demand(cx: &mut TestAppContext) {
    let table = load_table(cx);
    let entity = table.root(cx).unwrap();
    cx.read(|cx| {
        let delegate = entity.read(cx).delegate();
        let revenues: Vec<f64> = (0..delegate.rows_count(cx))
            .map(|ix| delegate.row(ix).unwrap().revenue)
            .collect();
        assert!(revenues.windows(2).all(|pair| pair[0] >= pair[1]));
    });
    table
        .update(cx, |state, window, cx| {
            TableDelegate::perform_sort(state.delegate_mut(), 4, ColumnSort::Ascending, window, cx);
        })
        .unwrap();
    cx.read(|cx| {
        let delegate = entity.read(cx).delegate();
        let revenues: Vec<f64> = (0..delegate.rows_count(cx))
            .map(|ix| delegate.row(ix).unwrap().revenue)
            .collect();
        assert!(revenues.windows(2).all(|pair| pair[0] <= pair[1]));
    });
}

#[gpui_kit::test]
async fn page_sort_ascending_orders_page_strings(cx: &mut TestAppContext) {
    let table = load_table(cx);
    let entity = table.root(cx).unwrap();
    table
        .update(cx, |state, window, cx| {
            TableDelegate::perform_sort(state.delegate_mut(), 0, ColumnSort::Ascending, window, cx);
        })
        .unwrap();
    cx.read(|cx| {
        let delegate = entity.read(cx).delegate();
        let last = delegate.rows_count(cx) - 1;
        let first = delegate.row(0).unwrap().page.to_string();
        let last_page = delegate.row(last).unwrap().page.to_string();
        assert!(first <= last_page);
    });
}

#[gpui_kit::test]
async fn social_segment_keeps_twelve_rows(cx: &mut TestAppContext) {
    let table = table_for(cx, WorkspaceData::load().channels(Segment::Social));
    let entity = table.root(cx).unwrap();
    cx.read(|cx| {
        let delegate = entity.read(cx).delegate();
        assert_eq!(delegate.rows_count(cx), 12);
        assert_eq!(delegate.row(0).unwrap().channel, "social");
    });
}

#[gpui_kit::test]
async fn key_at_round_trips_into_channel_series(cx: &mut TestAppContext) {
    let data = WorkspaceData::load();
    let table = load_table(cx);
    let entity = table.root(cx).unwrap();
    let key = cx.read(|cx| {
        let delegate = entity.read(cx).delegate();
        delegate.key_at(0).expect("row 0 has a key")
    });
    assert_eq!(data.channel_series(&key.channel).len(), 90);
    cx.read(|cx| {
        let delegate = entity.read(cx).delegate();
        assert_eq!(delegate.row_ix_for_key(&key), Some(0));
    });
}

#[gpui_kit::test]
async fn set_rows_preserves_the_active_sort(cx: &mut TestAppContext) {
    let data = WorkspaceData::load();
    let table = load_table(cx);
    let entity = table.root(cx).unwrap();
    table
        .update(cx, |state, window, cx| {
            TableDelegate::perform_sort(state.delegate_mut(), 3, ColumnSort::Ascending, window, cx);
        })
        .unwrap();
    table
        .update(cx, |state, _, cx| {
            state.delegate_mut().set_rows(data.channels(Segment::All));
            cx.notify();
        })
        .unwrap();
    cx.read(|cx| {
        let delegate = entity.read(cx).delegate();
        let last = delegate.rows_count(cx) - 1;
        let sessions = |ix: usize| delegate.row(ix).unwrap().sessions;
        assert!(sessions(0) <= sessions(last));
    });
}

#[gpui_kit::test]
async fn set_loading_drives_the_loading_flag(cx: &mut TestAppContext) {
    let table = load_table(cx);
    let entity = table.root(cx).unwrap();
    table
        .update(cx, |state, _, cx| {
            state.delegate_mut().set_loading(true);
            cx.notify();
        })
        .unwrap();
    cx.read(|cx| {
        let delegate = entity.read(cx).delegate();
        assert!(delegate.loading(cx));
    });
}

#[gpui_kit::test]
async fn fitted_page_column_holds_the_longest_cell_text(cx: &mut TestAppContext) {
    let table = load_table(cx);
    let entity = table.root(cx).unwrap();
    let longest_page = cx.read(|cx| {
        let delegate = entity.read(cx).delegate();
        (0..delegate.rows_count(cx))
            .map(|ix| delegate.row(ix).unwrap().page.clone())
            .max_by_key(|page| page.len())
            .expect("fixture has rows")
    });

    table
        .update(cx, |state, window, cx| {
            assert!(state.fit_col(0, window, cx));
            let width = state.col_width(0).expect("column 0 exists");
            let rem = window.rem_size();
            let font = window.text_style().font();
            let text = window
                .text_system()
                .shape_line(
                    longest_page.clone(),
                    rem * 0.875,
                    &[TextRun {
                        len: longest_page.len(),
                        font,
                        color: Hsla::default(),
                        background_color: None,
                        underline: None,
                        strikethrough: None,
                    }],
                    None,
                )
                .width;
            let padding = Size::default().table_cell_padding();
            let content_box = width - padding.left - padding.right;
            assert!(
                content_box >= text,
                "fitted content box {content_box} must hold {text} of page text"
            );
        })
        .unwrap();
}
