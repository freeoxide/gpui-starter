//! Data Table section, ported from the upstream `DataTableStory`, with the
//! `table_in_scrollable` example folded in as a subsection.

use std::{ops::Range, sync::LazyLock, sync::Mutex, time::Duration};

use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, Size, StyleSized as _, StyledExt as _,
    button::{Button, DropdownButton},
    h_flex,
    menu::{PopupMenu, PopupMenuItem},
    spinner::Spinner,
    table::{Column, ColumnFixed, ColumnGroup, ColumnSort, DataTable, TableDelegate, TableState},
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};
use serde::Deserialize;

use crate::gallery::registry::GallerySection;

use super::demo::{DemoToggle, demo_toolbar, rng_f64, rng_usize, section, size_label};

actions!(gallery_tables, [ClearTableSelection]);

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_tables, no_json)]
enum TableRows {
    Rows100,
    Rows500,
    Rows5000,
    Rows10000,
    Rows1000000,
}

impl TableRows {
    fn count(&self) -> usize {
        match self {
            TableRows::Rows100 => 100,
            TableRows::Rows500 => 500,
            TableRows::Rows5000 => 5_000,
            TableRows::Rows10000 => 10_000,
            TableRows::Rows1000000 => 1_000_000,
        }
    }
}

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_tables, no_json)]
enum TableExtraColumns {
    Extra0,
    Extra4,
    Extra8,
    Extra16,
    Extra32,
}

impl TableExtraColumns {
    fn count(&self) -> usize {
        match self {
            TableExtraColumns::Extra0 => 0,
            TableExtraColumns::Extra4 => 4,
            TableExtraColumns::Extra8 => 8,
            TableExtraColumns::Extra16 => 16,
            TableExtraColumns::Extra32 => 32,
        }
    }
}

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_tables, no_json)]
enum TableOption {
    LoopSelection,
    ColumnResize,
    ColumnOrder,
    Sortable,
    ColumnSelection,
    RowSelection,
    CellSelection,
    RowHeader,
    FixedColumn,
    Striped,
    Loading,
    LazyLoad,
    RefreshData,
    GroupHeaders,
}

#[derive(Action, Clone, PartialEq, Eq, Deserialize)]
#[action(namespace = gallery_tables, no_json)]
enum TableGoTo {
    Top,
    Bottom,
    Cell53,
    Cell107,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct Counter {
    symbol: SharedString,
    market: SharedString,
    name: SharedString,
}

static ALL_COUNTERS: LazyLock<Vec<Counter>> =
    LazyLock::new(|| serde_json::from_str(include_str!("fixtures/counters.json")).unwrap());
static INCREMENT_ID: LazyLock<Mutex<usize>> = LazyLock::new(|| Mutex::new(0));

impl Counter {
    fn random() -> Self {
        let len = ALL_COUNTERS.len();
        ALL_COUNTERS[rng_usize(0, len)].clone()
    }

    /// The symbol carrying its market, e.g. `AAPL.US`; HK symbols already ship
    /// the suffix, so appending the market again would read `0700.HK.HK`.
    fn symbol_code(&self) -> SharedString {
        if self.symbol.contains('.') {
            self.symbol.clone()
        } else {
            format!("{}.{}", self.symbol, self.market).into()
        }
    }
}

#[derive(Clone, Debug, Default)]
struct Stock {
    id: usize,
    counter: Counter,
    price: f64,
    change: f64,
    change_percent: f64,
    volume: f64,
    turnover: f64,
    market_cap: f64,
    ttm: f64,
    five_mins_ranking: f64,
    th60_days_ranking: f64,
    year_change_percent: f64,
    bid: f64,
    bid_volume: f64,
    ask: f64,
    ask_volume: f64,
    open: f64,
    prev_close: f64,
    high: f64,
    low: f64,
    turnover_rate: f64,
    rise_rate: f64,
    amplitude: f64,
    pe_status: f64,
    pb_status: f64,
    volume_ratio: f64,
    bid_ask_ratio: f64,
    latest_pre_close: f64,
    latest_post_close: f64,
    pre_market_cap: f64,
    pre_market_percent: f64,
    pre_market_change: f64,
    post_market_cap: f64,
    post_market_percent: f64,
    post_market_change: f64,
    float_cap: f64,
    shares: i64,
    shares_float: i64,
    day_5_ranking: f64,
    day_10_ranking: f64,
    day_30_ranking: f64,
    day_120_ranking: f64,
    day_250_ranking: f64,
}

impl Stock {
    /// Ticks the quote, keeping the fields consistent with the new price.
    fn random_update(&mut self) {
        self.change_percent = rng_f64(-0.1, 0.1);
        self.price = rng_f64(5.0, 999.0);
        self.change = self.price * self.change_percent;
        self.prev_close = self.price - self.change;
        self.volume = rng_f64(1e4, 5e7);
        self.turnover = self.volume * self.price;
        self.market_cap = self.price * self.shares as f64;
        self.ttm = rng_f64(5.0, 80.0);
        self.five_mins_ranking *= 1.0 + rng_f64(-0.2, 0.2);
        self.bid = self.price * (1.0 - rng_f64(0.0, 0.01));
        self.bid_volume = rng_f64(100.0, 5e4);
        self.ask = self.price * (1.0 + rng_f64(0.0, 0.01));
        self.ask_volume = rng_f64(100.0, 5e4);
        self.bid_ask_ratio = self.bid_volume / self.ask_volume;
        self.volume_ratio = rng_f64(0.0, 3.0);
        self.high = self.price * (1.0 + rng_f64(0.0, 0.08));
        self.low = self.price * (1.0 - rng_f64(0.0, 0.08));
    }
}

/// (foreground, background) for a change value: a rise is green, a fall is
/// red, both from the theme; unchanged values keep the default cell colors.
fn change_colors(val: f64, cx: &App) -> Option<(Hsla, Hsla)> {
    if val > 0. {
        Some((cx.theme().green, cx.theme().green_light))
    } else if val < 0. {
        Some((cx.theme().red, cx.theme().red_light))
    } else {
        None
    }
}

/// Formats a large number with a compact unit, e.g. `1.24B`, so wide columns
/// stay readable.
fn compact(val: f64) -> String {
    const UNITS: [(f64, &str); 4] = [(1e12, "T"), (1e9, "B"), (1e6, "M"), (1e3, "K")];

    UNITS
        .into_iter()
        .find(|(scale, _)| val.abs() >= *scale)
        .map(|(scale, unit)| format!("{:.2}{unit}", val / scale))
        .unwrap_or_else(|| format!("{val:.2}"))
}

/// Restarts ids from zero and builds a fresh set of rows.
fn regenerate_stocks(size: usize) -> Vec<Stock> {
    *INCREMENT_ID.lock().unwrap() = 0;
    random_stocks(size)
}

fn random_stocks(size: usize) -> Vec<Stock> {
    // Incremental ID with size.
    let start = {
        let mut id_lock = INCREMENT_ID.lock().unwrap();
        let start = *id_lock;
        *id_lock += size + 1;
        start
    };

    // Drawing every field per row is slow in a debug build: draw a pool of rows
    // once and clone from it, each clone with its own id and counter.
    const POOL_SIZE: usize = 512;
    if size > POOL_SIZE * 2 {
        let pool = random_stocks_exact(0, POOL_SIZE);
        return (start..start + size)
            .map(|id| {
                let mut stock = pool[(id - start) % POOL_SIZE].clone();
                stock.id = id;
                stock.counter = Counter::random();
                stock
            })
            .collect();
    }

    random_stocks_exact(start, size)
}

/// Builds `size` rows whose fields hang together like a real quote: the change
/// is the price's percentage, and bid, ask, open, high and low sit around it.
fn random_stocks_exact(start: usize, size: usize) -> Vec<Stock> {
    (start..start + size)
        .map(|id| {
            let price = rng_f64(5.0, 999.0);
            let change_percent = rng_f64(-0.1, 0.1);
            let change = price * change_percent;
            let shares = rng_f64(1_000_000., 3_000_000_000.) as i64;
            let volume = rng_f64(1e4, 5e7);

            Stock {
                id,
                counter: Counter::random(),
                price,
                change,
                change_percent,
                volume,
                turnover: volume * price,
                market_cap: price * shares as f64,
                ttm: rng_f64(5.0, 80.0),
                five_mins_ranking: rng_f64(0.0, 1000.0),
                th60_days_ranking: rng_f64(0.0, 1000.0),
                year_change_percent: rng_f64(-1.0, 1.0),
                bid: price * (1.0 - rng_f64(0.0, 0.01)),
                bid_volume: rng_f64(100.0, 5e4),
                ask: price * (1.0 + rng_f64(0.0, 0.01)),
                ask_volume: rng_f64(100.0, 5e4),
                open: price * (1.0 + rng_f64(-0.05, 0.05)),
                prev_close: price - change,
                high: price * (1.0 + rng_f64(0.0, 0.08)),
                low: price * (1.0 - rng_f64(0.0, 0.08)),
                turnover_rate: rng_f64(0.0, 0.2),
                rise_rate: rng_f64(0.0, 1.0),
                amplitude: rng_f64(0.0, 0.15),
                pe_status: rng_f64(5.0, 60.0),
                pb_status: rng_f64(0.5, 12.0),
                volume_ratio: rng_f64(0.0, 3.0),
                bid_ask_ratio: rng_f64(0.0, 3.0),
                latest_pre_close: price * (1.0 + rng_f64(-0.03, 0.03)),
                latest_post_close: price * (1.0 + rng_f64(-0.03, 0.03)),
                pre_market_cap: price * shares as f64,
                pre_market_percent: rng_f64(-0.05, 0.05),
                pre_market_change: price * rng_f64(-0.05, 0.05),
                post_market_cap: price * shares as f64,
                post_market_percent: rng_f64(-0.05, 0.05),
                post_market_change: price * rng_f64(-0.05, 0.05),
                float_cap: price * shares as f64 * rng_f64(0.3, 1.0),
                shares,
                shares_float: (shares as f64 * rng_f64(0.3, 1.0)) as i64,
                day_5_ranking: rng_f64(0.0, 1000.0),
                day_10_ranking: rng_f64(0.0, 1000.0),
                day_30_ranking: rng_f64(0.0, 1000.0),
                day_120_ranking: rng_f64(0.0, 1000.0),
                day_250_ranking: rng_f64(0.0, 1000.0),
            }
        })
        .collect()
}

struct StockTableDelegate {
    stocks: Vec<Stock>,
    columns: Vec<Column>,
    /// Number of extra "Column N" columns appended after the built-in columns.
    extra_columns_count: usize,
    size: Size,
    loading: bool,
    lazy_load: bool,
    full_loading: bool,
    show_group_headers: bool,
    clicked_row: Option<usize>,
    eof: bool,
    visible_rows: Range<usize>,
    visible_cols: Range<usize>,

    _load_task: Task<()>,
}

impl StockTableDelegate {
    fn new(size: usize) -> Self {
        Self {
            size: Size::default(),
            stocks: random_stocks(size),
            lazy_load: false,
            clicked_row: None,
            columns: vec![
                Column::new("id", "ID")
                    .width(60.)
                    .fixed(ColumnFixed::Left)
                    .resizable(true)
                    .min_width(40.)
                    .max_width(100.),
                Column::new("market", "Market")
                    .width(60.)
                    .fixed(ColumnFixed::Left)
                    .resizable(true)
                    .min_width(50.),
                Column::new("name", "Name")
                    .width(180.)
                    .fixed(ColumnFixed::Left)
                    .max_width(300.),
                Column::new("symbol", "Symbol")
                    .width(100.)
                    .fixed(ColumnFixed::Left)
                    .sortable(),
                Column::new("price", "Price").sortable().text_right().p_0(),
                Column::new("change", "Chg").sortable().text_right().p_0(),
                Column::new("change_percent", "Chg%")
                    .sortable()
                    .text_right()
                    .p_0(),
                Column::new("volume", "Volume").text_right().p_0(),
                Column::new("turnover", "Turnover").text_right().p_0(),
                Column::new("market_cap", "Market Cap").text_right().p_0(),
                Column::new("ttm", "TTM").text_right().p_0(),
                Column::new("five_mins_ranking", "5m Ranking")
                    .text_right()
                    .p_0(),
                Column::new("th60_days_ranking", "60d Ranking").text_right(),
                Column::new("year_change_percent", "Year Chg%").text_right(),
                Column::new("bid", "Bid").text_right().p_0(),
                Column::new("bid_volume", "Bid Vol").text_right().p_0(),
                Column::new("ask", "Ask").text_right().p_0(),
                Column::new("ask_volume", "Ask Vol").text_right().p_0(),
                Column::new("open", "Open").text_right().p_0(),
                Column::new("prev_close", "Prev Close").text_right().p_0(),
                Column::new("high", "High").text_right().p_0(),
                Column::new("low", "Low").text_right().p_0(),
                Column::new("turnover_rate", "Turnover Rate").text_right(),
                Column::new("rise_rate", "Rise Rate").text_right(),
                Column::new("amplitude", "Amplitude").text_right(),
                Column::new("pe_status", "P/E").text_right(),
                Column::new("pb_status", "P/B").text_right(),
                Column::new("volume_ratio", "Volume Ratio")
                    .text_right()
                    .p_0(),
                Column::new("bid_ask_ratio", "Bid Ask Ratio")
                    .text_right()
                    .p_0(),
                Column::new("latest_pre_close", "Latest Pre Close").text_right(),
                Column::new("latest_post_close", "Latest Post Close").text_right(),
                Column::new("pre_market_cap", "Pre Mkt Cap").text_right(),
                Column::new("pre_market_percent", "Pre Mkt%").text_right(),
                Column::new("pre_market_change", "Pre Mkt Chg").text_right(),
                Column::new("post_market_cap", "Post Mkt Cap").text_right(),
                Column::new("post_market_percent", "Post Mkt%").text_right(),
                Column::new("post_market_change", "Post Mkt Chg").text_right(),
                Column::new("float_cap", "Float Cap").text_right(),
                Column::new("shares", "Shares").text_right(),
                Column::new("shares_float", "Float Shares").text_right(),
                Column::new("day_5_ranking", "5d Ranking").text_right(),
                Column::new("day_10_ranking", "10d Ranking").text_right(),
                Column::new("day_30_ranking", "30d Ranking").text_right(),
                Column::new("day_120_ranking", "120d Ranking").text_right(),
                Column::new("day_250_ranking", "250d Ranking").text_right(),
            ],
            extra_columns_count: 0,
            loading: false,
            full_loading: false,
            show_group_headers: true,
            eof: false,
            visible_cols: Range::default(),
            visible_rows: Range::default(),
            _load_task: Task::ready(()),
        }
    }

    fn set_stocks(&mut self, stocks: Vec<Stock>) {
        self.eof = stocks.len() <= 50;
        self.stocks = stocks;
        self.loading = false;
        self.full_loading = false;
    }

    /// The frame shared by value cells: fills the row and follows the column
    /// alignment, supplying padding for `p_0` columns that dropped their own.
    fn value_cell(&self, col: &Column) -> Div {
        div()
            .h_full()
            .h_flex()
            .items_center()
            .when(col.paddings.is_some(), |this| {
                this.table_cell_size(self.size)
            })
            .when(col.align == TextAlign::Right, |this| this.justify_end())
    }

    /// A plain number, already formatted by the caller.
    fn render_number(&self, col: &Column, text: String) -> AnyElement {
        self.value_cell(col).child(text).into_any_element()
    }

    /// A percentage, tinted over the whole cell as ticker tables do.
    fn render_percent(&self, col: &Column, val: f64, cx: &mut App) -> AnyElement {
        self.value_cell(col)
            .when_some(change_colors(val, cx), |this, (foreground, background)| {
                this.text_color(foreground).bg(background.alpha(0.05))
            })
            .child(format!("{:+.2}%", val * 100.))
            .into_any_element()
    }

    /// A signed change, colored by its direction.
    fn render_change(&self, col: &Column, val: f64, cx: &mut App) -> AnyElement {
        self.value_cell(col)
            .when_some(change_colors(val, cx), |this, (foreground, _)| {
                this.text_color(foreground)
            })
            .child(format!("{val:+.2}"))
            .into_any_element()
    }
}

impl TableDelegate for StockTableDelegate {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len() + self.extra_columns_count
    }

    fn rows_count(&self, _: &App) -> usize {
        self.stocks.len()
    }

    fn column(&self, col_ix: usize, _cx: &App) -> Column {
        if let Some(col) = self.columns.get(col_ix) {
            col.clone()
        } else {
            let n = col_ix - self.columns.len() + 1;
            Column::new(format!("extra_{n}"), format!("Column {n}"))
        }
    }

    fn group_headers(&self, cx: &App) -> Option<Vec<Vec<ColumnGroup>>> {
        if !self.show_group_headers {
            return None;
        }
        // Both rows must span every column, and the lower row subdivides the
        // upper one, so each group lines up with the columns it names.
        let trailing = self.columns_count(cx) - self.columns.len();

        Some(vec![
            vec![
                ColumnGroup {
                    label: "Stock".into(),
                    span: 4,
                },
                ColumnGroup {
                    label: "Market Data".into(),
                    span: 10,
                },
                ColumnGroup {
                    label: "Quotes".into(),
                    span: 8,
                },
                ColumnGroup {
                    label: "Stats".into(),
                    span: 7,
                },
                ColumnGroup {
                    label: "Extended Hours".into(),
                    span: 8,
                },
                ColumnGroup {
                    label: "Shares & Rankings".into(),
                    span: 8 + trailing,
                },
            ],
            vec![
                ColumnGroup {
                    label: "Identity".into(),
                    span: 4,
                },
                ColumnGroup {
                    label: "Price & Change".into(),
                    span: 3,
                },
                ColumnGroup {
                    label: "Turnover".into(),
                    span: 4,
                },
                ColumnGroup {
                    label: "Momentum".into(),
                    span: 3,
                },
                ColumnGroup {
                    label: "Order Book".into(),
                    span: 4,
                },
                ColumnGroup {
                    label: "Session".into(),
                    span: 4,
                },
                ColumnGroup {
                    label: "Activity".into(),
                    span: 3,
                },
                ColumnGroup {
                    label: "Valuation".into(),
                    span: 2,
                },
                ColumnGroup {
                    label: "Ratios".into(),
                    span: 2,
                },
                ColumnGroup {
                    label: "Pre & Post Market".into(),
                    span: 8,
                },
                ColumnGroup {
                    label: "Shares".into(),
                    span: 3,
                },
                ColumnGroup {
                    label: "Rankings".into(),
                    span: 5 + trailing,
                },
            ],
        ])
    }

    fn render_th(
        &mut self,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let col = self.column(col_ix, cx);

        div()
            // Same rule as the cells: supply the padding only for the columns
            // that dropped their own, so a header lines up with its column.
            .when(col.paddings.is_some(), |this| {
                this.table_cell_size(self.size)
            })
            .when(col.align == TextAlign::Center, |this| {
                this.h_flex().w_full().justify_center()
            })
            .when(col.align == TextAlign::Right, |this| {
                this.h_flex().w_full().justify_end()
            })
            .child(col.name.clone())
    }

    fn context_menu(
        &mut self,
        row_ix: usize,
        menu: PopupMenu,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> PopupMenu {
        let table = cx.entity();
        menu.item(
            PopupMenuItem::new(SharedString::from(format!("Selected Row: {}", row_ix))).on_click(
                move |_, _, cx| {
                    table.update(cx, |table, cx| {
                        table.delegate_mut().clicked_row = Some(row_ix);
                        cx.notify();
                    });
                },
            ),
        )
        .separator()
        .menu("Size 48px", Box::new(DemoToggle::Size48Px))
        .menu("Size Large", Box::new(DemoToggle::SizeLarge))
        .menu("Size Medium", Box::new(DemoToggle::SizeMedium))
        .menu("Size Small", Box::new(DemoToggle::SizeSmall))
        .menu("Size XSmall", Box::new(DemoToggle::SizeXSmall))
    }

    fn render_tr(
        &mut self,
        row_ix: usize,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> Stateful<Div> {
        div()
            .id(row_ix)
            .on_click(cx.listener(move |table, _: &ClickEvent, _window, cx| {
                table.delegate_mut().clicked_row = Some(row_ix);
                cx.notify();
            }))
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let stock = self.stocks.get(row_ix).unwrap();
        let Some(col) = self.columns.get(col_ix) else {
            return div().child("--").into_any_element();
        };

        match col.key.as_ref() {
            "id" => self
                .value_cell(&col)
                .text_color(cx.theme().muted_foreground)
                .when(col.align == TextAlign::Center, |this| this.justify_center())
                .child(stock.id.to_string())
                .into_any_element(),
            "market" => self
                .value_cell(&col)
                .map(|this| {
                    if stock.counter.market == "US" {
                        this.text_color(cx.theme().blue)
                    } else {
                        this.text_color(cx.theme().magenta)
                    }
                })
                .child(stock.counter.market.clone())
                .into_any_element(),
            "symbol" => self
                .value_cell(&col)
                .font_medium()
                .child(stock.counter.symbol_code())
                .into_any_element(),
            "name" => self
                .value_cell(&col)
                .child(div().truncate().child(stock.counter.name.clone()))
                .into_any_element(),
            "price" => self
                .value_cell(&col)
                .font_semibold()
                .child(format!("{:.2}", stock.price))
                .into_any_element(),
            "change" => self.render_change(&col, stock.change, cx),
            "change_percent" => self.render_percent(&col, stock.change_percent, cx),
            "volume" => self.render_number(&col, compact(stock.volume)),
            "turnover" => self.render_number(&col, compact(stock.turnover)),
            "market_cap" => self.render_number(&col, compact(stock.market_cap)),
            "ttm" => self.render_number(&col, compact(stock.ttm)),
            "five_mins_ranking" => {
                self.render_number(&col, format!("{:.0}", stock.five_mins_ranking))
            }
            "th60_days_ranking" => {
                self.render_number(&col, format!("{:.0}", stock.th60_days_ranking))
            }
            "year_change_percent" => self.render_percent(&col, stock.year_change_percent, cx),
            "bid" => self.render_number(&col, format!("{:.2}", stock.bid)),
            "bid_volume" => self.render_number(&col, compact(stock.bid_volume)),
            "ask" => self.render_number(&col, format!("{:.2}", stock.ask)),
            "ask_volume" => self.render_number(&col, compact(stock.ask_volume)),
            "open" => self.render_number(&col, format!("{:.2}", stock.open)),
            "prev_close" => self.render_number(&col, format!("{:.2}", stock.prev_close)),
            "high" => self.render_number(&col, format!("{:.2}", stock.high)),
            "low" => self.render_number(&col, format!("{:.2}", stock.low)),
            "turnover_rate" => {
                self.render_number(&col, format!("{:.2}%", stock.turnover_rate * 100.))
            }
            "rise_rate" => self.render_number(&col, format!("{:.2}%", stock.rise_rate * 100.)),
            "amplitude" => self.render_number(&col, format!("{:.2}%", stock.amplitude * 100.)),
            "pe_status" => self.render_number(&col, format!("{:.2}", stock.pe_status)),
            "pb_status" => self.render_number(&col, format!("{:.2}", stock.pb_status)),
            "volume_ratio" => self.render_number(&col, format!("{:.2}", stock.volume_ratio)),
            "bid_ask_ratio" => self.render_number(&col, format!("{:.2}", stock.bid_ask_ratio)),
            "latest_pre_close" => {
                self.render_number(&col, format!("{:.2}", stock.latest_pre_close))
            }
            "latest_post_close" => {
                self.render_number(&col, format!("{:.2}", stock.latest_post_close))
            }
            "pre_market_cap" => self.render_number(&col, compact(stock.pre_market_cap)),
            "pre_market_percent" => self.render_percent(&col, stock.pre_market_percent, cx),
            "pre_market_change" => self.render_change(&col, stock.pre_market_change, cx),
            "post_market_cap" => self.render_number(&col, compact(stock.post_market_cap)),
            "post_market_percent" => self.render_percent(&col, stock.post_market_percent, cx),
            "post_market_change" => self.render_change(&col, stock.post_market_change, cx),
            "float_cap" => self.render_number(&col, compact(stock.float_cap)),
            "shares" => self.render_number(&col, compact(stock.shares as f64)),
            "shares_float" => self.render_number(&col, compact(stock.shares_float as f64)),
            "day_5_ranking" => self.render_number(&col, format!("{:.0}", stock.day_5_ranking)),
            "day_10_ranking" => self.render_number(&col, format!("{:.0}", stock.day_10_ranking)),
            "day_30_ranking" => self.render_number(&col, format!("{:.0}", stock.day_30_ranking)),
            "day_120_ranking" => self.render_number(&col, format!("{:.0}", stock.day_120_ranking)),
            "day_250_ranking" => self.render_number(&col, format!("{:.0}", stock.day_250_ranking)),
            _ => self
                .value_cell(&col)
                .text_color(cx.theme().muted_foreground)
                .child("--")
                .into_any_element(),
        }
    }

    fn move_column(
        &mut self,
        col_ix: usize,
        to_ix: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) {
        let col = self.columns.remove(col_ix);
        self.columns.insert(to_ix, col);
    }

    fn perform_sort(
        &mut self,
        col_ix: usize,
        sort: ColumnSort,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) {
        if let Some(col) = self.columns.get_mut(col_ix) {
            match col.key.as_ref() {
                "id" => self.stocks.sort_by(|a, b| match sort {
                    ColumnSort::Descending => b.id.cmp(&a.id),
                    _ => a.id.cmp(&b.id),
                }),
                "symbol" => self.stocks.sort_by(|a, b| match sort {
                    ColumnSort::Descending => b.counter.symbol.cmp(&a.counter.symbol),
                    _ => a.id.cmp(&b.id),
                }),
                "change" | "change_percent" => self.stocks.sort_by(|a, b| match sort {
                    ColumnSort::Descending => b
                        .change
                        .partial_cmp(&a.change)
                        .unwrap_or(std::cmp::Ordering::Equal),
                    _ => a.id.cmp(&b.id),
                }),
                _ => {}
            }
        }
    }

    fn loading(&self, _: &App) -> bool {
        self.full_loading
    }

    fn has_more(&self, _: &App) -> bool {
        if !self.lazy_load {
            return false;
        }
        if self.loading {
            return false;
        }

        !self.eof
    }

    fn load_more_threshold(&self) -> usize {
        150
    }

    fn load_more(&mut self, _: &mut Window, cx: &mut Context<TableState<Self>>) {
        if !self.lazy_load {
            return;
        }

        self.loading = true;

        self._load_task = cx.spawn(async move |view, cx| {
            // Simulate network request, delay 1s to load data.
            cx.background_executor().timer(Duration::from_secs(1)).await;

            _ = cx.update(|cx| {
                let _ = view.update(cx, |view, _| {
                    view.delegate_mut().stocks.extend(random_stocks(200));
                    view.delegate_mut().loading = false;
                    view.delegate_mut().eof = view.delegate().stocks.len() >= 6000;
                });
            });
        });
    }

    fn visible_rows_changed(
        &mut self,
        visible_range: Range<usize>,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) {
        self.visible_rows = visible_range;
    }

    fn visible_columns_changed(
        &mut self,
        visible_range: Range<usize>,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) {
        self.visible_cols = visible_range;
    }

    fn cell_text(&self, row_ix: usize, col_ix: usize, _cx: &App) -> String {
        let Some(stock) = self.stocks.get(row_ix) else {
            return String::new();
        };
        let Some(col) = self.columns.get(col_ix) else {
            return String::new();
        };

        match col.key.as_ref() {
            "id" => stock.id.to_string(),
            "market" => stock.counter.market.to_string(),
            "symbol" => stock.counter.symbol_code().to_string(),
            "name" => stock.counter.name.to_string(),
            "price" => format!("{:.3}", stock.price),
            "change" => format!("{:.3}", stock.change),
            "change_percent" => format!("{:.2}%", stock.change_percent * 100.),
            "volume" => format!("{:.3}", stock.volume),
            "turnover" => format!("{:.3}", stock.turnover),
            "market_cap" => format!("{:.3}", stock.market_cap),
            "ttm" => format!("{:.3}", stock.ttm),
            "five_mins_ranking" => format!("{:.3}", stock.five_mins_ranking),
            "th60_days_ranking" => stock.th60_days_ranking.floor().to_string(),
            "year_change_percent" => format!("{:.2}%", stock.year_change_percent * 100.),
            "bid" => format!("{:.3}", stock.bid),
            "bid_volume" => format!("{:.3}", stock.bid_volume),
            "ask" => format!("{:.3}", stock.ask),
            "ask_volume" => format!("{:.3}", stock.ask_volume),
            "open" => format!("{:.3}", stock.open),
            "prev_close" => format!("{:.3}", stock.prev_close),
            "high" => format!("{:.3}", stock.high),
            "low" => format!("{:.3}", stock.low),
            "turnover_rate" => format!("{:.0}", stock.turnover_rate * 100.),
            "rise_rate" => format!("{:.0}", stock.rise_rate * 100.),
            "amplitude" => format!("{:.0}", stock.amplitude * 100.),
            "pe_status" => stock.pe_status.floor().to_string(),
            "pb_status" => stock.pb_status.floor().to_string(),
            "volume_ratio" => format!("{:.3}", stock.volume_ratio),
            "bid_ask_ratio" => format!("{:.3}", stock.bid_ask_ratio),
            "latest_pre_close" => stock.latest_pre_close.floor().to_string(),
            "latest_post_close" => stock.latest_post_close.floor().to_string(),
            "pre_market_cap" => stock.pre_market_cap.floor().to_string(),
            "pre_market_percent" => format!("{:.2}%", stock.pre_market_percent * 100.),
            "pre_market_change" => stock.pre_market_change.floor().to_string(),
            "post_market_cap" => stock.post_market_cap.floor().to_string(),
            "post_market_percent" => format!("{:.2}%", stock.post_market_percent * 100.),
            "post_market_change" => stock.post_market_change.floor().to_string(),
            "float_cap" => stock.float_cap.floor().to_string(),
            "shares" => stock.shares.to_string(),
            "shares_float" => stock.shares_float.to_string(),
            "day_5_ranking" => stock.day_5_ranking.floor().to_string(),
            "day_10_ranking" => stock.day_10_ranking.floor().to_string(),
            "day_30_ranking" => stock.day_30_ranking.floor().to_string(),
            "day_120_ranking" => stock.day_120_ranking.floor().to_string(),
            "day_250_ranking" => stock.day_250_ranking.floor().to_string(),
            _ => String::new(),
        }
    }
}

/// The small fixed table the scrollable-page demo embeds.
struct UsersTable {
    columns: Vec<Column>,
}

impl UsersTable {
    fn new(_: &mut App) -> Self {
        let columns = vec![
            Column::new("id", "ID").width(50.),
            Column::new("name", "Name").width(150.),
            Column::new("email", "Email").width(250.),
            Column::new("role", "Role").width(150.),
            Column::new("status", "Status").width(100.),
        ];

        Self { columns }
    }
}

impl TableDelegate for UsersTable {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        30
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        self.columns[col_ix].clone()
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        match col_ix {
            0 => format!("{}", row_ix).into_any_element(),
            1 => format!("User {}", row_ix).into_any_element(),
            2 => format!("user-{}@mail.com", row_ix).into_any_element(),
            3 => "User".into_any_element(),
            4 => "Active".into_any_element(),
            _ => panic!("Invalid column index"),
        }
    }
}

pub struct DataTableSection {
    table: Entity<TableState<StockTableDelegate>>,
    scrollable_table: Entity<TableState<UsersTable>>,
    stripe: bool,
    refresh_data: bool,
    size: Size,

    _load_task: Task<()>,
    _load_rows_task: Task<()>,
}

impl DataTableSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let delegate = StockTableDelegate::new(5000);
        let table = cx.new(|cx| TableState::new(delegate, window, cx));
        let scrollable_table = cx.new(|cx| TableState::new(UsersTable::new(cx), window, cx));

        let _load_task = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(33))
                    .await;

                this.update(cx, |this, cx| {
                    if !this.refresh_data {
                        return;
                    }

                    this.table.update(cx, |table, _| {
                        // Only walk the head: the table paints a few dozen rows at a
                        // time, so ticking every row of a million-row set stalls.
                        const MAX_REFRESH_ROWS: usize = 2_000;
                        table
                            .delegate_mut()
                            .stocks
                            .iter_mut()
                            .take(MAX_REFRESH_ROWS)
                            .enumerate()
                            .for_each(|(i, stock)| {
                                let n = rng_usize(3, 10);
                                // update 30% of the stocks
                                if i % n == 0 {
                                    stock.random_update();
                                }
                            });
                    });
                    cx.notify();
                })
                .ok();
            }
        });

        Self {
            table,
            scrollable_table,
            stripe: false,
            refresh_data: false,
            size: Size::default(),
            _load_task,
            _load_rows_task: Task::ready(()),
        }
    }

    fn filler(&self, label: &str, height: Rems, cx: &App) -> impl IntoElement {
        div()
            .h(height)
            .w_full()
            .flex()
            .items_center()
            .justify_center()
            .border_1()
            .border_dashed()
            .border_color(cx.theme().border)
            .text_color(cx.theme().muted_foreground)
            .child(SharedString::from(label.to_string()))
    }
}

fn add_size_items_with_custom(menu: PopupMenu, size: Size) -> PopupMenu {
    menu.menu_with_check(
        "48px",
        size == Size::Size(px(48.)),
        Box::new(DemoToggle::Size48Px),
    )
    .menu_with_check(
        "Large",
        size == Size::Large,
        Box::new(DemoToggle::SizeLarge),
    )
    .menu_with_check(
        "Medium",
        size == Size::Medium,
        Box::new(DemoToggle::SizeMedium),
    )
    .menu_with_check(
        "Small",
        size == Size::Small,
        Box::new(DemoToggle::SizeSmall),
    )
    .menu_with_check(
        "XSmall",
        size == Size::XSmall,
        Box::new(DemoToggle::SizeXSmall),
    )
}

impl Render for DataTableSection {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let table = self.table.read(cx);
        let delegate = table.delegate();
        let rows_count = delegate.rows_count(cx);
        let columns_count = delegate.columns_count(cx);
        let size = self.size;
        let loop_selection = table.loop_selection;
        let col_resizable = table.col_resizable;
        let col_movable = table.col_movable;
        let sortable = table.sortable;
        let col_selectable = table.col_selectable;
        let row_selectable = table.row_selectable;
        let cell_selectable = table.cell_selectable;
        let row_header = table.row_header;
        let col_fixed = table.col_fixed;
        let striped = self.stripe;
        let full_loading = delegate.full_loading;
        let lazy_load = delegate.lazy_load;
        let refresh_data = self.refresh_data;
        let show_group_headers = delegate.show_group_headers;
        let loading = delegate.loading;
        let visible_rows = delegate.visible_rows.clone();
        let visible_cols = delegate.visible_cols.clone();
        let clicked_row = delegate.clicked_row;
        let selected_cell = table.selected_cell();
        let eof = delegate.eof;
        let extra_columns_count = delegate.extra_columns_count;

        v_flex()
            .on_action(cx.listener(|this, action: &DemoToggle, _, cx| {
                if let Some(size) = action.size() {
                    this.size = size;
                    cx.notify();
                }
            }))
            .on_action(cx.listener(|this, action: &TableRows, _, cx| {
                let count = action.count();
                if count == this.table.read(cx).delegate().stocks.len() {
                    return;
                }

                // Building the largest dataset allocates a few hundred MB; show the
                // loading state, build off-thread, then swap the rows in.
                this.table.update(cx, |table, cx| {
                    table.delegate_mut().full_loading = true;
                    cx.notify();
                });
                this._load_rows_task = cx.spawn(async move |this, cx| {
                    let stocks = cx
                        .background_spawn(async move { regenerate_stocks(count) })
                        .await;

                    this.update(cx, |this, cx| {
                        this.table.update(cx, |table, cx| {
                            table.delegate_mut().set_stocks(stocks);
                            table.refresh(cx);
                        });
                        cx.notify();
                    })
                    .ok();
                });
            }))
            .on_action(cx.listener(|this, action: &TableExtraColumns, _, cx| {
                this.table.update(cx, |table, cx| {
                    table.delegate_mut().extra_columns_count = action.count();
                    table.refresh(cx);
                });
                cx.notify();
            }))
            .on_action(cx.listener(|this, _: &ClearTableSelection, _, cx| {
                this.table.update(cx, |table, cx| table.clear_selection(cx));
            }))
            .on_action(cx.listener(|this, action: &TableGoTo, _, cx| {
                this.table.update(cx, |table, cx| match action {
                    TableGoTo::Top => table.scroll_to_row(0, cx),
                    TableGoTo::Bottom => {
                        table.scroll_to_row(table.delegate().rows_count(cx) - 1, cx)
                    }
                    TableGoTo::Cell53 => table.set_selected_cell(5, 3, cx),
                    TableGoTo::Cell107 => table.set_selected_cell(10, 7, cx),
                });
            }))
            .on_action(
                cx.listener(|this, action: &TableOption, _, cx| match action {
                    TableOption::LoopSelection => {
                        this.table.update(cx, |table, cx| {
                            table.loop_selection = !table.loop_selection;
                            cx.notify();
                        });
                    }
                    TableOption::ColumnResize => {
                        this.table.update(cx, |table, cx| {
                            table.col_resizable = !table.col_resizable;
                            cx.notify();
                        });
                    }
                    TableOption::ColumnOrder => {
                        this.table.update(cx, |table, cx| {
                            table.col_movable = !table.col_movable;
                            cx.notify();
                        });
                    }
                    TableOption::Sortable => {
                        this.table.update(cx, |table, cx| {
                            table.sortable = !table.sortable;
                            cx.notify();
                        });
                    }
                    TableOption::ColumnSelection => {
                        this.table.update(cx, |table, cx| {
                            table.col_selectable = !table.col_selectable;
                            cx.notify();
                        });
                    }
                    TableOption::RowSelection => {
                        this.table.update(cx, |table, cx| {
                            table.row_selectable = !table.row_selectable;
                            cx.notify();
                        });
                    }
                    TableOption::CellSelection => {
                        this.table.update(cx, |table, cx| {
                            table.cell_selectable = !table.cell_selectable;
                            cx.notify();
                        });
                    }
                    TableOption::RowHeader => {
                        this.table.update(cx, |table, cx| {
                            table.row_header = !table.row_header;
                            cx.notify();
                        });
                    }
                    TableOption::FixedColumn => {
                        this.table.update(cx, |table, cx| {
                            table.col_fixed = !table.col_fixed;
                            cx.notify();
                        });
                    }
                    TableOption::Striped => {
                        this.stripe = !this.stripe;
                        cx.notify();
                    }
                    TableOption::Loading => {
                        this.table.update(cx, |table, cx| {
                            table.delegate_mut().full_loading = !table.delegate().full_loading;
                            cx.notify();
                        });
                    }
                    TableOption::LazyLoad => {
                        this.table.update(cx, |table, cx| {
                            table.delegate_mut().lazy_load = !table.delegate().lazy_load;
                            cx.notify();
                        });
                    }
                    TableOption::RefreshData => {
                        this.refresh_data = !this.refresh_data;
                        cx.notify();
                    }
                    TableOption::GroupHeaders => {
                        this.table.update(cx, |table, cx| {
                            table.delegate_mut().show_group_headers =
                                !table.delegate().show_group_headers;
                            table.refresh_header_layout(cx);
                        });
                    }
                }),
            )
            .w_full()
            .items_center()
            .gap_6()
            .p_4()
            .text_sm()
            .child(demo_toolbar(vec![
                DropdownButton::new("data-table-size")
                    .button(
                        Button::new("data-table-size-trigger")
                            .label(format!("Size: {}", size_label(size))),
                    )
                    .dropdown_menu(move |menu, _, _| add_size_items_with_custom(menu, size))
                    .into_any_element(),
                DropdownButton::new("data-table-rows")
                    .button(
                        Button::new("data-table-rows-trigger").label(format!("Rows: {rows_count}")),
                    )
                    .dropdown_menu(move |menu, _, _| {
                        menu.menu_with_check("100", rows_count == 100, Box::new(TableRows::Rows100))
                            .menu_with_check("500", rows_count == 500, Box::new(TableRows::Rows500))
                            .menu_with_check(
                                "5,000",
                                rows_count == 5_000,
                                Box::new(TableRows::Rows5000),
                            )
                            .menu_with_check(
                                "10,000",
                                rows_count == 10_000,
                                Box::new(TableRows::Rows10000),
                            )
                            .menu_with_check(
                                "1,000,000",
                                rows_count == 1_000_000,
                                Box::new(TableRows::Rows1000000),
                            )
                    })
                    .into_any_element(),
                DropdownButton::new("data-table-extra-columns")
                    .button(
                        Button::new("data-table-extra-columns-trigger")
                            .label(format!("Extra Columns: {extra_columns_count}")),
                    )
                    .dropdown_menu(move |menu, _, _| {
                        menu.menu_with_check(
                            "None",
                            extra_columns_count == 0,
                            Box::new(TableExtraColumns::Extra0),
                        )
                        .menu_with_check(
                            "4",
                            extra_columns_count == 4,
                            Box::new(TableExtraColumns::Extra4),
                        )
                        .menu_with_check(
                            "8",
                            extra_columns_count == 8,
                            Box::new(TableExtraColumns::Extra8),
                        )
                        .menu_with_check(
                            "16",
                            extra_columns_count == 16,
                            Box::new(TableExtraColumns::Extra16),
                        )
                        .menu_with_check(
                            "32",
                            extra_columns_count == 32,
                            Box::new(TableExtraColumns::Extra32),
                        )
                    })
                    .into_any_element(),
                DropdownButton::new("data-table-options")
                    .button(Button::new("data-table-options-trigger").label("Options"))
                    .dropdown_menu(move |menu, _, _| {
                        menu.menu_with_check(
                            "Loop Selection",
                            loop_selection,
                            Box::new(TableOption::LoopSelection),
                        )
                        .menu_with_check(
                            "Column Resize",
                            col_resizable,
                            Box::new(TableOption::ColumnResize),
                        )
                        .menu_with_check(
                            "Column Order",
                            col_movable,
                            Box::new(TableOption::ColumnOrder),
                        )
                        .menu_with_check("Sortable", sortable, Box::new(TableOption::Sortable))
                        .menu_with_check(
                            "Column Selectable",
                            col_selectable,
                            Box::new(TableOption::ColumnSelection),
                        )
                        .menu_with_check(
                            "Row Selectable",
                            row_selectable,
                            Box::new(TableOption::RowSelection),
                        )
                        .menu_with_check(
                            "Cell Selectable",
                            cell_selectable,
                            Box::new(TableOption::CellSelection),
                        )
                        .menu_with_check("Row Header", row_header, Box::new(TableOption::RowHeader))
                        .menu_with_check(
                            "Fixed Column",
                            col_fixed,
                            Box::new(TableOption::FixedColumn),
                        )
                        .separator()
                        .menu_with_check("Striped Rows", striped, Box::new(TableOption::Striped))
                        .menu_with_check("Loading", full_loading, Box::new(TableOption::Loading))
                        .menu_with_check("Lazy Load", lazy_load, Box::new(TableOption::LazyLoad))
                        .menu_with_check(
                            "Refresh Data",
                            refresh_data,
                            Box::new(TableOption::RefreshData),
                        )
                        .menu_with_check(
                            "Group Headers",
                            show_group_headers,
                            Box::new(TableOption::GroupHeaders),
                        )
                        .separator()
                        .menu("Clear Selection", Box::new(ClearTableSelection))
                    })
                    .into_any_element(),
                DropdownButton::new("data-table-go-to")
                    .button(Button::new("data-table-go-to-trigger").label("Go To"))
                    .dropdown_menu(|menu, _, _| {
                        menu.menu("Top", Box::new(TableGoTo::Top))
                            .menu("Bottom", Box::new(TableGoTo::Bottom))
                            .separator()
                            .menu("Cell 5:3", Box::new(TableGoTo::Cell53))
                            .menu("Cell 10:7", Box::new(TableGoTo::Cell107))
                    })
                    .into_any_element(),
            ]))
            .child(
                section("data-table-quotes", "Stock quotes")
                    .description(
                        "Sort, resize, and reorder columns; select rows, columns, or cells; and \
                        load more rows as the viewport nears the end.",
                    )
                    .w_full()
                    .v_flex()
                    .child(
                        div().h(rems(30.)).w_full().child(
                            DataTable::new(&self.table)
                                .with_size(self.size)
                                .stripe(self.stripe),
                        ),
                    )
                    .child(
                        h_flex()
                            .w_full()
                            .min_h_9()
                            .items_center()
                            .justify_between()
                            .gap_3()
                            .px_3()
                            .bg(cx.theme().muted.opacity(0.35))
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!(
                                "Total · {} rows · {} columns",
                                rows_count, columns_count
                            ))
                            .child(
                                h_flex()
                                    .items_center()
                                    .justify_end()
                                    .gap_2()
                                    .when(loading, |this| this.child(Spinner::new().xsmall()))
                                    .child(format!(
                                        "Current · rows {:?} · columns {:?}",
                                        visible_rows, visible_cols
                                    ))
                                    .when_some(selected_cell, |this, (row, col)| {
                                        this.child(format!("· cell {}:{}", row, col))
                                    })
                                    .when_some(clicked_row, |this, row| {
                                        this.child(format!("· clicked row {}", row))
                                    })
                                    .when(eof, |this| this.child("· complete")),
                            ),
                    ),
            )
            .child(
                section("data-table-in-scrollable", "Nested in a scrollable page")
                    .description(
                        "The table carries its own scrollbar: the wheel scrolls its rows first, \
                        then this pane once the table reaches its edge or the cursor leaves it.",
                    )
                    .w_full()
                    .v_flex()
                    .child(self.filler("Content above the table", rems(25.), cx))
                    .child(
                        div().h(rems(18.75)).w_full().child(
                            DataTable::new(&self.scrollable_table)
                                .stripe(true)
                                .bordered(true),
                        ),
                    )
                    .child(self.filler("Content below the table", rems(50.), cx)),
            )
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "data-table",
        "Data Table",
        "A virtualized table with selection, sorting, column moving, lazy loading, and one \
        nested inside a scrolling page.",
        DataTableSection::view(window, cx),
    ));
}
