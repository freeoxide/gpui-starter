use gpui_kit::component::{
    IndexPath, WindowExt as _,
    table::{TableDelegate as _, TableEvent},
};
use gpui_kit::{
    AnyWindowHandle, AppContext as _, Bounds, Context, Entity, Pixels, Render as _, TestAppContext,
    VisualTestContext, Window, WindowOptions, point, px, size,
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

/// Opens the page in a window of an exact logical size, for bounds tests.
fn open_page_sized(cx: &TestAppContext, width: f32, height: f32) -> Page {
    let options = WindowOptions {
        window_bounds: Some(gpui_kit::WindowBounds::Windowed(Bounds {
            origin: point(px(0.), px(0.)),
            size: size(px(width), px(height)),
        })),
        ..WindowOptions::default()
    };
    let opened = cx.update(|cx| {
        gpui_kit::init(cx);
        gpui_kit::open_window(options, cx, |window, cx| {
            cx.new(|cx| AnalyticsPage::new(window, cx))
        })
    });
    opened.expect("test window must open")
}

/// Paints one frame and returns the recorded region bounds.
fn paint_regions(
    cx: &TestAppContext,
    handle: &AnyWindowHandle,
) -> Vec<(&'static str, Bounds<Pixels>)> {
    super::LAYOUT_PROBE.with_borrow_mut(|rows| rows.clear());
    let mut cx = VisualTestContext::from_window(handle.clone(), cx);
    cx.update(|window, cx| window.draw(cx).clear(cx));
    super::LAYOUT_PROBE.with_borrow(|rows| rows.clone())
}

fn all<'a>(regions: &'a [(&'static str, Bounds<Pixels>)], name: &str) -> Vec<Bounds<Pixels>> {
    regions
        .iter()
        .filter(|(n, _)| *n == name)
        .map(|(_, b)| *b)
        .collect()
}

fn distinct(bounds: &[Bounds<Pixels>]) -> Vec<Bounds<Pixels>> {
    let mut seen: Vec<Bounds<Pixels>> = Vec::new();
    for b in bounds {
        if !seen
            .iter()
            .any(|s| s.origin == b.origin && s.size == b.size)
        {
            seen.push(*b);
        }
    }
    seen
}

/// Pins the rebuilt structure: every key region resolves to definite bounds,
/// the four KPI cards form one aligned row of equal cards, and the paired
/// chart cards share a top and bottom line above the full-width sessions card.
#[gpui_kit::test]
async fn layout_keeps_regions_definite_and_columns_aligned(cx: &TestAppContext) {
    let (handle, page) = open_page_sized(cx, 1512., 982.);
    commit_first_reload(cx, &page);
    let regions = paint_regions(cx, &handle);

    // The toolbar carries no probe (a measuring child would change how its
    // wrap row resolves under taffy 0.15), so its height is pinned through
    // the next section's offset: 24 padding + toolbar + 24 gap.
    let kpis = all(&regions, "kpis");
    assert!(!kpis.is_empty(), "kpis section must render");
    let kpis_y = kpis[kpis.len() - 1].origin.y;
    assert!(
        kpis_y >= px(72.) && kpis_y <= px(96.),
        "toolbar must stay one row; kpis starts at {kpis_y:?}"
    );

    let kpi_cards = all(&regions, "kpi-card");
    assert_eq!(distinct(&kpi_cards).len(), 4, "four KPI cards");
    for card in &kpi_cards {
        assert!(card.size.width >= px(208.), "card width {card:?}");
        assert!(card.size.height >= px(80.), "card height {card:?}");
    }
    let ys: Vec<f32> = kpi_cards.iter().map(|b| b.origin.y.as_f32()).collect();
    let hs: Vec<f32> = kpi_cards.iter().map(|b| b.size.height.as_f32()).collect();
    let ws: Vec<f32> = kpi_cards.iter().map(|b| b.size.width.as_f32()).collect();
    assert!(
        ys.iter().all(|y| (y - ys[0]).abs() <= 0.5),
        "KPI cards must share one top line, got {ys:?}"
    );
    assert!(
        hs.iter().all(|h| (h - hs[0]).abs() <= 0.5),
        "KPI cards must share one height, got {hs:?}"
    );
    assert!(
        ws.iter().all(|w| (w - ws[0]).abs() <= 0.5),
        "KPI cards must take equal widths, got {ws:?}"
    );

    let cards = distinct(&all(&regions, "chart-card"));
    assert_eq!(cards.len(), 3, "three chart cards, got {cards:?}");
    let mut by_y = cards.clone();
    by_y.sort_by_key(|b| b.origin.y.as_f32() as i32);
    let peers = &by_y[..2];
    assert!(
        (peers[0].origin.y - peers[1].origin.y).abs() <= px(0.5),
        "paired chart cards must share a top line"
    );
    assert!(
        (peers[0].size.height - peers[1].size.height).abs() <= px(0.5),
        "paired chart cards must stretch to one height"
    );
    assert!(
        by_y[2].origin.y > peers[0].origin.y + px(100.),
        "sessions card below"
    );

    for plot in all(&regions, "chart-plot") {
        assert!(plot.size.height >= px(190.), "plot height {plot:?}");
        assert!(plot.size.width >= px(300.), "plot width {plot:?}");
    }

    let table_box = all(&regions, "table-box");
    assert!(!table_box.is_empty(), "table must render");
    let table_box = table_box[table_box.len() - 1];
    assert!((table_box.size.height - px(480.)).abs() <= px(1.));
    assert!((table_box.size.width - px(1464.)).abs() <= px(1.));
}

/// At a width where the KPI row cannot fit four cards they must wrap onto
/// two shared lines instead of collapsing or drifting mid-band.
#[gpui_kit::test]
async fn layout_wraps_kpi_cards_onto_aligned_lines(cx: &TestAppContext) {
    let (handle, page) = open_page_sized(cx, 650., 800.);
    commit_first_reload(cx, &page);
    let regions = paint_regions(cx, &handle);

    let cards = distinct(&all(&regions, "kpi-card"));
    assert_eq!(cards.len(), 4, "four KPI cards, got {cards:?}");
    let mut lines: Vec<f32> = cards.iter().map(|b| b.origin.y.as_f32()).collect();
    lines.sort_by(|a, b| a.partial_cmp(b).unwrap());
    lines.dedup_by(|a, b| (*a - *b).abs() <= 0.5);
    assert_eq!(lines.len(), 2, "two wrapped lines, got {lines:?}");
    for line in &lines {
        let row: Vec<&Bounds<Pixels>> = cards
            .iter()
            .filter(|b| (b.origin.y.as_f32() - line).abs() <= 0.5)
            .collect();
        assert_eq!(row.len(), 2, "two cards per line");
        assert!((row[0].size.height - row[1].size.height).abs() <= px(0.5));
        for card in &row {
            assert!(card.size.width >= px(208.));
            assert!(card.size.height >= px(80.));
        }
    }
}

/// The skeleton phase must occupy the same regions as the committed page so
/// the load never paints a collapse.
#[gpui_kit::test]
async fn layout_skeleton_regions_keep_definite_bounds(cx: &TestAppContext) {
    let (handle, _page) = open_page_sized(cx, 1512., 982.);
    let regions = paint_regions(cx, &handle);

    let skeletons = distinct(&all(&regions, "kpi-skeleton"));
    assert_eq!(skeletons.len(), 4, "four KPI skeletons");
    for card in &skeletons {
        assert!(card.size.width >= px(208.), "skeleton width {card:?}");
        assert!(card.size.height >= px(80.), "skeleton height {card:?}");
    }
    let charts = distinct(&all(&regions, "chart-skeleton"));
    assert_eq!(charts.len(), 3, "three chart skeletons, got {charts:?}");
    for card in &charts {
        assert!(card.size.width >= px(300.), "chart skeleton width {card:?}");
        assert!(
            card.size.height >= px(150.),
            "chart skeleton height {card:?}"
        );
    }
    for name in ["kpis", "kpi-row", "charts-row", "table-section"] {
        let found = all(&regions, name);
        assert!(!found.is_empty(), "{name} must render in skeleton phase");
        assert!(
            found
                .iter()
                .all(|b| b.size.width > px(0.) && b.size.height > px(0.))
        );
    }
}

/// Empty and error preview states must own the full region width instead of
/// shrinking to their content inside a wrap row.
#[gpui_kit::test]
async fn layout_empty_state_spans_the_region(cx: &TestAppContext) {
    let (handle, page) = open_page_sized(cx, 1512., 982.);
    commit_first_reload(cx, &page);
    cx.update(|cx| {
        page.update(cx, |page, cx| {
            page.set_preview(super::PreviewStateToggle::Empty, cx)
        })
    });
    let regions = paint_regions(cx, &handle);

    assert!(
        all(&regions, "charts-row").is_empty(),
        "charts replace with the state"
    );
    assert!(
        all(&regions, "table-box").is_empty(),
        "table replaces with the state"
    );
    let states = all(&regions, "region-state");
    assert!(states.len() >= 3, "one state per region, got {states:?}");
    for state in &states {
        assert!(state.size.width >= px(1400.), "state width {state:?}");
        assert!(state.size.height >= px(80.), "state height {state:?}");
    }
}

#[gpui_kit::test]
async fn scratch_check_toolbar_via_kpis(cx: &TestAppContext) {
    let (handle, page) = open_page_sized(cx, 1512., 982.);
    commit_first_reload(cx, &page);
    let regions = paint_regions(cx, &handle);
    let k = all(&regions, "kpis");
    let kpis = k[k.len() - 1];
    let toolbar_h = kpis.origin.y - px(24.) - px(24.);
    println!("kpis at {:?} => toolbar height {toolbar_h:?}", kpis.origin);
}
