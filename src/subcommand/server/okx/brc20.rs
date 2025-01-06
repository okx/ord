use super::*;
use crate::okx::brc20::{BRC20Balance, BRC20Ticker, BRC20TickerInfo, BRC20TransferAsset};
mod assets;
mod balance;
mod outpoint;
mod receipt;
mod ticker_info;

pub(crate) use self::{
  assets::{brc20_all_transferable, brc20_transferable},
  balance::{brc20_all_balance, brc20_balance},
  outpoint::brc20_outpoint,
  receipt::{brc20_block_events, brc20_tx_events},
  ticker_info::{brc20_all_tick_info, brc20_tick_info},
};
#[derive(Debug, thiserror::Error)]
pub(super) enum BRC20ApiError {
  #[error("Unknown ticker '{0}': failed to retrieve ticker from the database.")]
  UnknownTicker(String),
  #[error("Transaction receipt for Txid '{0}' not found: no matching receipt exists.")]
  TransactionReceiptNotFound(Txid),
  #[error("Block receipt for hash '{0}' not found: no matching receipt exists.")]
  BlockReceiptNotFound(BlockHash),
  #[error("Conflict detected for block at height '{0}'")]
  ConflictBlockByHeight(Height),
  #[error("Database is not ready: the database is not yet initialized.")]
  DataBaseNotReady,
}

impl From<BRC20ApiError> for ApiError {
  fn from(error: BRC20ApiError) -> Self {
    match error {
      BRC20ApiError::UnknownTicker(_) => Self::not_found(error.to_string()),
      BRC20ApiError::TransactionReceiptNotFound(_) => Self::not_found(error.to_string()),
      BRC20ApiError::BlockReceiptNotFound(_) => Self::not_found(error.to_string()),
      BRC20ApiError::ConflictBlockByHeight(_) => Self::internal(error.to_string()),
      BRC20ApiError::DataBaseNotReady => Self::internal(error.to_string()),
    }
  }
}
