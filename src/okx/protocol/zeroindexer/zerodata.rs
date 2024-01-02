use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct ZeroData {
  pub block_height: u64,
  pub block_hash: String,
  pub prev_block_hash: String,
  pub block_time: u32,
  pub txs: Vec<ZeroIndexerTx>,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct ZeroIndexerTx {
  pub protocol_name: String,
  pub inscription: String,
  pub inscription_context: String,
  pub btc_txid: String,
  pub btc_fee: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InscriptionContext {
  pub txid: String,
  pub inscription_id: String,
  pub inscription_number: i64,
  pub old_sat_point: String,
  pub new_sat_point: String,
  pub sender: String,
  pub receiver: String,
  pub is_transfer: bool,
  pub block_height: u64,
  pub block_time: u32,
  pub block_hash: String,
}
