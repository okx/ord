use super::*;
use crate::okx::{UtxoAddress, UtxoAddressRef};
use error::ApiError;
use serde::{ser::SerializeStruct, Serializer};
use types::ApiUtxoAddress;

pub mod brc20;
mod error;
pub mod info;
pub mod ord;
mod types;

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct ApiResponse<T: Serialize> {
  pub code: i32,
  pub msg: String,
  pub data: T,
}

impl<T> ApiResponse<T>
where
  T: Serialize,
{
  fn new(code: i32, msg: String, data: T) -> Self {
    Self { code, msg, data }
  }

  pub fn ok(data: T) -> Self {
    Self::new(0, "ok".to_string(), data)
  }
}

pub(crate) type ApiResult<T> = Result<Json<ApiResponse<T>>, ApiError>;

pub(super) trait ApiOptionExt<T> {
  fn ok_or_api_err<F: FnOnce() -> ApiError>(self, f: F) -> Result<T, ApiError>;
  fn ok_or_api_not_found<S: ToString>(self, s: S) -> Result<T, ApiError>;
}

impl<T> ApiOptionExt<T> for Option<T> {
  fn ok_or_api_err<F: FnOnce() -> ApiError>(self, f: F) -> Result<T, ApiError> {
    match self {
      Some(value) => Ok(value),
      None => Err(f()),
    }
  }
  fn ok_or_api_not_found<S: ToString>(self, s: S) -> Result<T, ApiError> {
    match self {
      Some(value) => Ok(value),
      None => Err(ApiError::not_found(s)),
    }
  }
}
