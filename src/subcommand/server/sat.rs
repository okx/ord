use {
  super::{error::ApiError, *},
  axum::Json,
  utoipa::ToSchema,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[schema(as = ord::ApiOutPointResult)]
#[serde(rename_all = "camelCase")]
pub struct ApiOutPointResult {
  pub result: Option<ApiOutpointSatRange>,
  pub latest_blockhash: String,
  #[schema(format = "uint64")]
  pub latest_height: u32,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ApiOutpointSatRange {
  /// The transaction id.
  pub outpoint: OutPoint,
  /// The script pubkey.
  pub sat_range: Vec<(u64, u64)>,
}

// /sat/outpoint/:outpoint/info
/// Retrieve the sat range of the outpoint.
#[utoipa::path(
    get,
    path = "/api/v1/sat/outpoint/{outpoint}/info",
    params(
        ("outpoint" = String, Path, description = "Outpoint")
  ),
    responses(
      (status = 200, description = "Obtain outpoint infomation", body = OrdOutPointData),
      (status = 400, description = "Bad query.", body = ApiError, example = json!(&ApiError::bad_request("bad request"))),
      (status = 404, description = "Not found.", body = ApiError, example = json!(&ApiError::not_found("not found"))),
      (status = 500, description = "Internal server error.", body = ApiError, example = json!(&ApiError::internal("internal error"))),
    )
  )]
pub(crate) async fn sat_range_by_outpoint(
  Extension(index): Extension<Arc<Index>>,
  Path(outpoint): Path<OutPoint>,
) -> ApiResult<ApiOutPointResult> {
  log::debug!("rpc: get sat_outpoint_sat_range: {outpoint}");

  let rtx = index.begin_read()?;

  let (latest_height, latest_blockhash) = rtx.latest_block()?.ok_or_api_err(|| {
    ApiError::internal("Failed to retrieve the latest block from the database.".to_string())
  })?;

  let sat_ranges = Index::list_sat_range(&rtx, outpoint, index.has_sat_index())?;

  Ok(Json(ApiResponse::ok(ApiOutPointResult {
    result: sat_ranges.map(|sat_range| ApiOutpointSatRange {
      outpoint,
      sat_range,
    }),
    latest_height: latest_height.n(),
    latest_blockhash: latest_blockhash.to_string(),
  })))
}
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_outpoint_sat_range_json_serialization() {
    let outpoint = unbound_outpoint();
    let sat_range = vec![(0, 100), (100, 200)];
    let api_outpoint_sat_range = ApiOutpointSatRange {
      outpoint,
      sat_range,
    };
    let json = serde_json::to_string(&api_outpoint_sat_range).unwrap();
    assert_eq!(
      json,
      r#"{"outpoint":"0000000000000000000000000000000000000000000000000000000000000000:0","satRange":[[0,100],[100,200]]}"#
    );
  }
}
