use gpui_kit::{
    Context, Entity, IntoElement, ParentElement, Render, Styled, TestAppContext, VisualTestContext,
    Window, div,
};

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

/// Mounts the section the way the gallery does, so the test can take it off
/// screen: the test harness redraws every notified window, so only an
/// unrendered section can see a tick land without a redraw.
struct Host {
    section: Entity<SystemMonitorSection>,
    show: bool,
}

impl Render for Host {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let host = div().size_full();
        if self.show {
            host.child(self.section.clone())
        } else {
            host
        }
    }
}

fn tick(cx: &mut VisualTestContext) {
    cx.executor().advance_clock(INTERVAL);
    cx.run_until_parked();
}

fn set_shown(cx: &mut VisualTestContext, host: &Entity<Host>, show: bool) {
    cx.update(|_, cx| {
        host.update(cx, |host, cx| {
            host.show = show;
            cx.notify();
        })
    });
}

#[gpui_kit::test]
fn tick_loop_sleeps_without_a_redraw_and_resumes_on_the_next_one(cx: &mut TestAppContext) {
    cx.update(gpui_kit::component::theme::init);
    let (host, cx) = cx.add_window_view(|window, cx| Host {
        section: SystemMonitorSection::view(window, cx),
        show: true,
    });
    cx.run_until_parked();
    let section = cx.update(|_, cx| host.read(cx).section.clone());

    let (head_start, device_count) =
        cx.update(|_, cx| section.update(cx, |this, _| (this.head, this.devices.len())));
    let head_after = |ticks: usize| (head_start + ticks) % device_count;

    // On screen, each tick's notify redraws the section, which re-arms it.
    cx.update(|window, cx| window.draw(cx).clear(cx));
    tick(cx);
    cx.update(|_, cx| {
        section.update(cx, |this, _| {
            assert!(this.ticking);
            assert_eq!(this.head, head_after(1));
        })
    });

    // Off screen, the tick its last render armed still advances; the next
    // one lands without a redraw and ends the loop.
    set_shown(cx, &host, false);
    tick(cx);
    cx.update(|_, cx| {
        section.update(cx, |this, _| {
            assert!(this.ticking, "the already-armed tick still runs");
            assert_eq!(this.head, head_after(2));
        })
    });
    tick(cx);
    cx.update(|_, cx| {
        section.update(cx, |this, _| {
            assert!(!this.ticking, "a tick without a redraw ends the loop");
            assert_eq!(this.head, head_after(2));
        })
    });

    cx.executor().advance_clock(INTERVAL * 8);
    cx.run_until_parked();
    cx.update(|_, cx| {
        section.update(cx, |this, _| {
            assert_eq!(
                this.head,
                head_after(2),
                "no ticks fire while the loop is down"
            );
        })
    });

    set_shown(cx, &host, true);
    tick(cx);
    cx.update(|_, cx| {
        section.update(cx, |this, _| {
            assert!(this.ticking, "a redraw restarts the loop");
            assert_eq!(this.head, head_after(3));
        })
    });
}
