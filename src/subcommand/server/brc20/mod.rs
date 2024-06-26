use super::{types::ScriptPubkey, *};
mod balance;
mod outpoint;
mod receipt;
mod ticker;
mod transferable;

pub(super) use {balance::*, outpoint::*, receipt::*, ticker::*, transferable::*};

#[derive(Debug, thiserror::Error)]
pub(super) enum BRC20ApiError {
  #[error("invalid ticker {0}, must be 4 or 5 characters long")]
  InvalidTicker(String),
  #[error("failed to retrieve ticker {0} in the database")]
  UnknownTicker(String),
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
      BRC20ApiError::TransactionReceiptNotFound(_) => Self::not_found(error.to_string()),
      BRC20ApiError::Internal(_) => Self::internal(error.to_string()),
    }
  }
}
