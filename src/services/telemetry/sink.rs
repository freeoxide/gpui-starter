#[cfg(feature = "otlp")]
use std::sync::Mutex;

use super::TelemetryError;

// Keyed by endpoint so an endpoint change rebuilds instead of
// first-installed-wins; emptied at shutdown to force a fresh build later.
#[cfg(feature = "otlp")]
static INSTALLED_PIPELINE: Mutex<Option<InstalledPipeline>> = Mutex::new(None);

#[cfg(feature = "otlp")]
struct InstalledPipeline {
    endpoint: String,
    provider: opentelemetry_sdk::trace::TracerProvider,
}

#[cfg(feature = "otlp")]
fn lock_pipeline() -> std::sync::MutexGuard<'static, Option<InstalledPipeline>> {
    INSTALLED_PIPELINE
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Drain a retired provider off-thread: shutdown blocks on the collector, and
/// dropping the last clone would block whichever thread does it.
#[cfg(feature = "otlp")]
fn retire(provider: opentelemetry_sdk::trace::TracerProvider) {
    std::thread::spawn(move || {
        let _ = provider.shutdown();
    });
}

/// Install an OTLP HTTP tracer provider on the global pipeline, reusing the
/// live one only when the endpoint is unchanged; a change drains the old
/// provider off-thread and builds a fresh export pipeline.
#[cfg(feature = "otlp")]
pub(super) fn install_otlp_tracer(
    endpoint: &str,
) -> Result<opentelemetry_sdk::trace::TracerProvider, TelemetryError> {
    use opentelemetry::KeyValue;
    use opentelemetry::global;
    use opentelemetry_otlp::WithExportConfig;
    use opentelemetry_sdk::Resource;
    use opentelemetry_sdk::propagation::TraceContextPropagator;
    use opentelemetry_sdk::runtime::Tokio;

    use super::SERVICE_NAME;

    let mut installed = lock_pipeline();
    if let Some(current) = installed.as_ref()
        && current.endpoint == endpoint
    {
        return Ok(current.provider.clone());
    }
    if let Some(old) = installed.take() {
        retire(old.provider);
    }

    let exporter = opentelemetry_otlp::new_exporter()
        .http()
        .with_endpoint(format!("{endpoint}/v1/traces"));

    let provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(opentelemetry_sdk::trace::Config::default().with_resource(
            Resource::new(vec![KeyValue::new("service.name", SERVICE_NAME)]),
        ))
        .install_batch(Tokio)
        .map_err(|e| TelemetryError::Otlp(Box::new(e)))?;

    global::set_text_map_propagator(TraceContextPropagator::new());
    // Keep the original for force_flush; the global registry holds a clone.
    global::set_tracer_provider(provider.clone());

    tracing::info!(
        target: "gpui_starter::telemetry",
        endpoint = %endpoint,
        "OTLP tracer provider installed"
    );
    *installed = Some(InstalledPipeline {
        endpoint: endpoint.to_owned(),
        provider: provider.clone(),
    });
    Ok(provider)
}

/// Retire the installed pipeline at app shutdown so a later install builds a
/// fresh provider instead of reusing a drained one as if it were live.
#[cfg(feature = "otlp")]
pub(super) fn shutdown_pipeline() {
    if let Some(current) = lock_pipeline().take() {
        retire(current.provider);
    }
}

#[derive(Default)]
pub(super) struct DisabledSink;

impl super::TelemetrySink for DisabledSink {
    fn record_event(&self, _name: &str) -> Result<(), TelemetryError> {
        Ok(())
    }

    fn record_error(&self, _error: &str) -> Result<(), TelemetryError> {
        Ok(())
    }

    fn set_user_properties(&self, _key: &str, _value: &str) -> Result<(), TelemetryError> {
        Ok(())
    }

    fn flush(&self) -> Result<(), TelemetryError> {
        Ok(())
    }
}

#[derive(Default)]
pub(super) struct LocalSink;

impl super::TelemetrySink for LocalSink {
    fn record_event(&self, name: &str) -> Result<(), TelemetryError> {
        tracing::debug!(target: "gpui_starter::telemetry", event = %name, "local telemetry event");
        Ok(())
    }

    fn record_error(&self, error: &str) -> Result<(), TelemetryError> {
        tracing::warn!(target: "gpui_starter::telemetry", error = %error, "local telemetry error");
        Ok(())
    }

    fn set_user_properties(&self, key: &str, value: &str) -> Result<(), TelemetryError> {
        tracing::debug!(target: "gpui_starter::telemetry", key = %key, value = %value, "local telemetry user property");
        Ok(())
    }

    fn flush(&self) -> Result<(), TelemetryError> {
        Ok(())
    }
}

/// Remote sink exporting via OTLP over HTTP. Endpoint precedence matches
/// [`super::resolve_otlp_endpoint`]; `provider_installed` says the SDK
/// pipeline was built, never that the collector is reachable.
#[derive(Clone)]
pub(super) struct RemoteSink {
    endpoint: String,
    pub(super) provider_installed: bool,
    /// Kept so flush can call force_flush without shutting down the global provider.
    #[cfg(feature = "otlp")]
    provider: Option<opentelemetry_sdk::trace::TracerProvider>,
}

impl RemoteSink {
    /// `runtime` must be the shared tokio runtime handle: the batch exporter
    /// spawns its batching task on the ambient runtime, and GPUI threads run
    /// outside any tokio context. Without a handle the sink stays uninstalled
    /// instead of panicking.
    pub(super) fn new(endpoint: &str, runtime: Option<tokio::runtime::Handle>) -> Self {
        #[cfg(feature = "otlp")]
        let (provider_installed, provider) = Self::install(endpoint, runtime);
        // The () install payload only exists without the `otlp` feature.
        #[cfg(not(feature = "otlp"))]
        let (provider_installed, ()) = Self::install(endpoint, runtime);
        Self {
            endpoint: endpoint.to_owned(),
            provider_installed,
            #[cfg(feature = "otlp")]
            provider,
        }
    }

    #[cfg(feature = "otlp")]
    fn install(
        endpoint: &str,
        runtime: Option<tokio::runtime::Handle>,
    ) -> (bool, Option<opentelemetry_sdk::trace::TracerProvider>) {
        let Some(handle) = runtime else {
            tracing::warn!(
                target: "gpui_starter::telemetry",
                "no tokio runtime handle available; OTLP exporter not installed"
            );
            return (false, None);
        };
        let _enter = handle.enter();
        match install_otlp_tracer(endpoint) {
            Ok(provider) => (true, Some(provider)),
            Err(err) => {
                tracing::warn!(
                    target: "gpui_starter::telemetry",
                    endpoint = %endpoint,
                    error = %err,
                    "OTLP tracer provider installation failed; events will be logged locally"
                );
                (false, None)
            }
        }
    }

    #[cfg(not(feature = "otlp"))]
    fn install(endpoint: &str, _runtime: Option<tokio::runtime::Handle>) -> (bool, ()) {
        tracing::debug!(
            target: "gpui_starter::telemetry",
            endpoint = %endpoint,
            "OTLP export not compiled in (rebuild with --features otlp); events stay local"
        );
        (false, ())
    }

    /// Push buffered spans to the collector without disabling the provider.
    #[cfg(feature = "otlp")]
    fn force_flush_provider(&self) -> Result<(), TelemetryError> {
        let Some(provider) = self.provider.as_ref() else {
            tracing::warn!(
                target: "gpui_starter::telemetry",
                "force_flush called but no SDK provider is available; falling back to no-op"
            );
            return Ok(());
        };

        let errors: Vec<_> = provider
            .force_flush()
            .into_iter()
            .filter_map(|r| r.err())
            .collect();

        if errors.is_empty() {
            Ok(())
        } else {
            let msg = errors
                .iter()
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join(", ");
            Err(TelemetryError::Otlp(Box::new(std::io::Error::other(
                format!("force_flush errors: {msg}"),
            ))))
        }
    }

    #[cfg(not(feature = "otlp"))]
    fn force_flush_provider(&self) -> Result<(), TelemetryError> {
        Ok(())
    }

    /// Gate for methods that need an installed provider; `log_dropped` keeps
    /// each call site's structured fields (event name, error text, key/value)
    /// in its own tracing macro.
    fn require_installed(&self, log_dropped: impl FnOnce()) -> Result<(), TelemetryError> {
        if self.provider_installed {
            return Ok(());
        }
        log_dropped();
        Err(TelemetryError::NotAvailable(format!(
            "OTLP provider not installed for {} (collector reachability unverified)",
            self.endpoint
        )))
    }
}

impl super::TelemetrySink for RemoteSink {
    fn record_event(&self, name: &str) -> Result<(), TelemetryError> {
        self.require_installed(|| {
            tracing::warn!(
                target: "gpui_starter::telemetry",
                endpoint = %self.endpoint,
                event = %name,
                "remote telemetry event dropped (provider not installed)"
            )
        })?;
        tracing::debug!(target: "gpui_starter::telemetry", endpoint = %self.endpoint, event = %name, "remote telemetry event queued");
        Ok(())
    }

    fn record_error(&self, error: &str) -> Result<(), TelemetryError> {
        self.require_installed(|| {
            tracing::warn!(
                target: "gpui_starter::telemetry",
                endpoint = %self.endpoint,
                error = %error,
                "remote telemetry error dropped (provider not installed)"
            )
        })?;
        tracing::warn!(target: "gpui_starter::telemetry", endpoint = %self.endpoint, error = %error, "remote telemetry error queued");
        Ok(())
    }

    fn set_user_properties(&self, key: &str, value: &str) -> Result<(), TelemetryError> {
        self.require_installed(|| {
            tracing::debug!(
                target: "gpui_starter::telemetry",
                endpoint = %self.endpoint,
                key = %key,
                value = %value,
                "remote telemetry user property dropped (provider not installed)"
            )
        })?;
        tracing::debug!(target: "gpui_starter::telemetry", endpoint = %self.endpoint, key = %key, value = %value, "remote telemetry user property queued");
        Ok(())
    }

    /// Flush is repeatable (Settings button) and leaves the provider active,
    /// unlike the one-way `shutdown_tracer_provider()`.
    fn flush(&self) -> Result<(), TelemetryError> {
        self.require_installed(|| {
            tracing::debug!(
                target: "gpui_starter::telemetry",
                endpoint = %self.endpoint,
                "remote telemetry flush skipped (provider not installed)"
            )
        })?;
        tracing::debug!(target: "gpui_starter::telemetry", endpoint = %self.endpoint, "remote telemetry flush");
        self.force_flush_provider()
    }
}
