use gpui_kit::TestAppContext;
use gpui_kit::prelude::*;

use crate::features::pages::MicrophonePage;
use crate::microphone::{self, CaptureStatus, InputDeviceInfo};

fn device(id: &str) -> InputDeviceInfo {
    InputDeviceInfo {
        id: id.to_string(),
        name: format!("Device {id}"),
        is_default: false,
        channels: 2,
        sample_rate: 48_000,
        sample_format: "F32".to_string(),
    }
}

#[gpui_kit::test]
fn idle_page_renders_without_a_source(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::app_state::initialize(cx);
        microphone::initialize(cx);
    });
    let waveform = cx.read(|cx| microphone::snapshot(cx)).waveform;
    cx.update(gpui_kit::component::theme::init);
    cx.add_window(|window, cx| MicrophonePage::new(window, cx));
    cx.read(|cx| {
        assert!(!waveform.read(cx).has_source());
        assert_eq!(microphone::snapshot(cx).status, CaptureStatus::Idle);
    });
}

#[gpui_kit::test]
fn demo_page_renders_with_a_zero_meter_before_the_first_tick(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::app_state::initialize(cx);
        microphone::initialize(cx);
        microphone::force_demo_for_tests(cx);
        microphone::start(cx);
    });
    let waveform = cx.read(|cx| microphone::snapshot(cx)).waveform;
    cx.update(gpui_kit::component::theme::init);
    cx.add_window(|window, cx| MicrophonePage::new(window, cx));
    cx.read(|cx| {
        assert!(waveform.read(cx).has_source());
        assert_eq!(microphone::meter(cx).read(), (0.0, 0.0));
    });
    cx.update(|cx| microphone::stop(cx));
}

#[gpui_kit::test]
fn device_refresh_reconciles_the_select_items(cx: &mut TestAppContext) {
    cx.update(|cx| {
        crate::app_state::initialize(cx);
        microphone::initialize(cx);
    });
    // Let initialize's background enumeration land so it cannot race the seeds.
    cx.executor().run_until_parked();
    cx.update(gpui_kit::component::theme::init);
    let mut page = None;
    cx.add_window(|window, cx| {
        let view = MicrophonePage::new(window, cx);
        page = Some(cx.entity());
        view
    });
    let page = page.unwrap();
    cx.update(|cx| {
        let global: Vec<String> = microphone::devices(cx)
            .iter()
            .map(|device| device.id.clone())
            .collect();
        assert_eq!(page.read(cx).device_ids, global);
    });
    cx.update(|cx| {
        microphone::seed_devices_for_tests(cx, vec![device("alsa:one"), device("alsa:two")])
    });
    cx.executor().run_until_parked();
    cx.update(|cx| {
        assert_eq!(page.read(cx).device_ids, vec!["alsa:one", "alsa:two"]);
    });
    cx.update(|cx| microphone::seed_devices_for_tests(cx, vec![device("alsa:three")]));
    cx.executor().run_until_parked();
    cx.update(|cx| {
        assert_eq!(page.read(cx).device_ids, vec!["alsa:three"]);
    });
}
