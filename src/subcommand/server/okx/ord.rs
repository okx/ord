use super::*;

mod inscription;
mod outpoint;
mod receipt;

pub(crate) use {inscription::*, outpoint::*, receipt::*};

#[derive(Debug, thiserror::Error)]
pub enum OrdApiError {
  #[error("unknown inscription number {0}")]
  UnknownInscriptionNumber(i32),
  #[error("transaction {0} not found")]
  TransactionNotFound(Txid),
  #[error("transaction receipt {0} not found")]
  TransactionReceiptNotFound(Txid),
  #[error("invalid inscription {0}")]
  InvalidInscription(InscriptionId),
  #[error("satpoint not found for inscription {0}")]
  SatPointNotFound(InscriptionId),
  #[error("internal error: {0}")]
  Internal(String),
}

impl From<OrdApiError> for ApiError {
  fn from(error: OrdApiError) -> Self {
    match error {
      OrdApiError::UnknownInscriptionNumber(_) => Self::not_found(error.to_string()),
      OrdApiError::TransactionReceiptNotFound(_) => Self::not_found(error.to_string()),
      OrdApiError::TransactionNotFound(_) => Self::not_found(error.to_string()),
      OrdApiError::InvalidInscription(_) => Self::internal(error.to_string()),
      OrdApiError::SatPointNotFound(_) => Self::internal(error.to_string()),
      OrdApiError::Internal(_) => Self::internal(error.to_string()),
    }
  }
}
