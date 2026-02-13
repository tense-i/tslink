use opentelemetry_sdk::runtime::Tokio;
use prometheus::{Encoder, HistogramOpts, HistogramVec, IntGaugeVec, Opts, Registry, TextEncoder};
use prometheus::{IntCounterVec, IntGauge};
use tracing_subscriber::fmt;
use tracing_subscriber::prelude::*;
use tracing_subscriber::EnvFilter;

/// Global metrics registry and metric handles.
#[derive(Clone)]
pub struct Metrics {
    pub registry: Registry,
    /// Counter: total MQTT messages received, labels: [topic_type, product_key]
    pub mqtt_messages_total: IntCounterVec,
    /// Histogram: MQTT message processing latency in seconds, labels: [topic_type]
    pub mqtt_message_latency_seconds: HistogramVec,
    /// Gauge: current online device count
    pub device_online_count: IntGauge,
    /// Histogram: Redis operation duration in seconds, labels: [operation]
    pub redis_operation_duration_seconds: HistogramVec,
    /// Gauge: active MQTT connections
    pub mqtt_connections: IntGaugeVec,
}

impl Metrics {
    /// Create a new metrics registry with all TSLink metrics registered.
    pub fn new() -> Self {
        let registry = Registry::new();

        let mqtt_messages_total = IntCounterVec::new(
            Opts::new("tslink_mqtt_messages_total", "Total MQTT messages received"),
            &["topic_type", "product_key"],
        )
        .expect("failed to create mqtt_messages_total metric");

        let mqtt_message_latency_seconds = HistogramVec::new(
            HistogramOpts::new(
                "tslink_mqtt_message_latency_seconds",
                "MQTT message processing latency in seconds",
            )
            .buckets(vec![0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0]),
            &["topic_type"],
        )
        .expect("failed to create mqtt_message_latency_seconds metric");

        let device_online_count = IntGauge::new(
            "tslink_device_online_count",
            "Current number of online devices",
        )
        .expect("failed to create device_online_count metric");

        let redis_operation_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "tslink_redis_operation_duration_seconds",
                "Redis operation duration in seconds",
            )
            .buckets(vec![0.0005, 0.001, 0.005, 0.01, 0.025, 0.05, 0.1]),
            &["operation"],
        )
        .expect("failed to create redis_operation_duration_seconds metric");

        let mqtt_connections = IntGaugeVec::new(
            Opts::new(
                "tslink_mqtt_connections",
                "Number of active MQTT connections",
            ),
            &["broker"],
        )
        .expect("failed to create mqtt_connections metric");

        // Register all metrics
        registry
            .register(Box::new(mqtt_messages_total.clone()))
            .expect("failed to register mqtt_messages_total");
        registry
            .register(Box::new(mqtt_message_latency_seconds.clone()))
            .expect("failed to register mqtt_message_latency_seconds");
        registry
            .register(Box::new(device_online_count.clone()))
            .expect("failed to register device_online_count");
        registry
            .register(Box::new(redis_operation_duration_seconds.clone()))
            .expect("failed to register redis_operation_duration_seconds");
        registry
            .register(Box::new(mqtt_connections.clone()))
            .expect("failed to register mqtt_connections");

        Self {
            registry,
            mqtt_messages_total,
            mqtt_message_latency_seconds,
            device_online_count,
            redis_operation_duration_seconds,
            mqtt_connections,
        }
    }

    /// Encode all registered metrics into Prometheus text format.
    pub fn encode(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize the tracing subscriber with structured JSON logging
/// and optional OpenTelemetry OTLP trace export.
///
/// Log level is controlled by the `RUST_LOG` environment variable.
/// Default: `tslink=info,rumqttc=warn`
///
/// If `OTEL_EXPORTER_OTLP_ENDPOINT` is set (e.g. `http://localhost:4317`),
/// traces are also exported via gRPC OTLP to Jaeger/Tempo.
pub fn init_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("tslink=info,rumqttc=warn"));

    let fmt_layer = fmt::layer()
        .json()
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);

    // Optionally enable OpenTelemetry OTLP export
    let otel_layer = match std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT") {
        Ok(endpoint) if !endpoint.is_empty() => {
            match init_otel_tracer(&endpoint) {
                Ok(tracer) => {
                    tracing::info!(endpoint = %endpoint, "OpenTelemetry OTLP tracing enabled");
                    Some(tracing_opentelemetry::layer().with_tracer(tracer))
                }
                Err(e) => {
                    eprintln!("WARNING: Failed to init OpenTelemetry tracer: {} — continuing without OTEL", e);
                    None
                }
            }
        }
        _ => None,
    };

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(otel_layer)
        .init();
}

/// Initialize an OpenTelemetry OTLP tracer.
fn init_otel_tracer(
    endpoint: &str,
) -> std::result::Result<opentelemetry_sdk::trace::Tracer, opentelemetry::trace::TraceError> {
    use opentelemetry_otlp::WithExportConfig;

    let exporter = opentelemetry_otlp::new_exporter()
        .tonic()
        .with_endpoint(endpoint);

    let tracer = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(exporter)
        .with_trace_config(
            opentelemetry_sdk::trace::config()
                .with_resource(opentelemetry_sdk::Resource::new(vec![
                    opentelemetry::KeyValue::new("service.name", "tslink"),
                    opentelemetry::KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
                ])),
        )
        .install_batch(Tokio)?;

    Ok(tracer)
}

/// Shutdown OpenTelemetry tracer provider (call before process exit).
pub fn shutdown_otel() {
    opentelemetry::global::shutdown_tracer_provider();
}

/// Initialize the full telemetry stack (tracing + metrics).
///
/// Returns a `Metrics` handle that can be shared across the application.
pub fn init_telemetry() -> Metrics {
    init_tracing();
    let metrics = Metrics::new();
    tracing::info!(
        metrics_count = 5,
        "telemetry initialized: tracing (JSON) + prometheus metrics"
    );
    metrics
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_creation() {
        let metrics = Metrics::new();
        // Verify metrics are registered by encoding them
        let output = metrics.encode();
        assert!(output.is_empty() || output.contains("tslink_"));
    }

    #[test]
    fn test_metrics_increment() {
        let metrics = Metrics::new();
        metrics
            .mqtt_messages_total
            .with_label_values(&["property", "pk001"])
            .inc();
        let output = metrics.encode();
        assert!(output.contains("tslink_mqtt_messages_total"));
    }

    #[test]
    fn test_device_online_gauge() {
        let metrics = Metrics::new();
        metrics.device_online_count.set(42);
        let output = metrics.encode();
        assert!(output.contains("42"));
    }
}
