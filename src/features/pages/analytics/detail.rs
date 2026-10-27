use std::rc::Rc;
use std::time::Duration;

use gpui_kit::component::{
    ActiveTheme as _, Sizable as _, Size,
    chart::LineChart,
    description_list::{DescriptionItem, DescriptionList},
    h_flex,
    skeleton::Skeleton,
    v_flex,
};
use gpui_kit::{Axis, Task, *};

use crate::i18n::localize;

use super::format;
use super::model::{ChannelRow, WorkspaceData};

const DETAIL_DELAY: Duration = Duration::from_millis(300);

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum DetailPhase {
    Loading,
    Ready,
}

pub(crate) struct ChannelDetail {
    row: ChannelRow,
    series: Vec<(SharedString, f64)>,
    phase: DetailPhase,
    size: Size,
    _task: Task<()>,
}

impl ChannelDetail {
    pub(crate) fn view(
        row: ChannelRow,
        data: Rc<WorkspaceData>,
        size: Size,
        cx: &mut App,
    ) -> Entity<Self> {
        let series = data.channel_series(&row.channel);
        let detail = cx.new(|_| Self {
            series,
            phase: DetailPhase::Loading,
            size,
            row,
            _task: Task::ready(()),
        });
        let task = detail.update(cx, |_, cx| {
            cx.spawn(async move |this, cx| {
                cx.background_executor().timer(DETAIL_DELAY).await;
                let _ = this.update(cx, |this, cx| this.finish_loading(cx));
            })
        });
        detail.update(cx, |detail, _| detail._task = task);
        detail
    }

    fn finish_loading(&mut self, cx: &mut Context<Self>) {
        self.phase = DetailPhase::Ready;
        cx.notify();
    }

    fn item(label_key: &'static str, value: String) -> DescriptionItem {
        DescriptionItem::new(localize(label_key))
            .value(div().child(value).into_any_element())
            .span(1)
    }

    fn render_loading(&self, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_3()
            .p_4()
            .child(v_flex().gap_2().children((0..6).map(|_| {
                h_flex()
                    .gap_3()
                    .child(Skeleton::new().w_1_3().h_4().rounded(cx.theme().radius))
                    .child(Skeleton::new().w_1_2().h_4().rounded(cx.theme().radius))
            })))
            .child(Skeleton::new().h_12().w_full().rounded(cx.theme().radius))
            .child(
                div()
                    .text_xs()
                    .text_color(cx.theme().muted_foreground)
                    .child(localize("analytics_loading")),
            )
    }

    fn render_ready(&self, cx: &Context<Self>) -> impl IntoElement {
        let secs = self.row.avg_session_secs as usize;
        let sparkline = LineChart::new(self.series.clone())
            .x(|point: &(SharedString, f64)| point.0.clone())
            .y(|point| point.1)
            .stroke(cx.theme().chart_2)
            .x_axis(false)
            .y_axis(false)
            .grid(false)
            .interactive(false)
            .id("analytics-detail-spark");
        v_flex().p_4().gap_3().child(
            DescriptionList::new()
                .layout(Axis::Horizontal)
                .bordered(true)
                .columns(2)
                .with_size(self.size)
                .child(Self::item(
                    "analytics_detail_page",
                    self.row.page.to_string(),
                ))
                .child(Self::item(
                    "analytics_detail_channel",
                    self.row.channel.to_string(),
                ))
                .child(Self::item(
                    "analytics_detail_visitors",
                    format::compact(self.row.visitors),
                ))
                .child(Self::item(
                    "analytics_detail_sessions",
                    format::compact(self.row.sessions),
                ))
                .child(Self::item(
                    "analytics_detail_revenue",
                    format::money(self.row.revenue),
                ))
                .child(Self::item(
                    "analytics_detail_conversion",
                    format::percent(self.row.conversion, 1),
                ))
                .child(Self::item(
                    "analytics_detail_avg_session",
                    format!("{}m {}s", secs / 60, secs % 60),
                ))
                .child(DescriptionItem::Separator)
                .child(
                    DescriptionItem::new(localize("analytics_detail_trend"))
                        .value(div().h_16().w_full().child(sparkline).into_any_element())
                        .span(2),
                ),
        )
    }
}

impl Render for ChannelDetail {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        match self.phase {
            DetailPhase::Loading => self.render_loading(cx).into_any_element(),
            DetailPhase::Ready => self.render_ready(cx).into_any_element(),
        }
    }
}

#[cfg(test)]
#[path = "detail.test.rs"]
mod detail_test;
