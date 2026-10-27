use std::time::Duration;

use gpui_kit::TestAppContext;

use super::*;

struct MicConfigGuard {
    device_id: Option<String>,
    gain_db: f32,
}

fn capture_guard(cx: &TestAppContext) -> MicConfigGuard {
    cx.read(|cx| {
        crate::app_state::with_config(cx, |c| MicConfigGuard {
            device_id: c.mic_device_id.clone(),
            gain_db: c.mic_gain_db,
        })
    })
}

fn setup(cx: &mut TestAppContext) -> MicConfigGuard {
    cx.update(|cx| {
        crate::app_state::initialize(cx);
        initialize(cx);
    });
    // initialize's detached device enumeration parks; drain it here, before
    // any test statement arms the config debounce.
    cx.executor().run_until_parked();
    capture_guard(cx)
}

// SAVE_SCHEDULED is process-global while tests run in parallel; restoring the
// pre-test values then force_save completes the cycle with no disk write.
// Call it inside the same closure that armed the debounce, before any park.
fn complete_config_cycle(cx: &mut App, guard: &MicConfigGuard) {
    crate::app_state::update_config(cx, |c| {
        c.mic_device_id = guard.device_id.clone();
        c.mic_gain_db = guard.gain_db;
    });
    crate::app_state::force_save(cx);
}

fn device(id: &str, is_default: bool) -> InputDeviceInfo {
    InputDeviceInfo {
        id: id.to_string(),
        name: format!("Device {id}"),
        is_default,
        channels: 2,
        sample_rate: 48_000,
        sample_format: "F32".to_string(),
    }
}

#[test]
fn gain_multiplier_hits_the_anchored_decibels() {
    assert_eq!(gain_multiplier(0), 1.0);
    assert!((gain_multiplier(6 * 256) - 1.9953).abs() < 1e-3);
    assert!((gain_multiplier(-6 * 256) - 0.5012).abs() < 1e-3);
}

#[gpui_kit::test]
fn initialize_loads_persisted_gain_and_registers_capability(cx: &mut TestAppContext) {
    cx.update(|cx| crate::app_state::initialize(cx));
    let guard = capture_guard(cx);
    // One closure: the arm, the assertions, and the clear all run before the
    // closure exit can park initialize's spawned enumeration task.
    cx.update(|cx| {
        crate::app_state::update_config(cx, |config| config.mic_gain_db = 6.0);
        initialize(cx);
        assert_eq!(snapshot(cx).gain_db, 6.0);
        assert_ne!(snapshot(cx).permission, MicrophonePermissionState::Unknown);
        assert!(crate::capabilities::snapshot(cx).contains_key("microphone_input"));
        complete_config_cycle(cx, &guard);
    });
    cx.executor().run_until_parked();
}

#[gpui_kit::test]
fn set_gain_db_clamps_and_persists(cx: &mut TestAppContext) {
    let guard = setup(cx);
    cx.update(|cx| {
        set_gain_db(99., cx);
        assert_eq!(snapshot(cx).gain_db, 24.0);
        assert_eq!(crate::app_state::with_config(cx, |c| c.mic_gain_db), 24.0);
        complete_config_cycle(cx, &guard);
    });
    cx.update(|cx| {
        set_gain_db(-99., cx);
        assert_eq!(snapshot(cx).gain_db, -24.0);
        assert_eq!(crate::app_state::with_config(cx, |c| c.mic_gain_db), -24.0);
        complete_config_cycle(cx, &guard);
    });
}

#[gpui_kit::test]
fn select_device_round_trips_through_config(cx: &mut TestAppContext) {
    let guard = setup(cx);
    cx.update(|cx| {
        select_device(Some("alsa:hw:0,0".to_string()), cx);
        assert_eq!(snapshot(cx).selected_device.as_deref(), Some("alsa:hw:0,0"));
        assert_eq!(
            crate::app_state::with_config(cx, |c| c.mic_device_id.clone()),
            Some("alsa:hw:0,0".to_string())
        );
        complete_config_cycle(cx, &guard);
    });
    cx.update(|cx| {
        select_device(None, cx);
        assert_eq!(snapshot(cx).selected_device, None);
        assert_eq!(
            crate::app_state::with_config(cx, |c| c.mic_device_id.clone()),
            None
        );
        complete_config_cycle(cx, &guard);
    });
}

#[gpui_kit::test]
fn demo_start_pushes_samples_and_stop_detaches(cx: &mut TestAppContext) {
    let _guard = setup(cx);
    let waveform = cx.read(|cx| snapshot(cx)).waveform;
    cx.update(|cx| {
        force_demo_for_tests(cx);
        start(cx);
    });
    assert!(cx.read(|cx| matches!(snapshot(cx).status, CaptureStatus::Capturing { .. })));
    cx.executor().advance_clock(Duration::from_millis(100));
    cx.executor().run_until_parked();
    cx.update(|cx| {
        assert!(waveform.read(cx).has_source());
        assert_eq!(waveform.read(cx).dropped_samples(), 0);
    });
    cx.update(|cx| stop(cx));
    cx.update(|cx| {
        assert!(!waveform.read(cx).has_source());
        assert_eq!(meter(cx).read(), (0.0, 0.0));
        assert_eq!(snapshot(cx).status, CaptureStatus::Idle);
    });
}

#[gpui_kit::test]
fn stop_before_start_is_a_no_op(cx: &mut TestAppContext) {
    let _guard = setup(cx);
    cx.update(|cx| stop(cx));
    assert_eq!(cx.read(|cx| snapshot(cx).status), CaptureStatus::Idle);
}

#[gpui_kit::test]
fn live_gain_updates_state_without_persisting(cx: &mut TestAppContext) {
    let guard = setup(cx);
    cx.update(|cx| {
        set_gain_db_live(6., cx);
        assert_eq!(snapshot(cx).gain_db, 6.0);
        assert_eq!(crate::app_state::with_config(cx, |c| c.mic_gain_db), 0.0);
    });
    cx.update(|cx| {
        set_gain_db_live(6., cx);
        set_gain_db(6., cx);
        assert_eq!(crate::app_state::with_config(cx, |c| c.mic_gain_db), 6.0);
        complete_config_cycle(cx, &guard);
    });
}

#[gpui_kit::test]
fn demo_gain_scales_the_metered_signal(cx: &mut TestAppContext) {
    let _guard = setup(cx);
    cx.update(|cx| {
        force_demo_for_tests(cx);
        start(cx);
    });
    cx.executor().advance_clock(Duration::from_millis(200));
    cx.executor().run_until_parked();
    let (_, quiet) = cx.read(|cx| meter(cx).read());
    cx.update(|cx| stop(cx));
    cx.update(|cx| set_gain_db_live(20., cx));
    cx.update(|cx| start(cx));
    cx.executor().advance_clock(Duration::from_millis(200));
    cx.executor().run_until_parked();
    let (_, loud) = cx.read(|cx| meter(cx).read());
    assert!(
        quiet > 0. && quiet < 1.,
        "0 dB demo peak must sit strictly inside the meter range: {quiet}"
    );
    assert_eq!(loud, 1.0, "+20 dB demo peak must clamp at full scale");
    cx.update(|cx| stop(cx));
}

#[cfg(not(target_family = "wasm"))]
#[gpui_kit::test]
fn spawn_session_re_persists_a_resolved_stale_device_id(cx: &mut TestAppContext) {
    let guard = setup(cx);
    cx.update(|cx| {
        seed_devices_for_tests(
            cx,
            vec![device("alsa:fake,0", true), device("alsa:fake,1", false)],
        );
        select_device(Some("alsa:gone".to_string()), cx);
        spawn_session(cx);
        assert_eq!(
            crate::app_state::with_config(cx, |c| c.mic_device_id.clone()),
            Some("alsa:fake,0".to_string())
        );
        assert_eq!(snapshot(cx).selected_device.as_deref(), Some("alsa:fake,0"));
        complete_config_cycle(cx, &guard);
    });
    cx.executor().run_until_parked();
    // The seeded ids never match a real enumerated device, so the session
    // must land in Error rather than leave a half-open Starting state.
    cx.update(|cx| {
        assert!(matches!(snapshot(cx).status, CaptureStatus::Error(_)));
        assert!(!snapshot(cx).waveform.read(cx).has_source());
    });
    cx.update(|cx| stop(cx));
}
