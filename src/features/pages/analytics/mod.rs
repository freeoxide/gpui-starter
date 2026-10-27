//! Removable analytics workspace page. `AnalyticsPage` is the only public
//! surface via `views::AnalyticsPage`; everything else stays private to the
//! module. Removal = delete this directory and revert its seam touches: the
//! `mod analytics;`/`pub use` lines in features/pages/mod.rs, the
//! Page::Analytics arms and all() entry in shell/sidebar.rs, the VALID_HOSTS
//! entry in shell/route.rs, the AppRoot wiring in
//! shell/root/app_root/state.rs, the page count in tests/e2e_navigation.rs,
//! and the nav_analytics + analytics_* keys in locales/en.yml and
//! locales/zh-CN.yml.

use std::rc::Rc;
use std::time::Duration;

use gpui_kit::component::{
    ActiveTheme as _, Icon, IconName, IndexPath, Sizable as _, StyledExt as _, WindowExt as _,
    button::{Button, ButtonVariants as _, DropdownButton},
    chart::{AreaChart, BarChart, LineChart},
    empty::{
        Empty, EmptyContent, EmptyDescription, EmptyHeader, EmptyMedia, EmptyMediaVariant,
        EmptyTitle,
    },
    h_flex,
    plot::shape::BarAlignment,
    select::{Select, SelectEvent, SelectState},
    skeleton::Skeleton,
    spinner::Spinner,
    tab::{Tab, TabBar},
    table::{DataTable, TableDelegate as _, TableEvent, TableState},
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};
use serde::Deserialize;

use crate::accessibility::A11yExt as _;
use crate::i18n::{localize, localize_with_args};

use self::model::{ChannelKey, DateRange, Density, Segment, WorkspaceData};
use self::table::ChannelTableDelegate;

pub(crate) mod detail;
pub(crate) mod format;
pub(crate) mod model;
pub(crate) mod table;

use detail::ChannelDetail;

actions!(analytics, [OpenChannelDetails]);

#[derive(Action, Clone, Copy, PartialEq, Eq, Deserialize)]
#[action(namespace = analytics, no_json)]
enum PreviewStateToggle {
    Live,
    Loading,
    Empty,
    Error,
}

impl PreviewStateToggle {
    fn into_state(self) -> PreviewState {
        match self {
            PreviewStateToggle::Live => PreviewState::Live,
            PreviewStateToggle::Loading => PreviewState::Loading,
            PreviewStateToggle::Empty => PreviewState::Empty,
            PreviewStateToggle::Error => PreviewState::Error,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Phase {
    Initial,
    Refreshing,
    Ready,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum PreviewState {
    Live,
    Loading,
    Empty,
    Error,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum RegionView {
    Skeleton,
    Content,
    Empty,
    Error,
}

const RELOAD_DELAY: Duration = Duration::from_millis(350);

pub struct AnalyticsPage {
    data: Rc<WorkspaceData>,
    range: DateRange,
    segment: Segment,
    range_select: Entity<SelectState<Vec<SharedString>>>,
    segment_select: Entity<SelectState<Vec<SharedString>>>,
    density: Density,
    preview: PreviewState,
    phase: Phase,
    committed: bool,
    table: Entity<TableState<ChannelTableDelegate>>,
    selected_key: Option<ChannelKey>,
    _subscriptions: Vec<Subscription>,
    _reload_task: Task<()>,
}

impl AnalyticsPage {
    /// Full analytics workspace view. Parses the fixture JSON once (panics on
    /// malformed fixtures, same contract as the gallery `fixture()` helper),
    /// starts in the Initial skeleton phase, and kicks the first reload.
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let data = Rc::new(WorkspaceData::load());

        let range_items: Vec<SharedString> = DateRange::ALL
            .iter()
            .map(|range| localize(range.label_key()).into())
            .collect();
        let range_select = cx
            .new(|cx| SelectState::new(range_items, Some(IndexPath::default().row(1)), window, cx));

        let segment_items: Vec<SharedString> = Segment::ALL
            .iter()
            .map(|segment| localize(segment.label_key()).into())
            .collect();
        let segment_select = cx.new(|cx| {
            SelectState::new(segment_items, Some(IndexPath::default().row(0)), window, cx)
        });

        let all_rows = data.channels(Segment::All);
        let table = cx.new(|cx| {
            TableState::new(ChannelTableDelegate::new(all_rows), window, cx)
                .col_selectable(false)
                .col_movable(false)
        });

        let subscriptions = vec![
            cx.subscribe(
                &range_select,
                |this, _, _: &SelectEvent<Vec<SharedString>>, cx| {
                    this.on_range_committed(cx);
                },
            ),
            cx.subscribe(
                &segment_select,
                |this, _, _: &SelectEvent<Vec<SharedString>>, cx| {
                    this.on_segment_committed(cx);
                },
            ),
            cx.subscribe_in(&table, window, |this, _, event: &TableEvent, window, cx| {
                this.on_table_event(event, window, cx);
            }),
        ];

        cx.bind_keys([KeyBinding::new(
            "enter",
            OpenChannelDetails,
            Some("DataTable"),
        )]);

        let mut page = Self {
            range: DateRange::Last30,
            segment: Segment::All,
            data,
            range_select,
            segment_select,
            density: Density::Comfortable,
            preview: PreviewState::Live,
            phase: Phase::Initial,
            committed: false,
            table,
            selected_key: None,
            _subscriptions: subscriptions,
            _reload_task: Task::ready(()),
        };
        page.reload(cx);
        page
    }

    fn on_range_committed(&mut self, cx: &mut Context<Self>) {
        let next = self
            .range_select
            .read(cx)
            .selected_index(cx)
            .map(|path| DateRange::from_index(path.row))
            .unwrap_or(self.range);
        if next != self.range {
            self.range = next;
            self.reload(cx);
        }
    }

    fn on_segment_committed(&mut self, cx: &mut Context<Self>) {
        let next = self
            .segment_select
            .read(cx)
            .selected_index(cx)
            .map(|path| Segment::from_index(path.row))
            .unwrap_or(self.segment);
        if next != self.segment {
            self.segment = next;
            self.reload(cx);
        }
    }

    fn reload(&mut self, cx: &mut Context<Self>) {
        self.phase = if !self.committed {
            Phase::Initial
        } else {
            Phase::Refreshing
        };
        self.table
            .update(cx, |table, _| table.delegate_mut().set_loading(true));
        cx.notify();
        self._reload_task = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(RELOAD_DELAY).await;
            let _ = this.update(cx, |this, cx| this.commit_reload(cx));
        });
    }

    fn commit_reload(&mut self, cx: &mut Context<Self>) {
        let rows = self.data.channels(self.segment);
        let key = self.selected_key.clone();
        self.table.update(cx, |table, cx| {
            table.delegate_mut().set_rows(rows);
            table.refresh(cx);
            match key
                .as_ref()
                .and_then(|key| table.delegate().row_ix_for_key(key))
            {
                Some(row_ix) => table.set_selected_row(row_ix, cx),
                None => table.clear_selection(cx),
            }
        });
        if self.table.read(cx).selected_row().is_none() {
            self.selected_key = None;
        }
        self.phase = Phase::Ready;
        self.committed = true;
        cx.notify();
    }

    fn on_table_event(&mut self, event: &TableEvent, window: &mut Window, cx: &mut Context<Self>) {
        match event {
            TableEvent::SelectRow(row_ix) => {
                self.selected_key = self.table.read(cx).delegate().key_at(*row_ix);
            }
            TableEvent::DoubleClickedRow(row_ix) => {
                self.selected_key = self.table.read(cx).delegate().key_at(*row_ix);
                self.open_detail(window, cx);
            }
            // refresh() rebuilds col groups from the delegate's Column specs,
            // so user-fitted widths must live there to survive a reload.
            TableEvent::ColumnWidthsChanged(widths) => {
                self.table
                    .update(cx, |table, _| table.delegate_mut().set_col_widths(widths));
            }
            _ => {}
        }
    }

    fn open_detail(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let fallback = self
            .table
            .read(cx)
            .delegate()
            .menu_row()
            .and_then(|row_ix| self.table.read(cx).delegate().key_at(row_ix));
        let Some(key) = self.selected_key.clone().or(fallback) else {
            return;
        };
        let Some(row) = self.data.channel_row(&key) else {
            return;
        };
        if window.has_active_sheet(cx) {
            return;
        }
        let detail = ChannelDetail::view(row.clone(), self.data.clone(), self.density.size(), cx);
        let title = localize_with_args("analytics_detail_title", &[("page", row.page.as_ref())]);
        window.open_sheet(cx, move |sheet, _, _| {
            sheet
                .size(px(420.))
                .overlay(true)
                .overlay_closable(true)
                .title(title.clone())
                .child(detail.clone())
        });
    }

    fn set_preview(&mut self, toggle: PreviewStateToggle, cx: &mut Context<Self>) {
        self.preview = toggle.into_state();
        cx.notify();
    }

    fn region_view(&self) -> RegionView {
        match self.preview {
            PreviewState::Loading => RegionView::Skeleton,
            PreviewState::Empty => RegionView::Empty,
            PreviewState::Error => RegionView::Error,
            PreviewState::Live => match self.phase {
                Phase::Initial => RegionView::Skeleton,
                Phase::Refreshing | Phase::Ready => RegionView::Content,
            },
        }
    }

    fn set_density(&mut self, density: Density, cx: &mut Context<Self>) {
        self.density = density;
        self.table.update(cx, |table, cx| {
            table.delegate_mut().set_size(density.size());
            cx.notify();
        });
        cx.notify();
    }

    fn render_state(&self, empty: bool, cx: &mut Context<Self>) -> AnyElement {
        if empty {
            return Empty::new()
                .header(
                    EmptyHeader::new()
                        .media(
                            EmptyMedia::new()
                                .with_variant(EmptyMediaVariant::Icon)
                                .child(Icon::new(IconName::Inbox)),
                        )
                        .title(EmptyTitle::new().child(localize("analytics_empty_title")))
                        .description(
                            EmptyDescription::new().child(localize("analytics_empty_description")),
                        ),
                )
                .into_any_element();
        }
        Empty::new()
            .header(
                EmptyHeader::new()
                    .media(
                        EmptyMedia::new()
                            .with_variant(EmptyMediaVariant::Icon)
                            .child(Icon::new(IconName::TriangleAlert)),
                    )
                    .title(EmptyTitle::new().child(localize("analytics_error_title")))
                    .description(
                        EmptyDescription::new().child(localize("analytics_error_description")),
                    ),
            )
            .content(
                EmptyContent::new().child(
                    Button::new("analytics-retry")
                        .outline()
                        .icon(IconName::RotateCw)
                        .label(localize("analytics_retry"))
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.preview = PreviewStateToggle::Live.into_state();
                            this.reload(cx);
                        })),
                ),
            )
            .into_any_element()
    }

    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let size = self.density.size();
        let preview = self.preview;
        h_flex()
            .flex_wrap()
            .items_center()
            .gap_3()
            .child(
                Select::new(&self.range_select)
                    .title_prefix(localize("analytics_filter_range"))
                    .w(rems(15.))
                    .with_size(size),
            )
            .child(
                Select::new(&self.segment_select)
                    .title_prefix(localize("analytics_filter_segment"))
                    .w(rems(15.))
                    .with_size(size),
            )
            .child(
                Button::new("analytics-refresh")
                    .outline()
                    .icon(IconName::RotateCw)
                    .label(localize("analytics_refresh"))
                    .on_click(cx.listener(|this, _, _, cx| this.reload(cx))),
            )
            .child(
                div()
                    .id("analytics-density-group")
                    .a11y(Role::Group, localize("analytics_density"))
                    .child(
                        TabBar::new("analytics-density")
                            .segmented()
                            .selected_index(self.density as usize)
                            .on_click(cx.listener(|this, ix: &usize, _, cx| {
                                this.set_density(
                                    if *ix == 0 {
                                        Density::Compact
                                    } else {
                                        Density::Comfortable
                                    },
                                    cx,
                                );
                            }))
                            .child(Tab::new().label(localize("analytics_density_compact")))
                            .child(Tab::new().label(localize("analytics_density_comfortable"))),
                    ),
            )
            .child(
                DropdownButton::new("analytics-preview")
                    .button(
                        Button::new("analytics-preview-trigger")
                            .ghost()
                            .label(localize("analytics_preview_label")),
                    )
                    .dropdown_menu(move |menu, _, _| {
                        menu.menu_with_check(
                            localize("analytics_preview_live"),
                            preview == PreviewState::Live,
                            Box::new(PreviewStateToggle::Live),
                        )
                        .menu_with_check(
                            localize("analytics_preview_loading"),
                            preview == PreviewState::Loading,
                            Box::new(PreviewStateToggle::Loading),
                        )
                        .menu_with_check(
                            localize("analytics_preview_empty"),
                            preview == PreviewState::Empty,
                            Box::new(PreviewStateToggle::Empty),
                        )
                        .menu_with_check(
                            localize("analytics_preview_error"),
                            preview == PreviewState::Error,
                            Box::new(PreviewStateToggle::Error),
                        )
                    }),
            )
    }

    fn render_section_heading(&self, key: &'static str) -> Stateful<Div> {
        let label = localize(key);
        let id = SharedString::from(format!("analytics-section-{key}"));
        div()
            .id(id)
            .a11y(Role::Heading, label.clone())
            .aria_level(2)
            .text_lg()
            .font_weight(FontWeight::SEMIBOLD)
            .child(label)
    }

    fn render_kpis(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = self.region_view();
        let compact = matches!(self.density, Density::Compact);
        let refreshing = self.phase == Phase::Refreshing;
        v_flex()
            .gap_3()
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(self.render_section_heading("analytics_section_overview"))
                    .when(refreshing, |this| this.child(Spinner::new().xsmall())),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(localize("analytics_note_fixture")),
            )
            .child(h_flex().flex_wrap().gap_3().map(|row| {
                match view {
                    RegionView::Skeleton => {
                        row.children((0..4).map(|_| self.render_kpi_skeleton(cx)))
                    }
                    RegionView::Empty | RegionView::Error => {
                        row.child(self.render_state(view == RegionView::Empty, cx))
                    }
                    RegionView::Content => row.children(
                        self.data
                            .kpis(self.range)
                            .into_iter()
                            .map(|kpi| self.render_kpi_card(kpi, compact, cx)),
                    ),
                }
            }))
    }

    fn render_kpi_card(
        &self,
        kpi: model::Kpi,
        compact: bool,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let delta_row = match kpi.delta {
            Some(delta) => {
                let up = delta >= 0.;
                let color = if up {
                    cx.theme().success
                } else {
                    cx.theme().danger
                };
                h_flex()
                    .gap_1()
                    .items_center()
                    .child(
                        Icon::new(if up {
                            IconName::ArrowUp
                        } else {
                            IconName::ArrowDown
                        })
                        .size_4()
                        .text_color(color),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(color)
                            .child(format!("{delta:+.1}%")),
                    )
                    .child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .child(localize("analytics_vs_previous")),
                    )
                    .into_any_element()
            }
            None => div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child("-")
                .into_any_element(),
        };
        v_flex()
            .flex_1()
            .min_w(rems(14.))
            .gap_1()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .map(|card| if compact { card.p_3() } else { card.p_4() })
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(localize(kpi.label_key).to_uppercase()),
            )
            .child(div().text_xl().font_semibold().child(kpi.value))
            .child(delta_row)
    }

    fn render_kpi_skeleton(&self, cx: &Context<Self>) -> Div {
        v_flex()
            .flex_1()
            .min_w(rems(14.))
            .gap_2()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .map(|card| {
                if matches!(self.density, Density::Compact) {
                    card.p_3()
                } else {
                    card.p_4()
                }
            })
            .child(Skeleton::new().w_2_3().h_4().rounded(cx.theme().radius))
            .child(Skeleton::new().w_1_3().h_6().rounded(cx.theme().radius))
    }

    fn render_chart_card(
        &self,
        id: &'static str,
        label: String,
        current: (String, Hsla),
        chart: AnyElement,
        full_width: bool,
        min_height_rems: f32,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let compact = matches!(self.density, Density::Compact);
        v_flex()
            .map(|card| {
                if full_width {
                    card.w_full()
                } else {
                    card.flex_1().min_w(rems(24.))
                }
            })
            .min_w_0()
            .gap_2()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .map(|card| if compact { card.p_3() } else { card.p_4() })
            .child(
                h_flex()
                    .justify_between()
                    .items_center()
                    .child(div().text_sm().font_medium().child(label))
                    .child(
                        div()
                            .id(SharedString::from(format!("analytics-chart-value-{id}")))
                            .text_sm()
                            .text_color(current.1)
                            .child(current.0),
                    ),
            )
            .child(div().flex_1().min_h(rems(min_height_rems)).child(chart))
    }

    fn render_chart_skeleton(&self, cx: &Context<Self>) -> Div {
        v_flex()
            .flex_1()
            .min_w(rems(24.))
            .min_w_0()
            .gap_2()
            .border_1()
            .border_color(cx.theme().border)
            .rounded(cx.theme().radius)
            .map(|card| {
                if matches!(self.density, Density::Compact) {
                    card.p_3()
                } else {
                    card.p_4()
                }
            })
            .child(
                Skeleton::new()
                    .w_full()
                    .h(rems(10.))
                    .rounded(cx.theme().radius),
            )
            .child(Skeleton::new().w_full().h_4().rounded(cx.theme().radius))
            .child(Skeleton::new().w_2_3().h_4().rounded(cx.theme().radius))
    }

    fn render_charts(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = self.region_view();
        let window_rows: Vec<model::DailyRow> = self.data.window(self.range).to_vec();
        let min_height = if matches!(self.density, Density::Compact) {
            10_f32
        } else {
            12_f32
        };
        h_flex().flex_wrap().gap_3().map(|row| match view {
            RegionView::Skeleton => row
                .child(self.render_chart_skeleton(cx))
                .child(self.render_chart_skeleton(cx))
                .child(self.render_chart_skeleton(cx).flex_none().w_full()),
            RegionView::Empty | RegionView::Error => {
                row.child(self.render_state(view == RegionView::Empty, cx))
            }
            RegionView::Content => {
                let theme = cx.theme();
                let visitors_chart = AreaChart::new(window_rows.clone())
                    .x(|row: &model::DailyRow| row.date.clone())
                    .y(|row| row.organic)
                    .stroke(theme.chart_1)
                    .fill(format::area_gradient(theme.chart_1))
                    .name(localize("analytics_segment_organic"))
                    .y(|row| row.paid)
                    .stroke(theme.chart_2)
                    .fill(format::area_gradient(theme.chart_2))
                    .name(localize("analytics_segment_paid"))
                    .y(|row| row.social)
                    .stroke(theme.chart_3)
                    .fill(format::area_gradient(theme.chart_3))
                    .name(localize("analytics_segment_social"))
                    .y(|row| row.direct)
                    .stroke(theme.chart_4)
                    .fill(format::area_gradient(theme.chart_4))
                    .name(localize("analytics_segment_direct"))
                    .y_axis(true)
                    .y_tick_count(3)
                    .y_tick_format(format::compact)
                    .x_tick_count(4)
                    .grid(true)
                    .id("analytics-visitors");

                let revenue_chart = LineChart::new(window_rows.clone())
                    .x(|row: &model::DailyRow| row.date.clone())
                    .y(|row| row.revenue)
                    .stroke(theme.chart_5)
                    .name(localize("analytics_chart_revenue"))
                    .y_axis(true)
                    .y_tick_count(3)
                    .y_tick_format(format::money)
                    .x_tick_count(4)
                    .id("analytics-revenue");

                let mut top_rows = self.data.channels(self.segment);
                top_rows.sort_by(|a, b| b.sessions.total_cmp(&a.sessions));
                top_rows.truncate(6);
                let sessions_total: f64 = top_rows.iter().map(|row| row.sessions).sum();
                let sessions_fill = theme.chart_2;
                let sessions_chart = BarChart::new(top_rows)
                    .band(|row| row.page.clone())
                    .value(|row| row.sessions)
                    .name(localize("analytics_chart_sessions"))
                    .fill(move |_, _, _, _| sessions_fill)
                    .alignment(BarAlignment::Left)
                    .value_axis(true)
                    .value_tick_count(3)
                    .value_tick_format(format::compact)
                    .id("analytics-sessions");

                let last_day = window_rows.last();
                let visitors_now = last_day
                    .map(|row| format::compact(row.organic + row.paid + row.social + row.direct))
                    .unwrap_or_default();
                let revenue_now = last_day
                    .map(|row| format::money(row.revenue))
                    .unwrap_or_default();

                row.child(self.render_chart_card(
                    "visitors",
                    localize("analytics_chart_visitors"),
                    (visitors_now, theme.chart_1),
                    visitors_chart.into_any_element(),
                    false,
                    min_height,
                    cx,
                ))
                .child(self.render_chart_card(
                    "revenue",
                    localize("analytics_chart_revenue"),
                    (revenue_now, theme.chart_5),
                    revenue_chart.into_any_element(),
                    false,
                    min_height,
                    cx,
                ))
                .child(self.render_chart_card(
                    "sessions",
                    localize("analytics_chart_sessions"),
                    (format::compact(sessions_total), theme.chart_2),
                    sessions_chart.into_any_element(),
                    true,
                    min_height,
                    cx,
                ))
            }
        })
    }

    fn render_table(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = self.region_view();
        let size = self.density.size();
        let rows = self.table.read(cx).delegate().rows_count(cx);
        let segment_label = localize(self.segment.label_key());
        let range_label = localize(self.range.label_key());
        let summary = localize_with_args(
            "analytics_rows_summary",
            &[
                ("count", &rows.to_string()),
                ("segment", &segment_label),
                ("range", &range_label),
            ],
        );
        let selected_summary = self.selected_key.as_ref().map(|key| {
            localize_with_args(
                "analytics_selected_summary",
                &[
                    ("page", key.page.as_ref()),
                    ("channel", key.channel.as_ref()),
                ],
            )
        });
        let refreshing = self.phase == Phase::Refreshing;

        v_flex()
            .gap_3()
            .child(self.render_section_heading("analytics_section_channels"))
            .child(
                h_flex()
                    .flex_wrap()
                    .justify_between()
                    .items_center()
                    .gap_2()
                    .child(
                        h_flex()
                            .flex_wrap()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .id("analytics-rows-summary")
                                    .a11y(Role::Status, summary.clone())
                                    .a11y_live(accesskit::Live::Polite)
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child(summary),
                            )
                            .when_some(selected_summary, |this, chip| {
                                this.child(
                                    div()
                                        .id("analytics-selected-summary")
                                        .text_xs()
                                        .text_color(cx.theme().foreground)
                                        .child(chip),
                                )
                            }),
                    )
                    .when(refreshing, |this| this.child(Spinner::new().xsmall())),
            )
            .map(|section| match view {
                RegionView::Skeleton => {
                    section.child(
                        div()
                            .border_1()
                            .border_color(cx.theme().border)
                            .rounded(cx.theme().radius)
                            .p_3()
                            .child(v_flex().gap_2().children((0..6).map(|_| {
                                Skeleton::new().h_6().w_full().rounded(cx.theme().radius)
                            }))),
                    )
                }
                RegionView::Empty | RegionView::Error => {
                    section.child(self.render_state(view == RegionView::Empty, cx))
                }
                RegionView::Content => section.child(
                    div().h(rems(30.)).w_full().child(
                        DataTable::new(&self.table)
                            .bordered(true)
                            .stripe(true)
                            .with_size(size),
                    ),
                ),
            })
    }
}

impl Render for AnalyticsPage {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .min_h_full()
            .p_6()
            .gap_4()
            .on_action(cx.listener(|this, _: &OpenChannelDetails, window, cx| {
                this.open_detail(window, cx);
            }))
            .on_action(cx.listener(|this, action: &PreviewStateToggle, _, cx| {
                this.set_preview(*action, cx);
            }))
            .child(self.render_toolbar(cx))
            .child(self.render_kpis(cx))
            .child(self.render_charts(cx))
            .child(self.render_table(cx))
    }
}

#[cfg(test)]
#[path = "mod.test.rs"]
mod mod_test;
