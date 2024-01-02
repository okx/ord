#[derive(Debug, PartialEq, thiserror::Error)]
pub enum JSONError {
  #[error("invalid content type")]
  InvalidContentType,

  #[error("unsupport content type")]
  UnSupportContentType,

  #[error("invalid json string")]
  InvalidJson,

  #[error("not zeroindexer json")]
  NotZeroIndexerJson,

  #[error("op is not exist")]
  OpNotExist,
}

#[derive(Debug, thiserror::Error)]
pub enum LedgerError {
  #[error("ledger error: {0}")]
  LedgerError(anyhow::Error),
}
