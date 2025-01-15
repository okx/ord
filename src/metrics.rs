use prometheus::{Gauge, Histogram, HistogramOpts, Registry};

pub struct Metrics {
  registry: Registry,
  current_block_height: Gauge,
  total_transactions: Gauge,
  total_inscription_events: Gauge,
  total_brc20_events: Gauge,
  block_parse_duration: Histogram,
}

impl Metrics {
  pub fn registry(&self) -> &Registry {
    &self.registry
  }

  pub fn set_current_block_height(&self, height: u32) {
    self.current_block_height.set(height as f64);
  }

  pub fn increment_transaction_count(&self, count: u32) {
    self.total_transactions.add(count as f64);
  }

  pub fn increment_inscription_event_count(&self, count: u32) {
    self.total_inscription_events.add(count as f64);
  }

  pub fn increment_brc20_event_count(&self, count: u32) {
    self.total_brc20_events.add(count as f64);
  }

  pub fn observe_block_parse_duration(&self, duration: f64) {
    self.block_parse_duration.observe(duration);
  }
}

pub trait MetricsExt {
  fn set_current_block_height(&self, height: u32);
  fn increment_transaction_count(&self, count: u32);
  fn increment_inscription_event_count(&self, count: u32);
  fn increment_brc20_event_count(&self, count: u32);
  fn observe_block_parse_duration(&self, duration: f64);
}

impl MetricsExt for Option<Metrics> {
  fn set_current_block_height(&self, height: u32) {
    if let Some(metrics) = self {
      metrics.set_current_block_height(height);
    }
  }

  fn increment_transaction_count(&self, count: u32) {
    if let Some(metrics) = self {
      metrics.increment_transaction_count(count);
    }
  }

  fn increment_inscription_event_count(&self, count: u32) {
    if let Some(metrics) = self {
      metrics.increment_inscription_event_count(count);
    }
  }

  fn increment_brc20_event_count(&self, count: u32) {
    if let Some(metrics) = self {
      metrics.increment_brc20_event_count(count);
    }
  }

  fn observe_block_parse_duration(&self, duration: f64) {
    if let Some(metrics) = self {
      metrics.observe_block_parse_duration(duration);
    }
  }
}

pub(crate) fn setup_metrics() -> Metrics {
  let registry = Registry::new();

  let current_block_height = register_gauge(
    &registry,
    "current_block_height",
    "The latest block height that has been parsed.",
  );

  let total_transactions = register_gauge(
    &registry,
    "total_transactions",
    "The total number of transactions from blocks.",
  );

  let total_inscription_events = register_gauge(
    &registry,
    "total_inscription_events",
    "The total number of inscription events from blocks.",
  );

  let total_brc20_events = register_gauge(
    &registry,
    "total_brc20_events",
    "The total number of BRC20 events from blocks.",
  );

  let block_parse_duration = register_histogram(
    &registry,
    "block_parse_duration",
    "Histogram of block parsing duration in seconds.",
    vec![0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0],
  );

  Metrics {
    registry,
    current_block_height,
    total_transactions,
    total_inscription_events,
    total_brc20_events,
    block_parse_duration,
  }
}

fn register_gauge(registry: &Registry, name: &str, help: &str) -> Gauge {
  let gauge = Gauge::new(name, help).unwrap();
  registry.register(Box::new(gauge.clone())).unwrap();
  gauge
}

fn register_histogram(registry: &Registry, name: &str, help: &str, buckets: Vec<f64>) -> Histogram {
  let histogram = Histogram::with_opts(HistogramOpts::new(name, help).buckets(buckets)).unwrap();
  registry.register(Box::new(histogram.clone())).unwrap();
  histogram
}
