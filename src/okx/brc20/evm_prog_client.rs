use {
  alloy::primitives::FixedBytes,
  anyhow::{anyhow, Result},
  bitcoin::{
    hash_types::{BlockHash, Txid},
    hashes::Hash,
  },
  brc20_prog::{
    types::{AddressED, Base64Bytes, BlockResponseED, RawBytes, TraceED, TxReceiptED, B256ED},
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
    let client = HttpClientBuilder::new()
      .max_request_size(u32::MAX)
      .max_response_size(u32::MAX)
      .set_headers(auth_header)
      .build(url)?;
    BRC20_PROG_RUNTIME.block_on(async {
      client
        .brc20_clear_caches()
        .await
        .expect("Clearing BRC20 caches")
    });
    Ok(Self { client })
  }

  pub fn eth_block_number(&self) -> Result<u64> {
    let hex_str = BRC20_PROG_RUNTIME
      .block_on(async { self.client.eth_block_number().await })
      .map_err(|e| anyhow!("Failed to get block number: {}", e))?;

    let number_str = hex_str.trim_start_matches("0x");
    u64::from_str_radix(number_str, 16)
      .map_err(|e| anyhow!("Failed to parse block number '{}': {}", number_str, e))
  }

  pub fn brc20_initialise<T>(
    &self,
    genesis_block_hash: T,
    genesis_block_timestamp: u64,
    genesis_block_number: u64,
  ) -> Result<()>
  where
    T: Into<B256ED>,
  {
    BRC20_PROG_RUNTIME
      .block_on(async {
        self
          .client
          .brc20_initialise(
            genesis_block_hash.into(),
            genesis_block_timestamp,
            genesis_block_number,
          )
          .await
      })
      .map_err(|e| anyhow!("Failed to initialize BRC20: {}", e))
  }

  pub fn brc20_mine(&self, num_blocks: u64, timestamp: u64) -> Result<()> {
    BRC20_PROG_RUNTIME
      .block_on(async { self.client.brc20_mine(num_blocks, timestamp).await })
      .map_err(|e| anyhow!("Failed to mine blocks: {}", e))
  }

  pub fn brc20_commit_to_database(&self) -> Result<()> {
    BRC20_PROG_RUNTIME
      .block_on(async { self.client.brc20_commit_to_database().await })
      .map_err(|e| anyhow!("Failed to commit to database: {}", e))
  }

  pub fn brc20_reorg(&self, target_height: u64) -> Result<()> {
    BRC20_PROG_RUNTIME
      .block_on(async { self.client.brc20_reorg(target_height).await })
      .map_err(|e| anyhow!("Failed to reorg: {}", e))
  }

  pub fn brc20_finalise_block<T>(
    &self,
    timestamp: u64,
    block_hash: T,
    prog_tx_idx: u64,
  ) -> Result<()>
  where
    T: Into<B256ED>,
  {
    BRC20_PROG_RUNTIME
      .block_on(async {
        self
          .client
          .brc20_finalise_block(timestamp, block_hash.into(), prog_tx_idx)
          .await
      })
      .map_err(|e| anyhow!("Failed to finalize block: {}", e))
  }

  pub fn brc20_deposit<T>(
    &self,
    sender_address: String,
    ticker: String,
    amount: u128,
    block_timestamp: u64,
    block_hash: T,
    tx_idx: u64,
    inscription_id: String,
  ) -> Result<()>
  where
    T: Into<B256ED>,
  {
    BRC20_PROG_RUNTIME
      .block_on(async {
        self
          .client
          .brc20_deposit(
            sender_address,
            ticker,
            amount.into(),
            block_timestamp,
            block_hash.into(),
            tx_idx,
            inscription_id,
          )
          .await
          .map(|_| ()) // Ignore receipt, just check success
      })
      .map_err(|e| anyhow!("Failed to deposit: {}", e))
  }

  pub fn brc20_withdraw<T>(
    &self,
    sender_address: String,
    ticker: String,
    amount: u128,
    block_timestamp: u64,
    block_hash: T,
    tx_idx: u64,
    inscription_id: String,
  ) -> Result<TxReceiptED>
  where
    T: Into<B256ED>,
  {
    BRC20_PROG_RUNTIME
      .block_on(async {
        self
          .client
          .brc20_withdraw(
            sender_address,
            ticker,
            amount.into(),
            block_timestamp,
            block_hash.into(),
            tx_idx,
            inscription_id,
          )
          .await
      })
      .map_err(|e| anyhow!("Failed to withdraw: {}", e))
  }

  pub fn brc20_deploy<T>(
    &self,
    sender_address: String,
    bytecode: Option<RawBytes>,
    base64_bytecode: Option<Base64Bytes>,
    block_timestamp: u64,
    block_hash: T,
    tx_idx: u64,
    inscription_id: String,
    inscription_byte_length: u64,
    op_return_tx_id: T,
  ) -> Result<()>
  where
    T: Into<B256ED>,
  {
    BRC20_PROG_RUNTIME
      .block_on(async {
        self
          .client
          .brc20_deploy(
            sender_address,
            bytecode,
            base64_bytecode,
            block_timestamp,
            block_hash.into(),
            tx_idx,
            inscription_id,
            inscription_byte_length,
            op_return_tx_id.into(),
          )
          .await
          .map(|_| ()) // Convert to unit type
      })
      .map_err(|e| anyhow!("Failed to deploy module: {}", e))
  }

  pub fn brc20_call<T, U>(
    &self,
    sender_address: String,
    contract_address: Option<T>,
    contract_inscription_id: Option<String>,
    data: Option<RawBytes>,
    base64_data: Option<Base64Bytes>,
    block_timestamp: u64,
    block_hash: U,
    tx_idx: u64,
    inscription_id: String,
    inscription_byte_length: u64,
    op_return_tx_id: U,
  ) -> Result<()>
  where
    T: Into<AddressED>,
    U: Into<B256ED>,
  {
    BRC20_PROG_RUNTIME
      .block_on(async {
        self
          .client
          .brc20_call(
            sender_address,
            contract_address.map(|address| address.into()),
            contract_inscription_id,
            data,
            base64_data,
            block_timestamp,
            block_hash.into(),
            tx_idx,
            inscription_id,
            inscription_byte_length,
            op_return_tx_id.into(),
          )
          .await
          .map(|_| ()) // Ignore receipt
      })
      .map_err(|e| anyhow!("Failed to call module: {}", e))
  }

  pub fn brc20_transact<T>(
    &self,
    data: Option<RawBytes>,
    base64_data: Option<Base64Bytes>,
    block_timestamp: u64,
    block_hash: T,
    tx_idx: u64,
    inscription_id: String,
    inscription_byte_length: u64,
    op_return_tx_id: T,
  ) -> Result<Vec<TxReceiptED>>
  where
    T: Into<B256ED>,
  {
    BRC20_PROG_RUNTIME
      .block_on(async {
        self
          .client
          .brc20_transact(
            data,
            base64_data,
            block_timestamp,
            block_hash.into(),
            tx_idx,
            inscription_id,
            inscription_byte_length,
            op_return_tx_id.into(),
          )
          .await
      })
      .map_err(|e| anyhow!("Failed to transact: {}", e))
  }

  pub fn debug_trace_transaction<T>(&self, tx_idx: T) -> Result<Option<TraceED>>
  where
    T: Into<B256ED>,
  {
    BRC20_PROG_RUNTIME
      .block_on(async { self.client.debug_trace_transaction(tx_idx.into()).await })
      .map_err(|e| anyhow!("Failed to get trace: {}", e))
  }

  pub fn eth_get_block_by_number(
    &self,
    block_number: String,
    is_full: Option<bool>,
  ) -> Result<BlockResponseED> {
    BRC20_PROG_RUNTIME
      .block_on(async {
        self
          .client
          .eth_get_block_by_number(block_number, is_full)
          .await
      })
      .map_err(|e| anyhow!("Failed to get block: {}", e))
  }
}

pub trait ToEvmHash {
  fn to_evm_hash(&self) -> FixedBytes<32>;

  fn to_b256_ed(&self) -> B256ED {
    self.to_evm_hash().into()
  }
}

impl ToEvmHash for BlockHash {
  fn to_evm_hash(&self) -> FixedBytes<32> {
    let reversed_bytes = self
      .as_byte_array()
      .iter()
      .rev()
      .copied()
      .collect::<Vec<u8>>();
    FixedBytes::<32>::from_slice(&reversed_bytes)
  }
}

impl ToEvmHash for Txid {
  fn to_evm_hash(&self) -> FixedBytes<32> {
    let reversed_bytes = self
      .as_byte_array()
      .iter()
      .rev()
      .copied()
      .collect::<Vec<u8>>();
    FixedBytes::<32>::from_slice(&reversed_bytes)
  }
}
