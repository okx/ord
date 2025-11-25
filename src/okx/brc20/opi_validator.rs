use {
  super::event_hash::EVENT_SEPARATOR,
  crate::{
    chain::Chain,
    okx::brc20::{event_hash::BRC20BlockEventHash, evm_prog_client::Brc20ProgClient},
  },
  anyhow::{anyhow, Result},
  once_cell::sync::Lazy,
  std::{
    error::Error,
    fmt::{self, Display, Formatter},
    thread,
    time::Duration,
  },
};

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

pub enum OpiValidationMode {
  Strict,
  Relaxed,
  None,
}

impl Display for OpiValidationMode {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      OpiValidationMode::Strict => write!(f, "Strict"),
      OpiValidationMode::Relaxed => write!(f, "Relaxed"),
      OpiValidationMode::None => write!(f, "None"),
    }
  }
}

pub struct OpiValidator {
  height: u64,
  first_brc20_prog_height: u64,
  validation_mode: OpiValidationMode,
  chain: Chain,
}

impl OpiValidator {
  pub fn new(
    height: u64,
    first_brc20_prog_height: u64,
    validation_mode: OpiValidationMode,
    chain: Chain,
  ) -> Self {
    Self {
      height,
      first_brc20_prog_height,
      validation_mode,
      chain,
    }
  }

  pub fn validate(
    &self,
    brc20_event_hasher: &BRC20BlockEventHash,
    brc20_prog_client: &Brc20ProgClient,
  ) -> Result<()> {
    if matches!(self.validation_mode, OpiValidationMode::None) {
      return Ok(());
    }

    log::info!(
      "[OPI] Starting validation for block {} in {} mode",
      self.height,
      self.validation_mode,
    );

    // Get previous cumulative hashes
    let prev_hashes = match self.get_opi_cumulative_hashes(self.height - 1) {
      Ok(hash) => hash,
      Err(e) => {
        return self.handle_error(
          format!(
            "Failed to get previous OPI cumulative event hash at block {}: {}",
            self.height - 1,
            e
          ),
          "Failed to get previous OPI cumulative event hash",
        );
      }
    };

    // Calculate current cumulative event hash
    let event_hash = brc20_event_hasher.get_block_event_hash();
    let cumulative_event_hash = if prev_hashes.event_hash.is_empty() {
      event_hash.clone()
    } else {
      sha256::digest(prev_hashes.event_hash + &event_hash)
    };

    // Calculate trace hash (only from first BRC20 prog height)
    let trace_hash = if self.height >= self.first_brc20_prog_height {
      match self.calculate_trace_hash(brc20_prog_client) {
        Ok(hash) => hash,
        Err(e) => {
          return self.handle_error(
            format!(
              "Failed to calculate BRC20 prog traces hash at block {}: {}",
              self.height, e
            ),
            "Failed to calculate BRC20 prog traces hash",
          );
        }
      }
    } else {
      String::new()
    };

    let cumulative_trace_hash = if prev_hashes.trace_hash.is_empty() {
      sha256::digest(trace_hash)
    } else {
      sha256::digest(prev_hashes.trace_hash + &trace_hash)
    };

    // Get current cumulative hashes from OPI
    let current_hashes = match self.get_opi_cumulative_hashes(self.height) {
      Ok(hash) => hash,
      Err(e) => {
        return self.handle_error(
          format!(
            "Failed to get current OPI cumulative event hash at block {}: {}",
            self.height, e
          ),
          "Failed to get current OPI cumulative event hash",
        );
      }
    };

    // Validate event hash
    if !current_hashes.event_hash.is_empty() && cumulative_event_hash != current_hashes.event_hash {
      return self.handle_error(
        format!(
          "BRC20 Block Event Hash mismatch at block {}: computed {}, stored {}",
          self.height, cumulative_event_hash, current_hashes.event_hash
        ),
        "BRC20 Block Event Hash mismatch",
      );
    }

    log::info!(
      "[OPI] BRC20 Block Event Hash for block {}: {}",
      self.height,
      event_hash
    );

    // Validate trace hash
    if self.height >= self.first_brc20_prog_height
      && !current_hashes.trace_hash.is_empty()
      && cumulative_trace_hash != current_hashes.trace_hash
    {
      return self.handle_error(
        format!(
          "BRC20 Trace Hash mismatch at block {}: computed {}, stored {}",
          self.height, cumulative_trace_hash, current_hashes.trace_hash
        ),
        "BRC20 Trace Hash mismatch",
      );
    }

    log::info!("[OPI] Block {} validation passed successfully", self.height);
    Ok(())
  }

  /// Get OPI cumulative hashes with retry logic (synchronous)
  fn get_opi_cumulative_hashes(
    &self,
    block_height: u64,
  ) -> Result<OpiCumulativeHashes, Box<dyn Error>> {
    const RETRIES: u8 = 10;
    const DELAY_MS: u64 = 1000;

    for attempt in 0..RETRIES {
      match self.fetch_opi_hashes(block_height) {
        Ok(hash) => return Ok(hash),
        Err(e) => {
          if attempt < RETRIES - 1 {
            log::warn!(
              "[OPI] Attempt {} to get cumulative hashes for block {} failed: {}. Retrying...",
              attempt + 1,
              block_height,
              e
            );
            thread::sleep(Duration::from_millis(DELAY_MS * (attempt as u64 + 1)));
          }
        }
      }
    }

    Err("Failed to retrieve OPI cumulative hashes after retries".into())
  }

  /// Fetch OPI hashes from API (synchronous blocking call)
  fn fetch_opi_hashes(&self, block_height: u64) -> Result<OpiCumulativeHashes, Box<dyn Error>> {
    let network_type = match self.chain {
      Chain::Mainnet => "mainnet",
      Chain::Signet => "signet",
      _ => "testnet",
    };
    let url = format!(
      "http://{}/lc/get_best_hashes_for_block/{}?event_hash_version=2&network_type={}",
      option_env!("OPI_API_URL").unwrap_or("api.opi.network"),
      block_height,
      network_type
    );

    let response = OPI_CLIENT
      .get(&url)
      .send()
      .map_err(|e| format!("Failed to send request: {}", e))?;

    let json_value: serde_json::Value = response
      .json()
      .map_err(|e| format!("Failed to parse JSON: {}", e))?;

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

  /// Calculate BRC20 prog traces hash (synchronous)
  fn calculate_trace_hash(
    &self,
    brc20_prog_client: &Brc20ProgClient,
  ) -> Result<String, Box<dyn Error>> {
    let mut traces_hash_str = String::new();

    let block =
      brc20_prog_client.eth_get_block_by_number(format!("{}", self.height), Some(true))?;

    if block.transactions.is_left() {
      if block.transactions.left().unwrap_or_default().is_empty() {
        log::debug!("[OPI] No traces in block {}", self.height);
      } else {
        return Err(format!("Unexpected transaction format in block {}", self.height).into());
      }
    } else if let Some(mut txes) = block.transactions.right() {
      txes.sort_by_key(|tx| tx.transaction_index);

      for tx in txes {
        match brc20_prog_client.debug_trace_transaction(tx.hash) {
          Ok(Some(trace)) => {
            let trace_hash_str = serde_json_canonicalizer::to_string(&trace)?;
            traces_hash_str.push_str(&trace_hash_str);
            traces_hash_str.push_str(EVENT_SEPARATOR);
          }
          Ok(None) => {
            log::warn!(
              "[OPI] No trace found for transaction {:?} in block {}",
              tx.hash,
              self.height
            );
            continue;
          }
          Err(e) => {
            log::warn!(
              "[OPI] Error getting trace for transaction {:?} in block {}: {}",
              tx.hash,
              self.height,
              e
            );
            continue;
          }
        }
      }
    }

    log::debug!(
      "[OPI] Calculated traces for block {}: {}",
      self.height,
      traces_hash_str
    );

    Ok(sha256::digest(
      traces_hash_str
        .trim_end_matches(EVENT_SEPARATOR)
        .to_string(),
    ))
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
