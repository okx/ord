use {
  alloy::primitives::FixedBytes,
  anyhow::{anyhow, Result},
  bitcoin::{
    hash_types::{BlockHash, Txid},
    hashes::Hash,
  },
  brc20_prog::{
    types::{AddressED, Base64Bytes, RawBytes, TxReceiptED, B256ED},
    Brc20ProgApiClient,
  },
  jsonrpsee::http_client::{HeaderMap, HttpClient, HttpClientBuilder},
  once_cell::sync::Lazy,
  tokio::runtime::Runtime,
};

static BRC20_PROG_RUNTIME: Lazy<Runtime> = Lazy::new(|| {
  tokio::runtime::Builder::new_multi_thread()
    .thread_name("brc20-prog-worker")
    .enable_all()
    .build()
    .expect("Failed to create BRC20 Prog runtime")
});
pub(crate) struct Brc20ProgClient {
  client: HttpClient,
}

impl Brc20ProgClient {
  pub fn new(auth_header: HeaderMap, url: &str) -> Result<Self> {
    let client: Brc20ProgClient = Self {
      client: HttpClientBuilder::new()
        .max_request_size(u32::MAX)
        .max_response_size(u32::MAX)
        .set_headers(auth_header)
        .build(url)?,
    };

    client.brc20_clear_caches()?;
    Ok(client)
  }

  pub fn brc20_clear_caches(&self) -> Result<()> {
    BRC20_PROG_RUNTIME
      .block_on(async { self.client.brc20_clear_caches().await })
      .map_err(anyhow::Error::from)
  }

  pub fn eth_block_number(&self) -> Result<u64> {
    let hex_str = BRC20_PROG_RUNTIME
      .block_on(async { self.client.eth_block_number().await })
      .map_err(anyhow::Error::from)?;

    let number_str = hex_str.trim_start_matches("0x");
    u64::from_str_radix(number_str, 16)
      .map_err(|e| anyhow!("Failed to parse block number '{}': {}", number_str, e))
  }

  pub fn brc20_initialise(
    &self,
    genesis_block_hash: B256ED,
    genesis_block_timestamp: u64,
    genesis_block_number: u64,
  ) -> Result<()> {
    BRC20_PROG_RUNTIME
      .block_on(async {
        self
          .client
          .brc20_initialise(
            genesis_block_hash,
            genesis_block_timestamp,
            genesis_block_number,
          )
          .await
      })
      .map_err(anyhow::Error::from)
  }

  pub fn brc20_mine(&self, num_blocks: u64, timestamp: u64) -> Result<()> {
    BRC20_PROG_RUNTIME
      .block_on(async { self.client.brc20_mine(num_blocks, timestamp).await })
      .map_err(anyhow::Error::from)
  }

  pub fn brc20_commit_to_database(&self) -> Result<()> {
    BRC20_PROG_RUNTIME
      .block_on(async { self.client.brc20_commit_to_database().await })
      .map_err(anyhow::Error::from)
  }

  pub fn brc20_reorg(&self, target_height: u64) -> Result<()> {
    BRC20_PROG_RUNTIME
      .block_on(async { self.client.brc20_reorg(target_height).await })
      .map_err(anyhow::Error::from)
  }

  pub fn brc20_finalise_block(
    &self,
    timestamp: u64,
    block_hash: B256ED,
    prog_tx_idx: u64,
  ) -> Result<()> {
    BRC20_PROG_RUNTIME
      .block_on(async {
        self
          .client
          .brc20_finalise_block(timestamp, block_hash, prog_tx_idx)
          .await
      })
      .map_err(anyhow::Error::from)
  }

  pub fn brc20_deposit(
    &self,
    sender_address: String,
    ticker: String,
    amount: u128,
    block_timestamp: u64,
    block_hash: B256ED,
    tx_idx: u64,
    inscription_id: String,
  ) -> Result<()> {
    BRC20_PROG_RUNTIME
      .block_on(async {
        self
          .client
          .brc20_deposit(
            sender_address,
            ticker,
            amount.into(),
            block_timestamp,
            block_hash,
            tx_idx,
            inscription_id,
          )
          .await
          .map(|_| ()) // Ignore receipt, just check success
      })
      .map_err(anyhow::Error::from)
  }

  pub fn brc20_withdraw(
    &self,
    sender_address: String,
    ticker: String,
    amount: u128,
    block_timestamp: u64,
    block_hash: B256ED,
    tx_idx: u64,
    inscription_id: String,
  ) -> Result<TxReceiptED> {
    BRC20_PROG_RUNTIME
      .block_on(async {
        self
          .client
          .brc20_withdraw(
            sender_address,
            ticker,
            amount.into(),
            block_timestamp,
            block_hash,
            tx_idx,
            inscription_id,
          )
          .await
      })
      .map_err(anyhow::Error::from)
  }

  pub fn brc20_deploy(
    &self,
    sender_address: String,
    bytecode: Option<RawBytes>,
    base64_bytecode: Option<Base64Bytes>,
    block_timestamp: u64,
    block_hash: B256ED,
    tx_idx: u64,
    inscription_id: String,
    inscription_byte_length: u64,
    op_return_tx_id: B256ED,
  ) -> Result<()> {
    BRC20_PROG_RUNTIME
      .block_on(async {
        self
          .client
          .brc20_deploy(
            sender_address,
            bytecode,
            base64_bytecode,
            block_timestamp,
            block_hash,
            tx_idx,
            inscription_id,
            inscription_byte_length,
            op_return_tx_id,
          )
          .await
          .map(|_| ()) // Convert to unit type
      })
      .map_err(anyhow::Error::from)
  }

  pub fn brc20_call(
    &self,
    sender_address: String,
    contract_address: Option<AddressED>,
    contract_inscription_id: Option<String>,
    data: Option<RawBytes>,
    base64_data: Option<Base64Bytes>,
    block_timestamp: u64,
    block_hash: B256ED,
    tx_idx: u64,
    inscription_id: String,
    inscription_byte_length: u64,
    op_return_tx_id: B256ED,
  ) -> Result<()> {
    BRC20_PROG_RUNTIME
      .block_on(async {
        self
          .client
          .brc20_call(
            sender_address,
            contract_address,
            contract_inscription_id,
            data,
            base64_data,
            block_timestamp,
            block_hash,
            tx_idx,
            inscription_id,
            inscription_byte_length,
            op_return_tx_id,
          )
          .await
          .map(|_| ()) // Ignore receipt
      })
      .map_err(anyhow::Error::from)
  }

  pub fn brc20_transact(
    &self,
    data: Option<RawBytes>,
    base64_data: Option<Base64Bytes>,
    block_timestamp: u64,
    block_hash: B256ED,
    tx_idx: u64,
    inscription_id: String,
    inscription_byte_length: u64,
    op_return_tx_id: B256ED,
  ) -> Result<Vec<TxReceiptED>> {
    BRC20_PROG_RUNTIME
      .block_on(async {
        self
          .client
          .brc20_transact(
            data,
            base64_data,
            block_timestamp,
            block_hash,
            tx_idx,
            inscription_id,
            inscription_byte_length,
            op_return_tx_id,
          )
          .await
      })
      .map_err(anyhow::Error::from)
  }

  pub fn debug_get_block_trace_hash(&self, height: u32) -> Result<Option<String>> {
    BRC20_PROG_RUNTIME
      .block_on(async { self.client.debug_get_block_trace_hash(height.to_string()).await })
      .map_err(anyhow::Error::from)
  }
}

pub trait ToB256ED {
  fn to_b256_ed(&self) -> B256ED;
}

impl ToB256ED for BlockHash {
  fn to_b256_ed(&self) -> B256ED {
    let reversed_bytes = self
      .as_byte_array()
      .iter()
      .rev()
      .copied()
      .collect::<Vec<u8>>();
    FixedBytes::<32>::from_slice(&reversed_bytes).into()
  }
}

impl ToB256ED for Txid {
  fn to_b256_ed(&self) -> B256ED {
    let reversed_bytes = self
      .as_byte_array()
      .iter()
      .rev()
      .copied()
      .collect::<Vec<u8>>();
    FixedBytes::<32>::from_slice(&reversed_bytes).into()
  }
}
