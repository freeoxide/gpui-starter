use gpui_kit::TestAppContext;

// Explicit imports only: `use super::*` would pull in the parent's gpui_kit
// glob, whose `test` macro shadows the #[test] the kit macro expands into.
use super::{
    ColumnSort, DailyDevice, DeviceSortField, DeviceTableDelegate, INTERVAL, SystemMonitorSection,
    fixture,
};

#[gpui_kit::test]
fn date_sort_restores_chronological_fixture_order() {
    let devices: Vec<DailyDevice> = fixture(include_str!("fixtures/daily-devices.json"));
    let mut delegate = DeviceTableDelegate::new(devices.clone());
    delegate.sort_field = DeviceSortField::Date;
    delegate.sort_order = ColumnSort::Ascending;
    delegate.sort_rows();

    let sorted: Vec<String> = delegate
        .rows
        .iter()
        .map(|(_, d)| d.date.to_string())
        .collect();
    assert_eq!(
        sorted,
        devices
            .iter()
            .map(|d| d.date.to_string())
            .collect::<Vec<_>>(),
        "ascending Date must restore the fixture's chronological order"
    );
}

#[gpui_kit::test]
fn tick_loop_sleeps_without_a_redraw_and_resumes_on_the_next_one(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let (section, cx) = cx.add_window_view(|window, cx| SystemMonitorSection::new(window, cx));
    cx.run_until_parked();

    let (head_start, device_count) =
        cx.update(|_, cx| section.update(cx, |this, _| (this.head, this.devices.len())));

    // A redrawn section keeps ticking.
    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.executor().advance_clock(INTERVAL);
    cx.run_until_parked();
    cx.update(|_, cx| {
        section.update(cx, |this, _| {
            assert_eq!(this.head, (head_start + 1) % device_count);
        })
    });

    // A tick that lands with no redraw since the last one ends the loop
    // instead of advancing.
    cx.executor().advance_clock(INTERVAL);
    cx.run_until_parked();
    cx.update(|_, cx| {
        section.update(cx, |this, _| {
            assert!(!this.ticking, "a tick without a redraw ends the loop");
            assert_eq!(this.head, (head_start + 1) % device_count);
        })
    });

    cx.executor().advance_clock(INTERVAL * 8);
    cx.run_until_parked();
    cx.update(|_, cx| {
        section.update(cx, |this, _| {
            assert_eq!(
                this.head,
                (head_start + 1) % device_count,
                "no ticks fire while the loop is down"
            );
        })
    });

    cx.update(|window, cx| window.draw(cx).clear(cx));
    cx.executor().advance_clock(INTERVAL);
    cx.run_until_parked();
    cx.update(|_, cx| {
        section.update(cx, |this, _| {
            assert!(this.ticking, "a redraw restarts the loop");
            assert_eq!(this.head, (head_start + 2) % device_count);
        })
    });
}
