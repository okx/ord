use {super::*, utoipa::ToSchema};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[schema(as = ord::ApiOutPointResult)]
#[serde(rename_all = "camelCase")]
pub struct ApiOutPointResult {
  #[schema(value_type = Option<brc20::ApiTransferableAssets>)]
  pub result: Option<Vec<ApiTransferableAsset>>,
  pub latest_blockhash: String,
  #[schema(format = "uint64")]
  pub latest_height: u32,
}

// /brc20/outpoint/:outpoint/transferable
/// Retrieve the outpoint brc20 transferable assets with the specified outpoint.
#[utoipa::path(
  get,
  path = "/api/v1/brc20/outpoint/{outpoint}/transferable",
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
pub(crate) async fn brc20_outpoint(
  Extension(index): Extension<Arc<Index>>,
  Path(outpoint): Path<OutPoint>,
) -> ApiResult<ApiOutPointResult> {
  log::debug!("rpc: get brc20_outpoint: {outpoint}");

  let rtx = index.begin_read()?;

  let (latest_height, latest_blockhash) = rtx.latest_block()?.ok_or_api_err(|| {
    BRC20ApiError::Internal("Failed to retrieve the latest block from the database.".to_string())
      .into()
  })?;

  let transferable_assets_with_satpoints =
    rtx.brc20_transferable_assets_on_output_with_satpoints(outpoint)?;

  // If there are no inscriptions on the output, return None and parsed block states.
  if transferable_assets_with_satpoints.is_empty() {
    return Ok(Json(ApiResponse::ok(ApiOutPointResult {
      result: None,
      latest_height: latest_height.n(),
      latest_blockhash: latest_blockhash.to_string(),
    })));
  }

  Ok(Json(ApiResponse::ok(ApiOutPointResult {
    result: Some(
      transferable_assets_with_satpoints
        .into_iter()
        .map(|(satpoint, asset)| ApiTransferableAsset {
          inscription_id: asset.inscription_id.to_string(),
          inscription_number: asset.inscription_number,
          amount: asset.amount.to_string(),
          tick: asset.tick.as_str().to_string(),
          owner: asset.owner.to_string(),
          location: satpoint,
        })
        .collect(),
    ),
    latest_height: latest_height.n(),
    latest_blockhash: latest_blockhash.to_string(),
  })))
}
