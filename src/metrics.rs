use {std::time::Duration, strum_macros::AsRefStr};

/// Metric name prefix
const METRIC_PREFIX: &str = "ord";

/// Metric names
mod metric_names {
  pub const BLOCK_HEIGHT: &str = "block_height";
  pub const BLOCK_DOWNLOAD_DURATION: &str = "block_download_duration_seconds";
  pub const BLOCK_PHASE_DURATION: &str = "block_phase_duration_seconds";
  pub const BLOCK_STATS: &str = "block_stats";
  pub const DB_COMMIT_DURATION: &str = "db_commit_duration_seconds";
  pub const API_REQUEST_COUNT: &str = "api_request_total";
  pub const API_REQUEST_DURATION: &str = "api_request_duration_seconds";
}

/// Label names
mod label_names {
  // Block-related labels
  pub const HEIGHT_STATE: &str = "height_state";
  pub const INDEXING_PHASE: &str = "indexing_phase";
  pub const STAT_TYPE: &str = "stat_type";

  // HTTP/API-related labels
  pub const HTTP_METHOD: &str = "http_method";
  pub const HTTP_PATH: &str = "http_path";
  pub const HTTP_STATUS_CODE: &str = "http_status_code";
}

/// Block height state at different stages of processing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum BlockHeightState {
  /// Latest block height from connected Bitcoin node
  Network,
  /// Current block height being processed by ord indexer
  Processed,
  /// Latest block height committed to database
  DbCommitted,
}

/// Block indexing phase types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum IndexingPhase {
  /// Time waiting for async block download to complete
  BlockWait,
  /// Time spent indexing ord inscriptions
  InscriptionIndexing,
  /// Time spent indexing inscription receipts
  InscriptionReceiptsIndexing,
  /// Time spent indexing BRC20 tokens
  Brc20Indexing,
  /// Time spent indexing bitmap collection
  BitmapIndexing,
  /// Time spent indexing BTC domain collection
  BtcDomainIndexing,
  /// Total time spent on all OKX extended content indexing
  OkxTotalIndexing,
}

/// Block statistics categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, AsRefStr)]
#[strum(serialize_all = "snake_case")]
pub enum BlockStatistic {
  /// Number of transactions in block
  Transactions,
  /// Number of inscriptions in block
  Inscriptions,
  /// Number of BRC20 events in block
  Brc20Events,
  /// Number of bitmaps in block
  Bitmaps,
  /// Number of BTC domains in block
  BtcDomains,
}

/// Build full metric name with prefix
#[inline]
fn metric_name(name: &str) -> String {
  format!("{}_{}", METRIC_PREFIX, name)
}

/// Record block height at specific stage
///
/// # Arguments
/// * `state` - State of height being recorded (network/processed/db_committed)
/// * `height` - Block height value
///
/// # Metric
/// - Name: `ord_block_height`
/// - Type: Gauge
/// - Labels: `height_state`
#[inline]
pub fn record_height(state: BlockHeightState, height: u64) {
  let state_str = state.as_ref().to_string();
  metrics::gauge!(
    metric_name(metric_names::BLOCK_HEIGHT),
    label_names::HEIGHT_STATE => state_str
  )
  .set(height as f64);
}

/// Record block download duration from Bitcoin node
///
/// # Arguments
/// * `duration` - Time spent downloading the block
///
/// # Metric
/// - Name: `ord_block_download_duration_seconds`
/// - Type: Histogram
/// - Unit: seconds
#[inline]
pub fn record_download(duration: Duration) {
  metrics::histogram!(metric_name(metric_names::BLOCK_DOWNLOAD_DURATION))
    .record(duration.as_secs_f64());
}

/// Record block indexing phase duration
///
/// # Arguments
/// * `phase` - Indexing phase (inscription_indexing, brc20_indexing, etc.)
/// * `duration` - Time spent in this phase
///
/// # Metric
/// - Name: `ord_block_phase_duration_seconds`
/// - Type: Histogram
/// - Unit: seconds
/// - Labels: `indexing_phase`
#[inline]
pub fn record_phase(phase: IndexingPhase, duration: Duration) {
  let phase_str = phase.as_ref().to_string();
  metrics::histogram!(
    metric_name(metric_names::BLOCK_PHASE_DURATION),
    label_names::INDEXING_PHASE => phase_str
  )
  .record(duration.as_secs_f64());
}

/// Record database commit duration
///
/// # Arguments
/// * `duration` - Time spent committing to database
///
/// # Metric
/// - Name: `ord_db_commit_duration_seconds`
/// - Type: Histogram
/// - Unit: seconds
#[inline]
pub fn record_commit(duration: Duration) {
  metrics::histogram!(metric_name(metric_names::DB_COMMIT_DURATION)).record(duration.as_secs_f64());
}

/// Record block statistics
///
/// # Arguments
/// * `statistic` - Category of statistic (transactions, inscriptions, etc.)
/// * `count` - Number of items
///
/// # Metric
/// - Name: `ord_block_stats`
/// - Type: Gauge
/// - Labels: `stat_type`
#[inline]
pub fn record_stats(statistic: BlockStatistic, count: u64) {
  let stats_str = statistic.as_ref().to_string();
  metrics::gauge!(
    metric_name(metric_names::BLOCK_STATS),
    label_names::STAT_TYPE => stats_str
  )
  .set(count as f64);
}

/// Record API request metrics
///
/// # Arguments
/// * `method` - HTTP method (GET, POST, etc.)
/// * `path` - Route template (e.g., "/api/v1/ord/id/:id")
/// * `status_code` - HTTP status code (200, 404, etc.)
/// * `duration_secs` - Request duration in seconds
///
/// # Metrics
/// - Name: `ord_api_request_total` (Counter)
/// - Name: `ord_api_request_duration_seconds` (Histogram)
/// - Labels: `method`, `path`, `status_code`
/// - Unit: seconds (for duration)
///
/// # Note
/// Uses route templates (not actual paths) to avoid high cardinality issues.
#[inline]
pub fn record_api_request(method: &str, path: &str, status_code: u16, duration_secs: f64) {
  let method_str = method.to_string();
  let path_str = path.to_string();
  let status_code_str = status_code.to_string();

  // Record request count
  metrics::counter!(
    metric_name(metric_names::API_REQUEST_COUNT),
    label_names::HTTP_METHOD => method_str.clone(),
    label_names::HTTP_PATH => path_str.clone(),
    label_names::HTTP_STATUS_CODE => status_code_str.clone()
  )
  .increment(1);

  // Record request duration
  metrics::histogram!(
    metric_name(metric_names::API_REQUEST_DURATION),
    label_names::HTTP_METHOD => method_str,
    label_names::HTTP_PATH => path_str,
    label_names::HTTP_STATUS_CODE => status_code_str
  )
  .record(duration_secs);
}

/// Helper macro to create metric labels from tuples
///
/// This ensures type safety and consistency in label handling.
#[macro_export]
#[doc(hidden)]
macro_rules! __metrics_labels {
  ($($key:expr => $value:expr),* $(,)?) => {
    &[$(($key, $value.to_string())),*]
  };
}
