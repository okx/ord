use {
  crate::TRACER_PROVIDER,
  anyhow::{anyhow, Result},
  opentelemetry::{global, trace::TracerProvider, KeyValue},
  opentelemetry_otlp::WithExportConfig,
  opentelemetry_sdk::{
    trace::{Sampler, SdkTracerProvider},
    Resource,
  },
  opentelemetry_semantic_conventions::{
    attribute::{DEPLOYMENT_ENVIRONMENT_NAME, HOST_IP, SERVICE_VERSION},
    SCHEMA_URL,
  },
  reqwest::Url,
  tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter, Layer, Registry},
};

/// Get the local IP address
///
/// Implements the same logic as Go's getLocalIP using if_addrs crate:
/// Iterates through all network interfaces and returns the first non-loopback IPv4 address.
///
/// Returns "unknown" if no suitable address is found.
fn get_local_ip() -> String {
  if_addrs::get_if_addrs()
    .ok()
    .and_then(|addrs| {
      addrs
        .into_iter()
        .find(|addr| {
          // Find first non-loopback IPv4 address
          !addr.is_loopback() && addr.ip().is_ipv4()
        })
        .map(|addr| addr.ip().to_string())
    })
    .unwrap_or_else(|| "unknown".to_string())
}

/// Initialize OpenTelemetry tracing
///
/// Sets up the OpenTelemetry pipeline to export traces to an OTLP endpoint.
/// This enables distributed tracing across the application.
///
/// # Arguments
/// * `otlp_endpoint` - OTLP endpoint URL (e.g., "http://localhost:4318/v1/traces")
/// * `filter` - Tracing filter (e.g., "info", "debug", "ord=debug")
/// * `sample_rate` - Sampling rate (0.0-1.0, where 1.0 means sample all traces)
/// * `environment` - Deployment environment name (e.g., "pro", "pre", "dev")
/// * `service_name` - Service name
///
/// # Example
/// ```ignore
/// use tracing_subscriber::EnvFilter;
/// init_tracing(
///   &"http://localhost:4318/v1/traces".parse()?,
///   EnvFilter::new("info"),
///   1.0,
///   "pro",
///   "ord-indexer"
/// )?;
/// ```
pub fn init_tracing(
  otlp_endpoint: &Url,
  filter: EnvFilter,
  sample_rate: f64,
  environment: &str,
  service_name: &str,
) -> Result<()> {
  let local_ip = get_local_ip();

  let resource = Resource::builder()
    .with_service_name(service_name.to_string())
    .with_schema_url(
      [
        KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION")),
        KeyValue::new(DEPLOYMENT_ENVIRONMENT_NAME, environment.to_string()),
        KeyValue::new(HOST_IP, local_ip.clone()),
      ],
      SCHEMA_URL,
    )
    .build();

  let sampler = Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(sample_rate)));

  let span_exporter = opentelemetry_otlp::SpanExporter::builder()
    .with_http()
    .with_endpoint(otlp_endpoint.to_string())
    .with_timeout(std::time::Duration::from_secs(10))
    .build()
    .map_err(|e| anyhow!("Failed to build OTLP exporter: {}", e))?;

  let tracer_provider = SdkTracerProvider::builder()
    .with_batch_exporter(span_exporter)
    .with_resource(resource)
    .with_sampler(sampler)
    .build();

  global::set_tracer_provider(tracer_provider.clone());
  let tracer = tracer_provider.tracer("ord");

  // Store the tracer provider for graceful shutdown
  TRACER_PROVIDER.lock().unwrap().replace(tracer_provider);

  let telemetry_layer = tracing_opentelemetry::layer()
    .with_tracer(tracer)
    .with_filter(filter.clone());

  let fmt_layer = fmt::layer()
    .with_target(true)
    .with_thread_ids(true)
    .with_line_number(true)
    .with_filter(filter);

  let subscriber = Registry::default().with(telemetry_layer).with(fmt_layer);

  tracing::subscriber::set_global_default(subscriber)
    .map_err(|e| anyhow!("Failed to set tracing subscriber: {}", e))?;

  log::info!(
    "OpenTelemetry tracing initialized: service_name={}, environment={}, host_ip={}, endpoint={}, sample_rate={}",
    service_name,
    environment,
    local_ip,
    otlp_endpoint,
    sample_rate
  );

  Ok(())
}

/// Macro to wrap database operations with tracing spans
#[macro_export]
macro_rules! trace_db_call {
  ($operation:expr, $code:expr) => {{
    use opentelemetry::trace::SpanKind;
    let span = tracing::info_span!(
      "db_call",
      otel.kind = ?SpanKind::Client,
      otel.name = %format!("db.{}", $operation),
      db.system = "redb",
      db.operation = $operation,
    );
    let _enter = span.enter();
    $code
  }};
}

/// Macro to wrap Bitcoin RPC calls with tracing spans
#[macro_export]
macro_rules! trace_rpc_call {
  ($method:expr, $code:expr) => {{
    use opentelemetry::trace::SpanKind;
    let span = tracing::info_span!(
      "bitcoin_rpc",
      otel.kind = ?SpanKind::Client,
      otel.name = %format!("bitcoin.{}", $method),
      rpc.method = $method,
      rpc.system = "bitcoin",
    );
    let _enter = span.enter();
    $code
  }};
}
