use super::*;
#[repr(i32)]
pub(crate) enum ApiError {
  /// Internal server error.
  Internal(String) = 1,
  /// Bad request.
  BadRequest(String) = 2,
  /// Resource not found.
  NotFound(String) = 3,
}

impl ApiError {
  pub(crate) fn code(&self) -> i32 {
    match self {
      Self::Internal(_) => 1,
      Self::BadRequest(_) => 2,
      Self::NotFound(_) => 3,
    }
  }

  pub(crate) fn not_found<S: ToString>(message: S) -> Self {
    Self::NotFound(message.to_string())
  }

  pub(crate) fn internal<S: ToString>(message: S) -> Self {
    Self::Internal(message.to_string())
  }

  pub(crate) fn bad_request<S: ToString>(message: S) -> Self {
    Self::BadRequest(message.to_string())
  }
}

impl Serialize for ApiError {
  fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    let mut state = serializer.serialize_struct("ApiError", 2)?;
    match self {
      ApiError::Internal(msg) | ApiError::BadRequest(msg) | ApiError::NotFound(msg) => {
        state.serialize_field("code", &self.code())?;
        state.serialize_field("msg", &msg)?;
        state.end()
      }
    }
  }
}

impl IntoResponse for ApiError {
  fn into_response(self) -> Response {
    let status_code = match &self {
      Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
      Self::BadRequest(_) => StatusCode::BAD_REQUEST,
      Self::NotFound(_) => StatusCode::NOT_FOUND,
    };

    (status_code, Json(self)).into_response()
  }
}

impl From<anyhow::Error> for ApiError {
  fn from(error: anyhow::Error) -> Self {
    Self::internal(error)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_serialize_api_error() {
    let api_error = ApiError::internal("internal error");
    let json = serde_json::to_string(&api_error).unwrap();
    assert_eq!(json, r#"{"code":1,"msg":"internal error"}"#);

    let api_error = ApiError::bad_request("bad request");
    let json = serde_json::to_string(&api_error).unwrap();
    assert_eq!(json, r#"{"code":2,"msg":"bad request"}"#);

    let api_error = ApiError::not_found("not found");
    let json = serde_json::to_string(&api_error).unwrap();
    assert_eq!(json, r#"{"code":3,"msg":"not found"}"#);
  }
}
