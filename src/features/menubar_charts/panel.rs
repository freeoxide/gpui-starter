#[cfg(test)]
use std::sync::atomic::{AtomicU64, Ordering};

use gpui_kit::component::{
    ActiveTheme as _,
    chart::{AreaChart, BarChart, LineChart},
    h_flex,
    switch::Switch,
    v_flex,
};
use gpui_kit::{prelude::FluentBuilder as _, *};

use super::metrics::{MetricsState, Sample};

#[cfg(test)]
static PANELS_BUILT: AtomicU64 = AtomicU64::new(0);

const GIB: f64 = 1024.0 * 1024.0 * 1024.0;

pub(crate) struct ChartsPanel {
    metrics: Entity<MetricsState>,
}

impl ChartsPanel {
    pub(crate) fn view(_window: &mut Window, cx: &mut App) -> Entity<Self> {
        #[cfg(test)]
        PANELS_BUILT.fetch_add(1, Ordering::SeqCst);
        let metrics = cx.new(|_| MetricsState::new());
        MetricsState::start(&metrics, cx);
        cx.new(|cx| {
            cx.observe(&metrics, |_, _, cx| cx.notify()).detach();
            Self { metrics }
        })
    }

    /// Panels built since process start. Sheets expose no readable identity,
    /// so tests count builds to tell a no-op open from a replacement.
    #[cfg(test)]
    pub(crate) fn panels_built() -> u64 {
        PANELS_BUILT.load(Ordering::SeqCst)
    }

    #[cfg(test)]
    pub(crate) fn metrics(&self) -> &Entity<MetricsState> {
        &self.metrics
    }
}

impl Render for ChartsPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let state = self.metrics.read(cx);
        let samples = state.samples();
        let paused = state.is_paused();
        let last = samples.back().copied();
        let memory_bars = samples
            .iter()
            .skip(samples.len().saturating_sub(30))
            .copied();
        let muted = cx.theme().muted_foreground;
        let spark_color = cx.theme().chart_2;
        let line_color = cx.theme().chart_4;
        let bar_color = cx.theme().chart_2;

        v_flex()
            .size_full()
            .p_3()
            .gap_3()
            .child(
                v_flex()
                    .gap_1()
                    .child(
                        h_flex()
                            .justify_between()
                            .child(
                                div()
                                    .text_sm()
                                    .text_color(muted)
                                    .child(crate::i18n::localize("menubar_charts_cpu")),
                            )
                            .child(
                                div().text_sm().text_color(cx.theme().foreground).child(
                                    last.map(|s| format!("{:.0}%", s.cpu_percent))
                                        .unwrap_or_else(|| "-".to_string()),
                                ),
                            ),
                    )
                    .child(
                        div().h_16().child(
                            AreaChart::new(samples.iter().copied())
                                .x(|s: &Sample| s.seq.to_string())
                                .y(|s: &Sample| s.cpu_percent as f64)
                                .stroke(spark_color)
                                .fill(linear_gradient(
                                    0.,
                                    linear_color_stop(spark_color.opacity(0.4), 1.),
                                    linear_color_stop(cx.theme().background.opacity(0.1), 0.),
                                ))
                                .x_axis(false)
                                .y_axis(false)
                                .grid(false)
                                .interactive(false)
                                .y_domain(0., 100.)
                                .id("menubar-charts-spark"),
                        ),
                    ),
            )
            .child(
                div().min_h(rems(10.)).flex_1().child(
                    LineChart::new(samples.iter().copied())
                        .x(|s: &Sample| s.seq.to_string())
                        .y(|s: &Sample| s.cpu_percent as f64)
                        .y_domain(0., 100.)
                        .y_axis(true)
                        .y_tick_count(3)
                        .y_tick_format(|v| format!("{v:.0}%"))
                        .x_axis(false)
                        .grid(true)
                        .stroke(line_color)
                        .id("menubar-charts-line"),
                ),
            )
            .child(
                div().flex_1().child(
                    BarChart::new(memory_bars)
                        .band(|s: &Sample| s.seq.to_string())
                        .value(|s: &Sample| s.mem_used_bytes as f64 / GIB)
                        .name(crate::i18n::localize("menubar_charts_memory"))
                        .fill(move |_, _, _, _| bar_color)
                        .value_axis(true)
                        .value_tick_count(3)
                        .value_tick_format(|v| format!("{v:.0} GiB"))
                        .id("menubar-charts-bar"),
                ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .justify_between()
                    .text_sm()
                    .child(
                        h_flex()
                            .gap_1()
                            .child(
                                div()
                                    .text_color(muted)
                                    .child(crate::i18n::localize("menubar_charts_memory_used")),
                            )
                            .child(
                                div().text_color(muted).child(
                                    last.map(|s| {
                                        format!(
                                            "{:.1} / {:.0} GiB",
                                            s.mem_used_bytes as f64 / GIB,
                                            s.mem_total_bytes as f64 / GIB
                                        )
                                    })
                                    .unwrap_or_else(|| "-".to_string()),
                                ),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(
                                Switch::new("menubar-charts-pause")
                                    .checked(paused)
                                    .accessibility_label(crate::i18n::localize(
                                        "menubar_charts_pause",
                                    ))
                                    .on_change(cx.listener(|this, checked: &bool, _, cx| {
                                        this.metrics.update(cx, |m, cx| m.set_paused(*checked, cx));
                                    })),
                            )
                            .when(paused, |row| {
                                row.child(
                                    div()
                                        .text_color(muted)
                                        .child(crate::i18n::localize("menubar_charts_paused")),
                                )
                            }),
                    ),
            )
    }
}

#[cfg(test)]
#[path = "panel.test.rs"]
mod panel_test;
