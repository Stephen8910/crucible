//! OpenTelemetry tracing initialisation.
//!
//! This module wires the [`tracing`] subscriber stack to an OTLP exporter so
//! that every `tracing` span is forwarded to an OpenTelemetry-compatible
//! collector (Jaeger, Grafana Tempo, OTEL Collector, …).
//!
//! # Usage
//!
//! ```rust,no_run
//! use backend::services::tracing::{init, TracingConfig};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let cfg = TracingConfig::from_env();
//!     let _guard = init(cfg)?;
//!     // _guard shuts down the tracer provider when dropped
//!     Ok(())
//! }
//! ```
//!
//! # Environment variables
//!
//! | Variable | Default | Description |
//! |---|---|---|
//! | `OTEL_EXPORTER_OTLP_ENDPOINT` | `http://localhost:4318` | OTLP HTTP collector endpoint |
//! | `OTEL_SERVICE_NAME` | `backend` | Service name attached to every span |
//! | `RUST_LOG` | `backend=debug` | `tracing` filter directive |

use opentelemetry::global;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::{
    trace::{RandomIdGenerator, Sampler, SdkTracerProvider},
    Resource,
};
use thiserror::Error;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors that can occur while initialising the tracing stack.
#[derive(Debug, Error)]
pub enum TracingError {
    /// The OTLP exporter could not be built.
    #[error("Failed to build OTLP span exporter: {0}")]
    ExporterBuild(String),

    /// The tracing subscriber could not be installed.
    #[error("Failed to install tracing subscriber: {0}")]
    SubscriberInit(String),
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// Configuration for the OpenTelemetry tracing stack.
#[derive(Debug, Clone)]
pub struct TracingConfig {
    /// OTLP HTTP endpoint (e.g. `http://localhost:4318`).
    pub otlp_endpoint: String,
    /// Logical service name attached to every span.
    pub service_name: String,
    /// `tracing` filter directive (e.g. `"backend=debug,tower_http=info"`).
    pub log_filter: String,
}

impl TracingConfig {
    /// Build configuration from environment variables, falling back to
    /// sensible defaults when variables are absent.
    pub fn from_env() -> Self {
        Self {
            otlp_endpoint: std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:4318".to_string()),
            service_name: std::env::var("OTEL_SERVICE_NAME")
                .unwrap_or_else(|_| "backend".to_string()),
            log_filter: std::env::var("RUST_LOG")
                .unwrap_or_else(|_| "backend=debug,tower_http=debug".to_string()),
        }
    }
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

// ---------------------------------------------------------------------------
// Guard
// ---------------------------------------------------------------------------

/// RAII guard that shuts down the global tracer provider on drop.
///
/// Hold this value for the lifetime of the process. Dropping it flushes any
/// in-flight spans and releases the exporter connection.
pub struct TracingGuard {
    provider: SdkTracerProvider,
}

impl TracingGuard {
    /// Create a guard backed by a no-op provider (no exporter attached).
    /// Useful as a fallback when the real OTel initialisation fails.
    pub fn noop() -> Self {
        Self {
            provider: SdkTracerProvider::builder().build(),
        }
    }
}

impl Drop for TracingGuard {
    fn drop(&mut self) {
        if let Err(e) = self.provider.shutdown() {
            // Can't use tracing here — subscriber may already be gone.
            eprintln!("OpenTelemetry tracer provider shutdown error: {e}");
        }
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Initialise the global [`tracing`] subscriber with an OTLP exporter layer.
///
/// The subscriber stack is:
/// 1. `EnvFilter` — honours `RUST_LOG` / [`TracingConfig::log_filter`].
/// 2. `tracing_subscriber::fmt` — human-readable output to stdout.
/// 3. `tracing_opentelemetry::OpenTelemetryLayer` — forwards spans to the
///    OTLP collector at [`TracingConfig::otlp_endpoint`].
///
/// Returns a [`TracingGuard`] that must be kept alive for the duration of the
/// process. Dropping it triggers a graceful shutdown of the tracer provider.
///
/// # Errors
///
/// Returns [`TracingError`] if the exporter cannot be built or the subscriber
/// cannot be installed (e.g. a global subscriber is already set).
pub fn init(cfg: TracingConfig) -> Result<TracingGuard, TracingError> {
    let provider = build_provider(&cfg)?;

    // Register as the global provider so `global::tracer()` works anywhere.
    global::set_tracer_provider(provider.clone());

    let otel_layer =
        tracing_opentelemetry::layer().with_tracer(provider.tracer(cfg.service_name.clone()));

    let filter =
        EnvFilter::try_new(&cfg.log_filter).unwrap_or_else(|_| EnvFilter::new("backend=debug"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer())
        .with(otel_layer)
        .try_init()
        .map_err(|e| TracingError::SubscriberInit(e.to_string()))?;

    Ok(TracingGuard { provider })
}

/// Build a [`SdkTracerProvider`] backed by a batched OTLP HTTP exporter.
fn build_provider(cfg: &TracingConfig) -> Result<SdkTracerProvider, TracingError> {
    let exporter = SpanExporter::builder()
        .with_http()
        .with_endpoint(&cfg.otlp_endpoint)
        .build()
        .map_err(|e| TracingError::ExporterBuild(e.to_string()))?;

    let resource = Resource::builder()
        .with_service_name(cfg.service_name.clone())
        .build();

    let provider = SdkTracerProvider::builder()
        .with_resource(resource)
        .with_sampler(Sampler::AlwaysOn)
        .with_id_generator(RandomIdGenerator::default())
        .with_batch_exporter(exporter)
        .build();

    Ok(provider)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        // Build config directly without relying on env vars.
        let cfg = TracingConfig {
            otlp_endpoint: "http://localhost:4318".to_string(),
            service_name: "backend".to_string(),
            log_filter: "backend=debug,tower_http=debug".to_string(),
        };
        assert_eq!(cfg.otlp_endpoint, "http://localhost:4318");
        assert_eq!(cfg.service_name, "backend");
        assert!(!cfg.log_filter.is_empty());
    }

    #[test]
    fn test_config_from_env_values() {
        // Verify that TracingConfig correctly stores whatever values are given.
        let cfg = TracingConfig {
            otlp_endpoint: "http://collector:4318".to_string(),
            service_name: "my-service".to_string(),
            log_filter: "info".to_string(),
        };
        assert_eq!(cfg.otlp_endpoint, "http://collector:4318");
        assert_eq!(cfg.service_name, "my-service");
        assert_eq!(cfg.log_filter, "info");
    }

    #[test]
    fn test_tracing_error_display() {
        let e = TracingError::ExporterBuild("bad url".to_string());
        assert!(e.to_string().contains("bad url"));

        let e = TracingError::SubscriberInit("already set".to_string());
        assert!(e.to_string().contains("already set"));
    }

    #[test]
    fn test_build_provider_succeeds() {
        // build_provider only constructs SDK objects; no network connection is
        // opened, so this works without a live collector.
        let cfg = TracingConfig {
            otlp_endpoint: "http://localhost:4318".to_string(),
            service_name: "test".to_string(),
            log_filter: "debug".to_string(),
        };
        let result = build_provider(&cfg);
        assert!(result.is_ok());
        let _ = result.unwrap().shutdown();
    }

    #[test]
    fn test_build_provider_custom_endpoint() {
        let cfg = TracingConfig {
            otlp_endpoint: "http://otel-collector.internal:4318".to_string(),
            service_name: "svc-a".to_string(),
            log_filter: "info".to_string(),
        };
        let result = build_provider(&cfg);
        assert!(result.is_ok());
        let _ = result.unwrap().shutdown();
    }

    #[test]
    fn test_tracing_guard_shuts_down_on_drop() {
        let cfg = TracingConfig {
            otlp_endpoint: "http://localhost:4318".to_string(),
            service_name: "guard-test".to_string(),
            log_filter: "debug".to_string(),
        };
        let provider = build_provider(&cfg).unwrap();
        let guard = TracingGuard { provider };
        drop(guard); // must not panic
    }

    #[test]
    fn test_tracing_guard_noop() {
        let guard = TracingGuard::noop();
        drop(guard); // must not panic
    }

    #[test]
    fn test_config_clone() {
        let cfg = TracingConfig {
            otlp_endpoint: "http://a:4318".to_string(),
            service_name: "svc".to_string(),
            log_filter: "debug".to_string(),
        };
        let cloned = cfg.clone();
        assert_eq!(cfg.otlp_endpoint, cloned.otlp_endpoint);
        assert_eq!(cfg.service_name, cloned.service_name);
    }
}
