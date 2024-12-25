use super::*;
use crate::okx::brc20::{BRC20Ticker, BRC20TickerInfo, BRC20TransferAsset};
mod assets;
mod balance;
mod receipt;
mod ticker_info;
mod outpoint;

pub(crate) use self::{
    assets::{brc20_all_transferable, brc20_transferable},
    balance::{brc20_all_balance, brc20_balance},
    receipt::{brc20_block_events, brc20_tx_events},
    ticker_info::{brc20_all_tick_info, brc20_tick_info},
    outpoint::brc20_outpoint,
};
#[derive(Debug, thiserror::Error)]
pub(super) enum BRC20ApiError {
  #[error("invalid ticker {0}, must be 4 characters long")]
  InvalidTicker(String),
  #[error("failed to retrieve ticker {0} in the database")]
  UnknownTicker(String),
  #[error("invalid address {0}")]
  InvalidAddress(String),
  /// Thrown when a transaction receipt was requested but not matching transaction receipt exists
  #[error("transaction receipt {0} not found")]
  TransactionReceiptNotFound(Txid),
  /// Thrown when an internal error occurs
  #[error("internal error: {0}")]
  Internal(String),
}

impl From<BRC20ApiError> for ApiError {
  fn from(error: BRC20ApiError) -> Self {
    match error {
      BRC20ApiError::InvalidTicker(_) => Self::bad_request(error.to_string()),
      BRC20ApiError::UnknownTicker(_) => Self::not_found(error.to_string()),
      BRC20ApiError::InvalidAddress(_) => Self::bad_request(error.to_string()),
      BRC20ApiError::TransactionReceiptNotFound(_) => Self::not_found(error.to_string()),
      BRC20ApiError::Internal(_) => Self::internal(error.to_string()),
    }
  }
}
