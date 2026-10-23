use gpui_kit::component::{
    IndexPath, WindowExt as _,
    table::{TableDelegate as _, TableEvent},
};
use gpui_kit::{
    AnyWindowHandle, AppContext as _, Context, Entity, Render as _, TestAppContext, Window,
    WindowOptions, px,
};

use super::AnalyticsPage;
use super::model::{ChannelKey, Segment, WorkspaceData};

type Page = (AnyWindowHandle, Entity<AnalyticsPage>);

fn open_page(cx: &TestAppContext) -> Page {
    let opened = cx.update(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(WindowOptions::default(), cx, |window, cx| {
            cx.new(|cx| AnalyticsPage::new(window, cx))
        })
    });
    opened.expect("test window must open")
}

fn key_of_first(segment: Segment) -> ChannelKey {
    let row = WorkspaceData::load()
        .channels(segment)
        .into_iter()
        .next()
        .expect("segment has rows");
    ChannelKey::of(&row)
}

fn commit_first_reload(cx: &TestAppContext, page: &Entity<AnalyticsPage>) {
    cx.update(|cx| page.update(cx, |page, cx| page.commit_reload(cx)));
}

fn with_window(
    cx: &TestAppContext,
    handle: &AnyWindowHandle,
    page: &Entity<AnalyticsPage>,
    drive: impl FnOnce(&mut AnalyticsPage, &mut Window, &mut Context<AnalyticsPage>),
) {
    cx.update(|cx| {
        handle
            .update(cx, |_, window, cx| {
                page.update(cx, |page, page_cx| drive(page, window, page_cx));
            })
            .unwrap();
    });
}

#[gpui_kit::test]
async fn page_constructs_and_commits_the_first_reload(cx: &TestAppContext) {
    let (_, page) = open_page(cx);
    cx.read(|cx| assert_eq!(page.read(cx).phase, super::Phase::Initial));
    commit_first_reload(cx, &page);
    cx.read(|cx| {
        let page = page.read(cx);
        assert_eq!(page.phase, super::Phase::Ready);
        assert!(page.committed);
    });
}

#[gpui_kit::test]
async fn segment_commit_refreshes_then_social_reload_keeps_twelve_rows(cx: &TestAppContext) {
    let (handle, page) = open_page(cx);
    with_window(cx, &handle, &page, |page, window, cx| {
        page.segment_select.update(cx, |state, cx| {
            state.set_selected_index(Some(IndexPath::new(3)), window, cx);
        });
        page.on_segment_committed(cx);
    });
    cx.read(|cx| {
        let page = page.read(cx);
        assert_eq!(page.phase, super::Phase::Initial);
        assert_eq!(page.segment, Segment::Social);
    });
    commit_first_reload(cx, &page);
    cx.read(|cx| {
        let page = page.read(cx);
        assert_eq!(page.phase, super::Phase::Ready);
        assert_eq!(page.table.read(cx).delegate().rows_count(cx), 12);
    });
}

#[gpui_kit::test]
async fn committed_filter_change_renders_refreshing_phase(cx: &TestAppContext) {
    let (handle, page) = open_page(cx);
    commit_first_reload(cx, &page);
    with_window(cx, &handle, &page, |page, window, cx| {
        page.segment_select.update(cx, |state, cx| {
            state.set_selected_index(Some(IndexPath::new(2)), window, cx);
        });
        page.on_segment_committed(cx);
    });
    cx.read(|cx| assert_eq!(page.read(cx).phase, super::Phase::Refreshing));
}

#[gpui_kit::test]
async fn reload_restores_selection_by_domain_key_and_clears_missing_keys(cx: &TestAppContext) {
    let (handle, page) = open_page(cx);
    let organic_key = key_of_first(Segment::Organic);

    cx.update(|cx| {
        page.update(cx, |page, cx| {
            page.selected_key = Some(organic_key.clone());
            page.commit_reload(cx);
        })
    });
    cx.read(|cx| {
        let page = page.read(cx);
        let restored = page.table.read(cx).delegate().row_ix_for_key(&organic_key);
        assert!(restored.is_some());
        assert_eq!(page.table.read(cx).selected_row(), restored);
    });

    with_window(cx, &handle, &page, |page, window, cx| {
        page.segment_select.update(cx, |state, cx| {
            state.set_selected_index(Some(IndexPath::new(3)), window, cx);
        });
        page.on_segment_committed(cx);
        page.commit_reload(cx);
    });
    cx.read(|cx| {
        let page = page.read(cx);
        assert_eq!(page.table.read(cx).delegate().rows_count(cx), 12);
        assert_eq!(page.table.read(cx).selected_row(), None);
        assert_eq!(page.selected_key, None);
    });
}

#[gpui_kit::test]
async fn select_row_records_the_domain_key(cx: &TestAppContext) {
    let (handle, page) = open_page(cx);
    commit_first_reload(cx, &page);
    with_window(cx, &handle, &page, |page, window, cx| {
        page.on_table_event(&TableEvent::SelectRow(0), window, cx);
    });
    cx.read(|cx| {
        let page = page.read(cx);
        let expected = page.table.read(cx).delegate().key_at(0);
        assert_eq!(page.selected_key, expected);
    });
}

#[gpui_kit::test]
async fn double_click_opens_the_detail_sheet(cx: &TestAppContext) {
    let (handle, page) = open_page(cx);
    commit_first_reload(cx, &page);
    with_window(cx, &handle, &page, |page, window, cx| {
        page.on_table_event(&TableEvent::DoubleClickedRow(0), window, cx);
    });
    let open = cx.update(|cx| {
        handle
            .update(cx, |_, window, cx| window.has_active_sheet(cx))
            .unwrap()
    });
    assert!(open, "double-clicked row must open the detail sheet");
}

#[gpui_kit::test]
async fn open_detail_is_a_no_op_while_a_sheet_is_open(cx: &TestAppContext) {
    let (handle, page) = open_page(cx);
    commit_first_reload(cx, &page);
    with_window(cx, &handle, &page, |page, window, cx| {
        page.on_table_event(&TableEvent::DoubleClickedRow(0), window, cx);
        page.on_table_event(&TableEvent::DoubleClickedRow(1), window, cx);
    });
    let open = cx.update(|cx| {
        handle
            .update(cx, |_, window, cx| window.has_active_sheet(cx))
            .unwrap()
    });
    assert!(open, "the original sheet must still be the only one");
}

#[gpui_kit::test]
async fn preview_toggle_maps_onto_the_preview_state(cx: &TestAppContext) {
    let (_, page) = open_page(cx);
    let cases = [
        (super::PreviewStateToggle::Live, super::PreviewState::Live),
        (
            super::PreviewStateToggle::Loading,
            super::PreviewState::Loading,
        ),
        (super::PreviewStateToggle::Empty, super::PreviewState::Empty),
        (super::PreviewStateToggle::Error, super::PreviewState::Error),
    ];
    for (toggle, expected) in cases {
        cx.update(|cx| page.update(cx, |page, cx| page.set_preview(toggle, cx)));
        let actual = cx.read(|cx| page.read(cx).preview);
        assert_eq!(actual, expected);
    }
}

#[gpui_kit::test]
async fn page_renders_in_every_region_state_without_panicking(cx: &TestAppContext) {
    let (handle, page) = open_page(cx);
    let render_here = |cx: &TestAppContext| {
        cx.update(|cx| {
            handle
                .update(cx, |_, window, app| {
                    page.update(app, |page, page_cx| {
                        let _frame = page.render(window, page_cx);
                    });
                })
                .unwrap();
        });
    };
    render_here(cx);
    commit_first_reload(cx, &page);
    render_here(cx);
    for toggle in [
        super::PreviewStateToggle::Loading,
        super::PreviewStateToggle::Empty,
        super::PreviewStateToggle::Error,
    ] {
        cx.update(|cx| page.update(cx, |page, cx| page.set_preview(toggle, cx)));
        render_here(cx);
    }
    cx.update(|cx| {
        page.update(cx, |page, cx| {
            page.set_density(super::model::Density::Compact, cx)
        })
    });
    render_here(cx);
}

#[gpui_kit::test]
async fn fitted_column_width_survives_a_reload(cx: &TestAppContext) {
    let (handle, page) = open_page(cx);
    commit_first_reload(cx, &page);

    let fitted = {
        let mut fitted = px(0.);
        with_window(cx, &handle, &page, |page, window, cx| {
            page.table.update(cx, |table, table_cx| {
                assert!(table.fit_col(0, window, table_cx));
                fitted = table.col_width(0).expect("column 0 exists");
            });
        });
        fitted
    };
    assert_ne!(
        fitted,
        px(200.),
        "fitting the page column must move it off its spec width"
    );

    commit_first_reload(cx, &page);
    cx.read(|cx| {
        let table = page.read(cx).table.read(cx);
        assert_eq!(table.col_width(0), Some(fitted));
        assert_eq!(table.delegate().column(0, cx).width, fitted);
    });
}
