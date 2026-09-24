//! Telemetry service: opt-in telemetry with local-only and remote (OTLP) modes.

mod sink;

use std::sync::Arc;

use gpui::{App, BorrowAppContext as _, Global};
use opentelemetry::global;

use sink::{DisabledSink, LocalSink, RemoteSink};

/// OpenTelemetry Collector default for HTTP/Protobuf transport (port 4318).
const DEFAULT_OTLP_ENDPOINT: &str = "http://localhost:4318";

const ENV_OTLP_ENDPOINT: &str = "OTEL_EXPORTER_OTLP_ENDPOINT";

#[cfg(feature = "otlp")]
const SERVICE_NAME: &str = "gpui-starter";

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TelemetryMode {
    Disabled,
    LocalOnly,
    Remote,
}

#[derive(Clone, Debug)]
pub struct TelemetrySnapshot {
    pub compiled: bool,
    pub consented: bool,
    pub enabled: bool,
    pub mode: TelemetryMode,
    pub endpoint_redacted: Option<String>,
    pub events_recorded: u64,
    pub last_export_error: Option<String>,
    pub last_error: Option<String>,
}

impl Default for TelemetrySnapshot {
    fn default() -> Self {
        Self {
            compiled: true,
            consented: false,
            enabled: false,
            mode: TelemetryMode::Disabled,
            endpoint_redacted: None,
            events_recorded: 0,
            last_export_error: None,
            last_error: None,
        }
    }
}

impl Global for TelemetrySnapshot {}

#[derive(Debug, thiserror::Error)]
pub enum TelemetryError {
    #[error("telemetry not available: {0}")]
    NotAvailable(String),
    #[error("OTLP error: {0}")]
    Otlp(#[source] Box<dyn std::error::Error + Send + Sync>),
}

pub trait TelemetrySink: Send + Sync {
    fn record_event(&self, name: &str) -> Result<(), TelemetryError>;
    fn record_error(&self, error: &str) -> Result<(), TelemetryError>;
    fn set_user_properties(&self, key: &str, value: &str) -> Result<(), TelemetryError>;
    fn flush(&self) -> Result<(), TelemetryError>;
}

#[derive(Clone)]
struct TelemetryRuntime {
    sink: Arc<dyn TelemetrySink>,
}

impl Global for TelemetryRuntime {}

pub fn initialize(cx: &mut App) {
    let snapshot = TelemetrySnapshot::default();
    let runtime = TelemetryRuntime {
        sink: Arc::new(DisabledSink),
    };
    set_capability(&snapshot, cx);
    cx.set_global(snapshot);
    cx.set_global(runtime);
}

pub fn snapshot(cx: &App) -> TelemetrySnapshot {
    cx.try_global::<TelemetrySnapshot>()
        .cloned()
        .unwrap_or_default()
}

/// Without `endpoint`, `OTEL_EXPORTER_OTLP_ENDPOINT` (or the built-in
/// default) is used. Only `Remote` + consent installs the OTLP exporter.
pub fn set_mode(mode: TelemetryMode, consented: bool, endpoint: Option<&str>, cx: &mut App) {
    let resolved = resolve_otlp_endpoint(endpoint);
    let endpoint_redacted = redact_endpoint(&resolved);
    let enabled = consented && mode != TelemetryMode::Disabled;

    let (sink, connection_error): (Arc<dyn TelemetrySink>, Option<String>) =
        match (&mode, consented) {
            (TelemetryMode::Disabled, _) | (_, false) => (Arc::new(DisabledSink), None),
            (TelemetryMode::LocalOnly, true) => (Arc::new(LocalSink), None),
            (TelemetryMode::Remote, true) => {
                // install_batch spawns its batching task on the ambient tokio
                // runtime; GPUI threads run outside any, so hand it the shared one.
                let shared = crate::services::tokio_runtime::handle(cx);
                let sink = RemoteSink::new(&resolved, shared.map(|rt| rt.handle().clone()));
                let err = if sink.connected {
                    None
                } else {
                    Some(format!("failed to connect OTLP exporter to {resolved}"))
                };
                (Arc::new(sink), err)
            }
        };

    let next = TelemetrySnapshot {
        compiled: true,
        consented,
        enabled,
        mode: mode.clone(),
        endpoint_redacted,
        events_recorded: snapshot(cx).events_recorded,
        last_export_error: connection_error,
        last_error: None,
    };

    tracing::info!(
        target: "gpui_starter::telemetry",
        consented = next.consented,
        enabled = next.enabled,
        mode = ?next.mode,
        endpoint = ?next.endpoint_redacted,
        "telemetry mode updated"
    );

    set_capability(&next, cx);
    cx.update_global::<TelemetrySnapshot, _>(|snap, _cx| {
        *snap = next;
    });
    cx.set_global(TelemetryRuntime { sink });
}

pub fn record_event(name: &str, cx: &mut App) {
    with_runtime(cx, |runtime, cx| {
        let result = runtime.sink.record_event(name);
        handle_record_result(result, cx);
    });
}

pub fn record_error(error: &str, cx: &mut App) {
    with_runtime(cx, |runtime, cx| {
        let result = runtime.sink.record_error(error);
        handle_record_result(result, cx);
    });
}

pub fn set_user_property(key: &str, value: &str, cx: &mut App) {
    with_runtime(cx, |runtime, cx| {
        let result = runtime.sink.set_user_properties(key, value);
        handle_record_result(result, cx);
    });
}

pub fn flush(cx: &mut App) {
    with_runtime(cx, |runtime, cx| {
        cx.update_global::<TelemetrySnapshot, _>(|snap, _cx| {
            if let Err(err) = runtime.sink.flush() {
                snap.last_export_error = Some(err.to_string());
                snap.last_error = Some(err.to_string());
            }
        });
    });
}

/// Flush pending telemetry and permanently shut down the global tracer
/// provider. Safe to call repeatedly; later calls are no-ops.
pub fn shutdown(cx: &mut App) {
    let state = snapshot(cx);
    tracing::debug!(
        target: "gpui_starter::telemetry",
        enabled = state.enabled,
        mode = ?state.mode,
        events_recorded = state.events_recorded,
        "telemetry shutdown requested"
    );
    flush(cx);
    // The only place shutdown_tracer_provider() may be called.
    global::shutdown_tracer_provider();
}

/// Explicit endpoint wins, then `OTEL_EXPORTER_OTLP_ENDPOINT`, then the default.
fn resolve_otlp_endpoint(explicit: Option<&str>) -> String {
    if let Some(ep) = explicit
        && !ep.trim().is_empty()
    {
        return ep.trim().to_owned();
    }
    match std::env::var(ENV_OTLP_ENDPOINT) {
        Ok(v) if !v.trim().is_empty() => v.trim().to_owned(),
        _ => DEFAULT_OTLP_ENDPOINT.to_owned(),
    }
}

fn with_runtime(cx: &mut App, f: impl FnOnce(TelemetryRuntime, &mut App)) {
    if let Some(runtime) = cx.try_global::<TelemetryRuntime>().cloned() {
        f(runtime, cx);
    }
}

fn handle_record_result(result: Result<(), TelemetryError>, cx: &mut App) {
    cx.update_global::<TelemetrySnapshot, _>(|snap, _cx| match result {
        Ok(()) => {
            snap.events_recorded = snap.events_recorded.saturating_add(1);
            snap.last_error = None;
        }
        Err(err) => {
            snap.last_export_error = Some(err.to_string());
            snap.last_error = Some(err.to_string());
        }
    });
}

fn set_capability(snapshot: &TelemetrySnapshot, cx: &mut App) {
    crate::capabilities::set(
        "telemetry",
        crate::capabilities::CapabilityStatus {
            supported: snapshot.compiled,
            enabled: snapshot.enabled,
            degraded: snapshot.last_error.is_some(),
            reason: if !snapshot.consented {
                Some("telemetry disabled until consent".into())
            } else {
                None
            },
            last_error: snapshot.last_error.clone().map(Into::into),
        },
        cx,
    );
}

/// Host plus a trailing ellipsis — enough to identify the collector, never the path.
fn redact_endpoint(endpoint: &str) -> Option<String> {
    let endpoint = endpoint.trim();
    if endpoint.is_empty() {
        return None;
    }
    let host = endpoint
        .split("://")
        .nth(1)
        .unwrap_or(endpoint)
        .split('/')
        .next()
        .unwrap_or(endpoint);
    Some(format!("{host}/…"))
}

#[cfg(test)]
#[path = "../telemetry.test.rs"]
mod telemetry_test;
