use bitcoin::Txid;
use indexer_sdk::error::IndexerError;
use redb::{CommitError, TableError};

#[derive(Debug, thiserror::Error)]
pub enum SimulateError {
  #[error("tx not found: {0}")]
  TxNotFound(Txid),

  #[error("error: {0}")]
  Anyhow(#[from] anyhow::Error),

  #[error("commit failed: {0}")]
  CommitError(#[from] CommitError),

  #[error("table failed: {0}")]
  TableError(#[from] TableError),

  #[error("indexer failed: {0}")]
  IndexerError(#[from] IndexerError),

  #[error("transaction failed: {0}")]
  TransactionError(#[from] redb::TransactionError),

  #[error("over flow")]
  StackOverFlow,
}
