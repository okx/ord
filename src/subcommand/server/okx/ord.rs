use super::*;

mod bitmap;
mod btc_domain;
mod inscription;
mod outpoint;
mod receipt;

pub(crate) use {bitmap::*, btc_domain::*, inscription::*, outpoint::*, receipt::*};
#[derive(Debug, thiserror::Error)]
pub enum OrdApiError {
  #[error("Unknown inscription number '{0}': no matching inscription found.")]
  InscriptionNotFoundByNum(i32),

  #[error("Invalid inscription ID '{0}': the inscription does not exist.")]
  InscriptionNotFoundById(InscriptionId),

  #[error("Inscription entry not found for sequence number '{0}'.")]
  InscriptionEntryNotFound(u32),

  #[error("Location not found for sequence number '{0}'.")]
  LocationNotFound(u32),

  #[error("Failed to parse envelope with index '{0}' for transaction '{1}'.")]
  ParsedEnvelopeError(u32, Txid),

  #[error("Transaction receipt for transaction ID '{0}' not found: no matching receipt exists.")]
  TransactionReceiptNotFound(Txid),

  #[error("Transaction with ID '{0}' not found.")]
  TransactionNotFound(Txid),

  #[error("Block receipt for hash '{0}' not found: no matching receipt exists.")]
  BlockReceiptNotFound(BlockHash),

  #[error("Conflict detected for block at height '{0}'.")]
  ConflictBlockByHeight(Height),

  #[error("Database is not ready: the database is not yet initialized.")]
  DataBaseNotReady,
}

impl From<OrdApiError> for ApiError {
  fn from(error: OrdApiError) -> Self {
    match error {
      OrdApiError::InscriptionNotFoundByNum(_)
      | OrdApiError::InscriptionEntryNotFound(_)
      | OrdApiError::LocationNotFound(_)
      | OrdApiError::TransactionReceiptNotFound(_)
      | OrdApiError::TransactionNotFound(_)
      | OrdApiError::BlockReceiptNotFound(_) => Self::not_found(error.to_string()),

      OrdApiError::InscriptionNotFoundById(_)
      | OrdApiError::ParsedEnvelopeError(_, _)
      | OrdApiError::ConflictBlockByHeight(_)
      | OrdApiError::DataBaseNotReady => Self::internal(error.to_string()),
    }
  }
}
