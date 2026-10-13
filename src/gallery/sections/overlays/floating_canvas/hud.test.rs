use std::time::Duration;

use gpui_kit::prelude::*;
use gpui_kit::{Bounds, TestAppContext, point, px, size};

use crate::ui::canvas::CanvasShape;

use super::{HUD_SAMPLE_CAP, HudMetrics, HudPanel, sample_value, sparkline_points};

#[gpui_kit::test]
fn tick_grows_samples_and_caps_the_ring(cx: &mut TestAppContext) {
    let metrics = cx.new(HudMetrics::new);
    metrics.update(cx, |metrics, cx| {
        for _ in 0..200 {
            metrics.tick(cx);
        }
    });
    assert_eq!(
        cx.read(|cx| metrics.read(cx).samples().len()),
        HUD_SAMPLE_CAP
    );
    assert_eq!(
        cx.read(|cx| metrics.read(cx).samples()[0]),
        sample_value(81)
    );
    assert_eq!(cx.read(|cx| metrics.read(cx).tick_count()), 200);
}

#[gpui_kit::test]
fn sample_value_stays_in_range_and_differs_across_ticks() {
    for tick in 0..1000u64 {
        let value = sample_value(tick);
        assert!(
            (0.08..=0.92).contains(&value),
            "tick {tick} produced {value}"
        );
    }
    let values: Vec<f32> = (0..10u64).map(sample_value).collect();
    for (index, value) in values.iter().enumerate() {
        assert!(
            !values[index + 1..].contains(value),
            "tick {index} duplicates a later sample"
        );
    }
}

#[gpui_kit::test]
async fn producer_ticks_samples_on_the_fake_clock(cx: &mut TestAppContext) {
    let metrics = cx.new(HudMetrics::new);
    cx.executor().advance_clock(Duration::from_millis(250 * 8));
    cx.run_until_parked();
    assert_eq!(cx.read(|cx| metrics.read(cx).samples().len()), 8);
}

#[gpui_kit::test]
fn sparkline_points_span_the_bounds() {
    let bounds = Bounds {
        origin: point(px(10.), px(20.)),
        size: size(px(100.), px(50.)),
    };
    assert!(sparkline_points(&[], bounds).is_empty());

    let samples = [0.1f32, 0.9, 0.5];
    let points = sparkline_points(&samples, bounds);
    assert_eq!(points.len(), samples.len());
    assert_eq!(points[0].x, bounds.origin.x);
    assert_eq!(points[0].y, bounds.origin.y + px(45.));
    assert_eq!(
        points.last().unwrap().x,
        bounds.origin.x + bounds.size.width
    );
    for point in &points {
        assert!(point.y >= bounds.origin.y);
        assert!(point.y <= bounds.origin.y + bounds.size.height);
    }
    assert!(points.windows(2).all(|pair| pair[0].x <= pair[1].x));
}

#[gpui_kit::test]
fn hud_panel_refresh_keeps_exactly_one_path(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let metrics = cx.new(HudMetrics::new);
    metrics.update(cx, |metrics, cx| {
        for _ in 0..8 {
            metrics.tick(cx);
        }
    });
    let panel = cx.new(|cx| HudPanel::new(metrics.clone(), cx));
    panel.update(cx, |panel, cx| panel.refresh(cx));
    panel.update(cx, |panel, cx| panel.refresh(cx));
    let canvas = cx.read(|cx| panel.read(cx).canvas.clone());
    cx.update(|cx| {
        let doc = canvas.read(cx).document();
        assert_eq!(doc.shapes().count(), 1);
        let CanvasShape::Path { points, .. } = doc.shapes().next().unwrap().2 else {
            panic!("expected the sparkline to be a path");
        };
        assert_eq!(points.len(), 8);
    });
}
