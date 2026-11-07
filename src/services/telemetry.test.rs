use gpui_kit::TestAppContext;

use super::*;
use std::sync::Arc;

use crate::capabilities::Capability;

struct FakeSink {
    fail_record: bool,
    fail_flush: bool,
}

impl TelemetrySink for FakeSink {
    fn record_event(&self, _name: &str) -> Result<(), TelemetryError> {
        if self.fail_record {
            Err(TelemetryError::NotAvailable("record failed (test)".into()))
        } else {
            Ok(())
        }
    }

    fn record_error(&self, _error: &str) -> Result<(), TelemetryError> {
        Ok(())
    }

    fn set_user_properties(&self, _key: &str, _value: &str) -> Result<(), TelemetryError> {
        Ok(())
    }

    fn flush(&self) -> Result<(), TelemetryError> {
        if self.fail_flush {
            Err(TelemetryError::NotAvailable("flush failed (test)".into()))
        } else {
            Ok(())
        }
    }
}

#[test]
fn redacts_endpoint_host_only() {
    let value =
        redact_endpoint("https://telemetry.example.com/v1/events").expect("redacted endpoint");
    assert_eq!(value, "telemetry.example.com/…");
}

#[test]
fn resolve_endpoint_prefers_explicit() {
    let ep = resolve_otlp_endpoint(Some("https://custom.example.com:4318"));
    assert_eq!(ep, "https://custom.example.com:4318");
}

#[test]
fn resolve_endpoint_ignores_empty_explicit() {
    let ep = resolve_otlp_endpoint(Some(""));
    assert!(!ep.is_empty());
}

#[test]
fn resolve_endpoint_falls_back_to_default() {
    // Env mutation is process-global and cargo test runs threads in one
    // process; sibling env readers here tolerate the var vanishing mid-run.
    unsafe { std::env::remove_var(ENV_OTLP_ENDPOINT) };
    let ep = resolve_otlp_endpoint(None);
    assert_eq!(ep, DEFAULT_OTLP_ENDPOINT);
}

// Without the `otlp` feature nothing is compiled in, so the sink must report
// nothing installed; with it, installation needs a tokio runtime (below).
#[cfg(not(feature = "otlp"))]
#[test]
fn remote_sink_reports_no_provider_without_feature() {
    let sink = RemoteSink::new("http://localhost:9999", None);
    assert!(!sink.provider_installed);
}

#[cfg(feature = "otlp")]
#[test]
fn remote_sink_installs_provider_with_tokio_runtime() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("tokio runtime for test");
    let sink = RemoteSink::new("http://localhost:9999", Some(rt.handle().clone()));
    assert!(sink.provider_installed);
}

#[cfg(feature = "otlp")]
#[test]
fn remote_sink_without_runtime_stays_uninstalled() {
    // Installing without a runtime handle used to panic (batch exporter needs
    // a reactor); it must degrade to an uninstalled sink instead.
    let sink = RemoteSink::new("http://localhost:9999", None);
    assert!(!sink.provider_installed);
}

// set_mode arms the config debounce; SAVE_SCHEDULED is process-global, so
// tests that persist hold the shared lock and drain the timer before ending.
#[gpui_kit::test]
async fn initialize_applies_the_persisted_telemetry_mode(cx: &TestAppContext) {
    let _guard = crate::app_state::debounce_test_lock();
    cx.update(|cx| {
        crate::app_state::install_for_tests(
            crate::app_state::AppConfig {
                telemetry_mode: TelemetryMode::LocalOnly,
                ..crate::app_state::AppConfig::default()
            },
            cx,
        );
        initialize(cx);
    });

    cx.read(|cx| {
        let snap = snapshot(cx);
        assert_eq!(snap.mode, TelemetryMode::LocalOnly);
        assert!(snap.enabled);
    });

    cx.executor()
        .advance_clock(std::time::Duration::from_millis(400));
    cx.executor().run_until_parked();
}

#[gpui_kit::test]
async fn initialize_keeps_telemetry_disabled_by_default(cx: &TestAppContext) {
    cx.update(|cx| {
        crate::app_state::install_for_tests(crate::app_state::AppConfig::default(), cx);
        initialize(cx);
    });

    cx.read(|cx| {
        let snap = snapshot(cx);
        assert_eq!(snap.mode, TelemetryMode::Disabled);
        assert!(!snap.enabled);
        assert!(!snap.consented);
    });
}

#[gpui_kit::test]
async fn set_mode_persists_the_decision(cx: &TestAppContext) {
    let _guard = crate::app_state::debounce_test_lock();
    cx.update(|cx| {
        crate::app_state::install_for_tests(crate::app_state::AppConfig::default(), cx);
        initialize(cx);
        set_mode(TelemetryMode::LocalOnly, true, None, cx);
    });

    cx.read(|cx| {
        assert_eq!(
            crate::app_state::config(cx).telemetry_mode,
            TelemetryMode::LocalOnly
        );
    });

    cx.executor()
        .advance_clock(std::time::Duration::from_millis(400));
    cx.executor().run_until_parked();
}

#[gpui_kit::test]
async fn set_mode_remote_without_consent_keeps_recording(cx: &TestAppContext) {
    let _guard = crate::app_state::debounce_test_lock();
    cx.update(|cx| {
        crate::app_state::install_for_tests(crate::app_state::AppConfig::default(), cx);
        initialize(cx);
        set_mode(
            TelemetryMode::Remote,
            false,
            Some("https://collector.example.com"),
            cx,
        );
        record_event("evt", cx);
    });

    // The test App has no tokio runtime, so an uninstalled RemoteSink would
    // reject the event; a counted record proves the consent gate won.
    cx.read(|cx| {
        let snap = snapshot(cx);
        assert_eq!(snap.mode, TelemetryMode::Remote);
        assert!(!snap.consented);
        assert!(!snap.enabled);
        assert_eq!(snap.events_recorded, 1);
        assert!(snap.last_error.is_none());
        assert_eq!(
            crate::capabilities::snapshot(cx)[&Capability::TELEMETRY]
                .reason
                .as_deref(),
            Some("telemetry disabled until consent")
        );
    });

    cx.executor()
        .advance_clock(std::time::Duration::from_millis(400));
    cx.executor().run_until_parked();
}

#[gpui_kit::test]
async fn set_mode_local_only_with_consent_records_events(cx: &TestAppContext) {
    let _guard = crate::app_state::debounce_test_lock();
    cx.update(|cx| {
        crate::app_state::install_for_tests(crate::app_state::AppConfig::default(), cx);
        initialize(cx);
        set_mode(TelemetryMode::LocalOnly, true, None, cx);
        record_event("evt", cx);
        record_error("boom", cx);
    });

    cx.read(|cx| {
        let snap = snapshot(cx);
        assert_eq!(snap.mode, TelemetryMode::LocalOnly);
        assert!(snap.enabled);
        assert_eq!(snap.events_recorded, 2);
        assert!(snap.last_error.is_none());
        assert!(!crate::capabilities::snapshot(cx)[&Capability::TELEMETRY].degraded);
    });

    cx.executor()
        .advance_clock(std::time::Duration::from_millis(400));
    cx.executor().run_until_parked();
}

#[gpui_kit::test]
async fn record_and_flush_outcomes_update_the_capability(cx: &TestAppContext) {
    fn fake_sink(fail_flush: bool) -> Arc<dyn TelemetrySink> {
        Arc::new(FakeSink {
            fail_record: false,
            fail_flush,
        })
    }

    cx.update(|cx| {
        initialize(cx);
        cx.set_global(TelemetryRuntime {
            sink: fake_sink(false),
        });
        record_event("evt", cx);
    });
    cx.executor().run_until_parked();

    cx.read(|cx| {
        assert_eq!(snapshot(cx).events_recorded, 1);
        assert!(!crate::capabilities::snapshot(cx)[&Capability::TELEMETRY].degraded);
    });

    cx.update(|cx| {
        cx.set_global(TelemetryRuntime {
            sink: fake_sink(true),
        });
        flush(cx);
    });
    cx.executor().run_until_parked();

    cx.read(|cx| {
        assert!(
            snapshot(cx).last_error.is_some(),
            "a failed flush must surface an error"
        );
        assert!(crate::capabilities::snapshot(cx)[&Capability::TELEMETRY].degraded);
    });

    cx.update(|cx| {
        cx.set_global(TelemetryRuntime {
            sink: fake_sink(false),
        });
        flush(cx);
    });
    cx.executor().run_until_parked();

    cx.read(|cx| {
        assert!(
            snapshot(cx).last_error.is_none(),
            "a successful flush clears the stale error"
        );
        assert!(!crate::capabilities::snapshot(cx)[&Capability::TELEMETRY].degraded);
    });
}

#[gpui_kit::test]
async fn record_failure_surfaces_error_without_counting(cx: &TestAppContext) {
    cx.update(|cx| {
        initialize(cx);
        cx.set_global(TelemetryRuntime {
            sink: Arc::new(FakeSink {
                fail_record: true,
                fail_flush: false,
            }),
        });
        record_event("evt", cx);
    });
    cx.executor().run_until_parked();

    cx.read(|cx| {
        let snap = snapshot(cx);
        assert_eq!(snap.events_recorded, 0, "failed records must not count");
        assert!(snap.last_error.is_some());
        assert!(crate::capabilities::snapshot(cx)[&Capability::TELEMETRY].degraded);
    });
}
