use super::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, thiserror::Error, Deserialize, Serialize)]
pub enum BRC20Error {
  #[error("Failed to parse ticker: {0}")]
  TickerParse(#[from] ticker::Error),

  #[error("Duplicate deployment detected for ticker: {0}")]
  DuplicateDeployment(String),

  #[error("Ticker '{0}' not found")]
  TickerNotFound(String),

  #[error("Decimals value {0} exceeds the maximum allowed limit of 18")]
  DecimalsExceedLimit(FixedPoint),

  #[error("Ticker has an invalid supply: {0}")]
  InvalidSupply(FixedPoint),

  #[error("Ticker has an invalid max mint limit: {0}")]
  InvalidMaxMintLimit(FixedPoint),

  #[error("Mint amount exceeds the allowed limit: {0}")]
  MintAmountExceedLimit(FixedPoint),

  #[error("Ticker has an invalid amount: {0}")]
  InvalidAmount(FixedPoint),

  #[error("Minting has reached the maximum supply limit")]
  MintingLimitReached,

  #[error("Insufficient balance: {0} {1}")]
  InsufficientBalance(FixedPoint, FixedPoint),

  #[error("Self-mint operation denied: insufficient permissions")]
  SelfMintPermissionDenied,

  #[error("Salt not found for ticker: {0}")]
  SaltNotFound(String),

  #[error("Salt is an invalid hex string for ticker: {0}")]
  SaltInvalidHex(String),

  #[error("Predeploy not found for ticker: {0}")]
  PredeployNotFound(String),

  #[error("Predeploy too young for ticker: {0}, block height: {1}")]
  PredeployTooYoung(String, u32),

  #[error("Predeploy hash invalid for ticker: {0}")]
  PredeployHashInvalid(String),

  #[error("Numeric error occurred: {0}")]
  NumericError(#[from] fixed_point::NumParseError),
}
