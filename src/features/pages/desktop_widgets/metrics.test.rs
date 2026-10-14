use gpui_kit::{AppContext as _, TestAppContext};

use super::{GIB, MetricsWidget, memory_footer};
use crate::features::menubar_charts::metrics::Sample;

#[test]
fn memory_footer_formats_used_over_total_gib() {
    let sample = Sample {
        seq: 1,
        cpu_percent: 0.0,
        mem_used_bytes: GIB as u64,
        mem_total_bytes: 4 * GIB as u64,
    };
    assert!(memory_footer(&sample).contains("1.0 / 4 GiB"));
}

#[gpui_kit::test]
async fn the_widget_entity_is_constructible_headless(cx: &mut TestAppContext) {
    let widget: gpui_kit::Entity<MetricsWidget> = cx.new(MetricsWidget::new_for_test);
    assert!(cx.read(|cx| widget.read(cx).metrics.read(cx).samples().is_empty()));
}
