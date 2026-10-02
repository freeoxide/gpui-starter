//! System Monitor section, ported from the upstream `system_monitor` example.
//! The example's sysinfo and battery live feeds are replaced by the story
//! fixture data: a rolling window slides through the daily-devices series on
//! the example's own 500 ms loop.

use std::collections::VecDeque;
use std::time::Duration;

use gpui_kit::component::{
    ActiveTheme as _, Icon, Sizable as _,
    chart::AreaChart,
    h_flex,
    progress::Progress,
    tab::{Tab, TabBar},
    table::{Column, ColumnSort, DataTable, TableDelegate, TableState},
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

use crate::gallery::registry::GallerySection;

use super::chart::{DailyDevice, compact, fixture};

const INTERVAL: Duration = Duration::from_millis(500);
/// The rolling window the charts show. The example kept 120 samples of a live
/// feed; the fixture has 91 days, so the window slides through those instead.
const MAX_DATA_POINTS: usize = 40;

const MONITOR_SVG: &[u8] = include_bytes!("assets/monitor.svg");
const SMARTPHONE_SVG: &[u8] = include_bytes!("assets/smartphone.svg");
const TABLET_SVG: &[u8] = include_bytes!("assets/tablet.svg");

/// Tab indices
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum MonitorTab {
    #[default]
    Overview = 0,
    Devices = 1,
}

impl MonitorTab {
    fn from_index(index: usize) -> Self {
        match index {
            1 => MonitorTab::Devices,
            _ => MonitorTab::Overview,
        }
    }
}

/// Sort field for the devices table
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum DeviceSortField {
    Date,
    #[default]
    Desktop,
    Mobile,
    Tablet,
    Watch,
}

/// Devices table delegate
struct DeviceTableDelegate {
    rows: Vec<DailyDevice>,
    columns: Vec<Column>,
    sort_field: DeviceSortField,
    sort_order: ColumnSort,
}

impl DeviceTableDelegate {
    fn new(rows: Vec<DailyDevice>) -> Self {
        let mut delegate = Self {
            rows,
            columns: vec![
                Column::new("date", "Date").width(90.).sortable(),
                Column::new("desktop", "Desktop")
                    .width(80.)
                    .sortable()
                    .sort(ColumnSort::Descending),
                Column::new("mobile", "Mobile").width(80.).sortable(),
                Column::new("tablet", "Tablet").width(80.).sortable(),
                Column::new("watch", "Watch").width(80.).sortable(),
            ],
            sort_field: DeviceSortField::Desktop,
            sort_order: ColumnSort::Descending,
        };
        delegate.sort_rows();
        delegate
    }

    fn sort_rows(&mut self) {
        let is_descending = matches!(self.sort_order, ColumnSort::Descending);

        match self.sort_field {
            DeviceSortField::Date => self.rows.sort_by(|a, b| {
                let cmp = a.date.cmp(&b.date);
                if is_descending { cmp.reverse() } else { cmp }
            }),
            DeviceSortField::Desktop => self.sort_by_count(is_descending, |d| d.desktop),
            DeviceSortField::Mobile => self.sort_by_count(is_descending, |d| d.mobile),
            DeviceSortField::Tablet => self.sort_by_count(is_descending, |d| d.tablet),
            DeviceSortField::Watch => self.sort_by_count(is_descending, |d| d.watch),
        }
    }

    fn sort_by_count(&mut self, is_descending: bool, count: impl Fn(&DailyDevice) -> f64) {
        self.rows.sort_by(|a, b| {
            let cmp = count(a)
                .partial_cmp(&count(b))
                .unwrap_or(std::cmp::Ordering::Equal);
            if is_descending { cmp.reverse() } else { cmp }
        });
    }
}

impl TableDelegate for DeviceTableDelegate {
    fn columns_count(&self, _cx: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _cx: &App) -> usize {
        self.rows.len()
    }

    fn column(&self, col_ix: usize, _cx: &App) -> Column {
        self.columns[col_ix].clone()
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _window: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let Some(row) = self.rows.get(row_ix) else {
            return div().into_any_element();
        };

        let count = |value: f64| {
            div()
                .text_xs()
                .text_color(cx.theme().foreground)
                .child(compact(value))
                .into_any_element()
        };

        match col_ix {
            0 => div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(row.date.clone())
                .into_any_element(),
            1 => count(row.desktop),
            2 => count(row.mobile),
            3 => count(row.tablet),
            4 => count(row.watch),
            _ => div().into_any_element(),
        }
    }

    fn perform_sort(
        &mut self,
        col_ix: usize,
        sort: ColumnSort,
        _window: &mut Window,
        _cx: &mut Context<TableState<Self>>,
    ) {
        self.sort_order = sort;
        self.sort_field = match col_ix {
            0 => DeviceSortField::Date,
            1 => DeviceSortField::Desktop,
            2 => DeviceSortField::Mobile,
            3 => DeviceSortField::Tablet,
            4 => DeviceSortField::Watch,
            _ => DeviceSortField::Desktop,
        };
        self.sort_rows();
    }
}

/// A monitor over fixture data: the charts and gauges read a rolling window of
/// the daily-devices series, which advances on the example's timer.
pub struct SystemMonitorSection {
    devices: Vec<DailyDevice>,
    data: VecDeque<DailyDevice>,
    /// The fixture index the next tick pushes.
    head: usize,
    active_tab: MonitorTab,
    device_table: Entity<TableState<DeviceTableDelegate>>,
}

impl SystemMonitorSection {
    pub fn view(window: &mut Window, cx: &mut App) -> Entity<Self> {
        cx.new(|cx| Self::new(window, cx))
    }

    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let devices: Vec<DailyDevice> = fixture(include_str!("fixtures/daily-devices.json"));
        let device_table = cx.new(|cx| {
            TableState::new(DeviceTableDelegate::new(devices.clone()), window, cx)
                .col_selectable(false)
                .col_movable(false)
        });

        let data: VecDeque<_> = devices.iter().take(MAX_DATA_POINTS).cloned().collect();
        let head = data.len() % devices.len();

        let monitor = Self {
            devices,
            data,
            head,
            active_tab: MonitorTab::default(),
            device_table,
        };

        // Start the update loop
        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(INTERVAL).await;

                let result = this.update(cx, |this, cx| {
                    this.advance(cx);
                });

                if result.is_err() {
                    break;
                }
            }
        })
        .detach();

        monitor
    }

    /// Slides the window one day forward, wrapping at the end of the fixture.
    fn advance(&mut self, cx: &mut Context<Self>) {
        let next = self.devices[self.head].clone();
        self.head = (self.head + 1) % self.devices.len();
        if self.data.len() >= MAX_DATA_POINTS {
            self.data.pop_front();
        }
        self.data.push_back(next);
        cx.notify();
    }

    fn set_active_tab(&mut self, index: usize, _window: &mut Window, cx: &mut Context<Self>) {
        self.active_tab = MonitorTab::from_index(index);
        cx.notify();
    }

    fn render_chart(
        &self,
        title: &str,
        color: Hsla,
        share: impl Fn(&DailyDevice) -> f64 + 'static,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let data: Vec<DailyDevice> = self.data.iter().cloned().collect();
        v_flex()
            .min_h(rems(10.))
            .flex_1()
            .gap_2()
            .border_1()
            .border_color(cx.theme().border)
            .child(
                h_flex()
                    .justify_between()
                    .py_1()
                    .px_3()
                    .child(
                        div()
                            .text_sm()
                            .text_color(cx.theme().foreground)
                            .child(title.to_string()),
                    )
                    .child({
                        let current_value = data.last().map(&share).unwrap_or(0.0);
                        div()
                            .text_sm()
                            .text_color(color)
                            .child(format!("{:.1}%", current_value))
                    }),
            )
            .child(
                AreaChart::new(data)
                    .x(|d| d.date.clone())
                    .y(move |d| share(d))
                    .stroke(color)
                    .fill(linear_gradient(
                        0.,
                        linear_color_stop(color.opacity(0.4), 1.),
                        linear_color_stop(cx.theme().background.opacity(0.1), 0.),
                    ))
                    .tick_margin(15),
            )
    }

    fn render_overview_tab(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .p_3()
            .gap_4()
            .flex_1()
            .child(self.render_chart(
                "Desktop share of visitors",
                cx.theme().chart_2,
                |d| d.desktop / device_total(d) * 100.,
                cx,
            ))
            .child(self.render_chart(
                "Mobile share of visitors",
                cx.theme().chart_4,
                |d| d.mobile / device_total(d) * 100.,
                cx,
            ))
    }

    fn render_devices_tab(&self, _cx: &Context<Self>) -> impl IntoElement {
        v_flex().size_full().child(
            div().h(rems(30.)).w_full().child(
                DataTable::new(&self.device_table)
                    .bordered(false)
                    .stripe(true)
                    .small(),
            ),
        )
    }

    fn render_gauge(
        &self,
        id: &'static str,
        icon: &'static [u8],
        percent: f32,
    ) -> impl IntoElement {
        h_flex()
            .gap_2()
            .w(rems(8.4375))
            .items_center()
            .child(Icon::default().data(icon))
            .child(Progress::new(id).w_12().h_2().value(percent))
            .child(format!("{:.0}%", percent))
    }

    fn render_status_bar(&self, cx: &Context<Self>) -> impl IntoElement {
        let latest = self.data.back().cloned();
        let share = |pick: fn(&DailyDevice) -> f64| {
            latest
                .as_ref()
                .map(|d| pick(d) as f32 / device_total(d) as f32 * 100.)
                .unwrap_or(0.)
        };
        let date = latest
            .as_ref()
            .map_or_else(|| "No data".into(), |d| d.date.to_string());

        h_flex()
            .px_3()
            .gap_4()
            .h_7()
            .text_sm()
            .items_center()
            .justify_between()
            .border_t_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().tab_bar)
            .text_color(cx.theme().muted_foreground)
            .child(
                h_flex()
                    .gap_4()
                    .child(self.render_gauge(
                        "system-monitor-desktop",
                        MONITOR_SVG,
                        share(|d| d.desktop),
                    ))
                    .child(self.render_gauge(
                        "system-monitor-mobile",
                        SMARTPHONE_SVG,
                        share(|d| d.mobile),
                    ))
                    .child(self.render_gauge(
                        "system-monitor-tablet",
                        TABLET_SVG,
                        share(|d| d.tablet),
                    )),
            )
            .child(div().child(date))
    }
}

/// The day's visitors across every device class.
fn device_total(d: &DailyDevice) -> f64 {
    d.desktop + d.mobile + d.tablet + d.watch
}

impl Render for SystemMonitorSection {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let active_tab_index = self.active_tab as usize;
        let in_view: f64 = self
            .data
            .iter()
            .map(|d| d.desktop + d.mobile + d.tablet + d.watch)
            .sum();

        v_flex()
            .w_full()
            .p_4()
            .gap_4()
            .child(
                h_flex()
                    .w_full()
                    .items_center()
                    .justify_between()
                    .child(
                        TabBar::new("system-monitor-tabs")
                            .segmented()
                            .px_0()
                            .py(rems(0.125))
                            .bg(cx.theme().title_bar)
                            .selected_index(active_tab_index)
                            .on_click(cx.listener(|this, ix: &usize, window, cx| {
                                this.set_active_tab(*ix, window, cx);
                            }))
                            .child(Tab::new().label("Overview"))
                            .child(Tab::new().label("Devices")),
                    )
                    .child(
                        div()
                            .mr_4()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(format!("{} visitors in view", compact(in_view))),
                    ),
            )
            .child(
                div()
                    .id("system-monitor-content")
                    .w_full()
                    .map(|this| match self.active_tab {
                        MonitorTab::Overview => this.child(self.render_overview_tab(cx)),
                        MonitorTab::Devices => this.child(self.render_devices_tab(cx)),
                    }),
            )
            .child(self.render_status_bar(cx))
    }
}

pub fn register(sections: &mut Vec<GallerySection>, window: &mut Window, cx: &mut App) {
    sections.push(GallerySection::new(
        "system-monitor",
        "System Monitor",
        "Rolling visitor-share charts, a sortable devices table, and status gauges on a half-second loop.",
        SystemMonitorSection::view(window, cx),
    ));
}
