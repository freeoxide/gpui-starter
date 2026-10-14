use gpui_kit::component::{ActiveTheme as _, Icon, IconName, chart::AreaChart, h_flex, v_flex};
use gpui_kit::{prelude::*, *};

use crate::features::menubar_charts::metrics::{MetricsState, Sample};

const GIB: f64 = 1024.0 * 1024.0 * 1024.0;

pub(crate) fn memory_footer(last: &Sample) -> SharedString {
    format!(
        "{:.1} / {:.0} GiB",
        last.mem_used_bytes as f64 / GIB,
        last.mem_total_bytes as f64 / GIB
    )
    .into()
}

pub(crate) struct MetricsWidget {
    metrics: Entity<MetricsState>,
}

impl MetricsWidget {
    pub(crate) fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let metrics = cx.new(|_| MetricsState::new());
        MetricsState::start(&metrics, cx);
        cx.observe(&metrics, |_, _, cx| cx.notify()).detach();
        Self { metrics }
    }

    #[cfg(test)]
    pub(crate) fn new_for_test(cx: &mut Context<Self>) -> Self {
        let metrics = cx.new(|_| MetricsState::new());
        cx.observe(&metrics, |_, _, cx| cx.notify()).detach();
        Self { metrics }
    }

    pub(crate) fn view(window: &mut Window, cx: &mut App) -> Entity<MetricsWidget> {
        cx.new(|cx| Self::new(window, cx))
    }
}

impl Render for MetricsWidget {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let samples: Vec<Sample> = self.metrics.read(cx).samples().iter().cloned().collect();
        let last = samples.last().copied();
        let muted = cx.theme().muted_foreground;
        let spark_color = cx.theme().chart_2;
        let cpu_now = last
            .map(|s| format!("{:.0}%", s.cpu_percent))
            .unwrap_or_else(|| "-".to_string());

        v_flex()
            .size_full()
            .p_3()
            .gap_2()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .size_4()
                            .flex_shrink_0()
                            .child(Icon::new(IconName::Cpu)),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(muted)
                            .child(crate::i18n::localize("widgets_cpu")),
                    )
                    .child(div().text_sm().child(cpu_now)),
            )
            .child(
                div().h_16().child(
                    AreaChart::new(samples)
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
                        .id("widgets-metrics-spark"),
                ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        div()
                            .size_4()
                            .flex_shrink_0()
                            .child(Icon::new(IconName::MemoryStick)),
                    )
                    .child(
                        div()
                            .text_sm()
                            .text_color(muted)
                            .child(crate::i18n::localize("widgets_memory")),
                    )
                    .child(match last {
                        Some(last) => div().text_sm().child(memory_footer(&last)),
                        None => div()
                            .text_sm()
                            .child(crate::i18n::localize("widgets_no_data")),
                    }),
            )
    }
}

#[cfg(test)]
#[path = "metrics.test.rs"]
mod metrics_test;
