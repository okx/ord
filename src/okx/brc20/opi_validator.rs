use {
  crate::{
    chain::Chain,
    okx::{
      brc20::{entry::OpiBlockValidation, evm_prog_client::Brc20ProgClient},
      context::TableContext,
    },
  },
  anyhow::{anyhow, bail, Result},
  bitcoin::BlockHash,
  chrono::Utc,
  once_cell::sync::Lazy,
  std::{str::FromStr, thread, time::Duration},
};

const RECENT_BLOCKS_TIME_WINDOW: i64 = 60 * 60 * 24; // 1 day

lazy_static::lazy_static! {
  static ref CHECKPOINT_INTERVAL: u32 = option_env!("CHECKPOINT_INTERVAL")
    .and_then(|s| s.parse::<u32>().ok())
    .unwrap_or(1000);
}

static OPI_CLIENT: Lazy<reqwest::blocking::Client> = Lazy::new(|| {
  reqwest::blocking::Client::builder()
    .timeout(Duration::from_secs(30))
    .build()
    .expect("Failed to create OPI HTTP client")
});

#[derive(Debug, Clone)]
pub struct OpiCumulativeHashes {
  pub event_hash: String,
  pub trace_hash: String,
}

#[derive(Debug, Clone, Copy, strum_macros::Display)]
pub enum OpiValidationMode {
  /// Strict mode will return an error if the cumulative hashes do not match.
  Strict,
  /// Relaxed mode will only log a warning if the cumulative hashes do not match.
  Relaxed,
  /// None mode will not validate the cumulative hashes.
  None,
}

pub struct OpiValidator<'a, 't: 'a, 'txn: 'a> {
  chain: Chain,
  validation_mode: OpiValidationMode,
  context: &'a mut TableContext<'t, 'txn>,
  brc20_prog_client: &'a Brc20ProgClient,
}

impl<'a, 't: 'a, 'txn: 'a> OpiValidator<'a, 't, 'txn> {
  pub fn new(
    chain: Chain,
    context: &'a mut TableContext<'t, 'txn>,
    validation_mode: OpiValidationMode,
    brc20_prog_client: &'a Brc20ProgClient,
  ) -> Self {
    Self {
      chain,
      context,
      validation_mode,
      brc20_prog_client,
    }
  }

  pub fn validate(
    &mut self,
    height: u32,
    block_hash: &BlockHash,
    block_timestamp: u32,
    brc20_block_event_hash: String,
  ) -> Result<Option<OpiBlockValidation>> {
    if height < self.chain.first_inscription_height()
      || matches!(self.validation_mode, OpiValidationMode::None)
    {
      return Ok(None);
    }
    let previous_block_in_database = self.context.get_opi_block_validations(height - 1)?;
    let previous_block = match previous_block_in_database {
      Some(previous) if matches!(self.validation_mode, OpiValidationMode::Strict) => previous,
      None
        if matches!(self.validation_mode, OpiValidationMode::Strict)
          && height > self.chain.first_inscription_height() =>
      {
        bail!(
          "Previous OPI block validation data not found for block {} in strict mode",
          height - 1
        );
      }
      _ => {
        // Use OPI stored trace/event hash if it's not in database (e.g., during initial sync, or an update) or not strict mode
        // This allows non-strict validation to continue without halting completely
        let previous_opi_cumulative_hashes = self.get_opi_cumulative_hashes(height - 1)?;
        OpiBlockValidation {
          block_hash: BlockHash::from_str(
            "0000000000000000000000000000000000000000000000000000000000000000",
          )
          .unwrap(),
          block_timestamp: 0,
          brc20_block_event_hash: String::new(),
          brc20_cumulative_event_hash: previous_opi_cumulative_hashes.event_hash,
          brc20_prog_block_trace_hash: None,
          brc20_cumulative_trace_hash: Some(previous_opi_cumulative_hashes.trace_hash),
        }
      }
    };

    // Calculate current cumulative event hash
    let current_cumulative_event_hash = if previous_block.brc20_cumulative_event_hash.as_str() != ""
    {
      sha256::digest(
        previous_block.brc20_cumulative_event_hash.to_owned() + &brc20_block_event_hash,
      )
    } else {
      brc20_block_event_hash.clone()
    };

    // Calculate current cumulative trace hash
    let (current_trace_hash, current_cumulative_trace_hash) =
      if height >= self.chain.first_brc20_prog_height() {
        let Ok(trace_hash) = retrieve_trace_hash(height, self.brc20_prog_client) else {
          bail!("BRC20 block trace hash is required for block {}", height);
        };

        let cumulative_trace_hash = sha256::digest(
          previous_block
            .brc20_cumulative_trace_hash
            .as_deref()
            .unwrap_or_default()
            .to_owned()
            + &trace_hash,
        );

        tracing::debug!(
          "[OPI] Block {}: current_trace_hash: {}, current_cumulative_trace_hash: {}",
          height,
          trace_hash,
          cumulative_trace_hash
        );

        (Some(trace_hash), Some(cumulative_trace_hash))
      } else {
        (None, None)
      };

    log::debug!(
      "[OPI] Validating block {}: previous_cumulative_event_hash: {}, previous_cumulative_trace_hash: {}, brc20_block_event_hash: {}, brc20_block_trace_hash: {}",
      height,
      previous_block.brc20_cumulative_event_hash.as_str(),
      brc20_block_event_hash,
      previous_block.brc20_cumulative_trace_hash.as_deref().unwrap_or("null"),
      current_trace_hash.as_deref().unwrap_or("null")
    );

    // Validation strategy:
    // - For recent blocks (within the last 24 hours): validate every height against OPI.
    // - For historical sync (older than 24 hours): validate every HISTORICAL_CHECKPOINT_INTERVAL blocks as checkpoints.
    if (Utc::now().timestamp() - block_timestamp as i64) < RECENT_BLOCKS_TIME_WINDOW
      || height % *CHECKPOINT_INTERVAL == 0
    {
      // Get current cumulative hashes from OPI
      let opi_cumulative_hashes = match self.get_opi_cumulative_hashes(height) {
        Ok(hash) => hash,
        Err(e) => {
          return self
            .handle_error(
              format!(
                "Failed to get current OPI cumulative event hash at block {}: {}",
                height, e
              ),
              "Failed to get current OPI cumulative event hash",
            )
            .map(|_| None);
        }
      };

      // Validate event hash
      if !opi_cumulative_hashes.event_hash.is_empty()
        && current_cumulative_event_hash != opi_cumulative_hashes.event_hash
      {
        return self
          .handle_error(
            format!(
              "BRC20 Block Event Hash mismatch at block {}: computed {}, stored {}",
              height, current_cumulative_event_hash, opi_cumulative_hashes.event_hash
            ),
            "BRC20 Block Event Hash mismatch",
          )
          .map(|_| None);
      }

      // Validate trace hash
      if let Some(trace_hash) = &current_cumulative_trace_hash {
        if trace_hash.to_owned() != opi_cumulative_hashes.trace_hash {
          return self
            .handle_error(
              format!(
                "BRC20 Trace Hash mismatch at block {}: computed {}, stored {}",
                height, trace_hash, opi_cumulative_hashes.trace_hash
              ),
              "BRC20 Trace Hash mismatch",
            )
            .map(|_| None);
        }
      }
      log::info!("[OPI] Block {} validation passed successfully. current_cumulative_event_hash: {}, current_cumulative_trace_hash: {}", height, current_cumulative_event_hash, current_cumulative_trace_hash.as_deref().unwrap_or("null"));
    }
    Ok(Some(OpiBlockValidation {
      block_hash: block_hash.clone(),
      block_timestamp,
      brc20_block_event_hash,
      brc20_cumulative_event_hash: current_cumulative_event_hash,
      brc20_prog_block_trace_hash: current_trace_hash,
      brc20_cumulative_trace_hash: current_cumulative_trace_hash,
    }))
  }

  /// Get OPI cumulative hashes with retry logic (synchronous)
  fn get_opi_cumulative_hashes(&self, block_height: u32) -> Result<OpiCumulativeHashes> {
    const RETRIES: u8 = 10;
    const DELAY_MS: u64 = 1000;
    for attempt in 0..RETRIES {
      match self.fetch_opi_hashes(block_height) {
        Ok(hash) => return Ok(hash),
        Err(e) => {
          if attempt < RETRIES - 1 {
            log::warn!(
              "[OPI] Attempt {} to get cumulative hashes for block {} failed: {:#}. Retrying...",
              attempt + 1,
              block_height,
              e
            );
            thread::sleep(Duration::from_millis(DELAY_MS * (attempt as u64 + 1)));
          }
        }
      }
    }

    Err(anyhow!(
      "Failed to retrieve OPI cumulative hashes after retries"
    ))
  }

  /// Fetch OPI hashes from API (synchronous blocking call)
  fn fetch_opi_hashes(&self, block_height: u32) -> Result<OpiCumulativeHashes> {
    let network_type = match self.chain {
      Chain::Mainnet => "mainnet",
      Chain::Signet => "signet",
      _ => "testnet",
    };
    let url = format!(
      "{}/lc/get_best_hashes_for_block/{}?event_hash_version=3&network_type={}",
      option_env!("OPI_API_URL").unwrap_or("https://api.opi.network"),
      block_height,
      network_type
    );

    let response = OPI_CLIENT
      .get(&url)
      .send()
      .map_err(|e| anyhow!("Failed to send request: {:#}", e))?;

    let json_value: serde_json::Value = response
      .json()
      .map_err(|e| anyhow!("Failed to parse JSON: {:#}", e))?;

    let mut event_hash = String::new();
    let mut trace_hash = String::new();

    if let Some(hash) = json_value
      .get("data")
      .and_then(|data| data.get("best_cumulative_hash"))
      .and_then(|h| h.as_str())
    {
      event_hash = hash.to_string();
    }

    if let Some(hash) = json_value
      .get("data")
      .and_then(|data| data.get("best_cumulative_trace_hash"))
      .and_then(|h| h.as_str())
    {
      trace_hash = hash.to_string();
    }

    Ok(OpiCumulativeHashes {
      event_hash,
      trace_hash,
    })
  }

  /// Handle validation error based on validation mode
  fn handle_error(&self, log_message: String, panic_message: &'static str) -> Result<()> {
    log::error!("[OPI] {}", log_message);

    match self.validation_mode {
      OpiValidationMode::Strict => Err(anyhow!(panic_message)),
      OpiValidationMode::Relaxed | OpiValidationMode::None => Ok(()),
    }
  }
}

/// Calculate BRC20 prog traces hash (synchronous)
fn retrieve_trace_hash(height: u32, brc20_prog_client: &Brc20ProgClient) -> Result<String> {
  let Ok(Some(block_trace_hash)) = brc20_prog_client.debug_get_block_trace_hash(height) else {
    return Err(anyhow!(
      "BRC20 prog block trace hash not found for block {}",
      height
    ));
  };
  Ok(block_trace_hash)
}
