use super::{assets::ApiTransferableAsset, *};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiOutPointResult {
  pub result: Option<Vec<ApiTransferableAsset>>,
  pub latest_blockhash: String,
  pub latest_height: u32,
}

// /brc20/outpoint/:outpoint/transferable
/// Retrieve the outpoint brc20 transferable assets with the specified outpoint.
pub(crate) async fn brc20_outpoint(
  Extension(index): Extension<Arc<Index>>,
  Path(outpoint): Path<OutPoint>,
) -> ApiResult<ApiOutPointResult> {
  log::debug!("rpc: get brc20_outpoint: {outpoint}");

  let rtx = index.begin_read()?;

  let (latest_height, latest_blockhash) = Index::latest_block(&rtx)?.ok_or_api_err(|| {
    BRC20ApiError::Internal("Failed to retrieve the latest block from the database.".to_string())
      .into()
  })?;

  let assets = Index::brc20_get_transferring_assets_with_location_by_outpoint(outpoint, &rtx)?;

  // If there are no inscriptions on the output, return None and parsed block states.
  if assets.is_empty() {
    return Ok(Json(ApiResponse::ok(ApiOutPointResult {
      result: None,
      latest_height: latest_height.n(),
      latest_blockhash: latest_blockhash.to_string(),
    })));
  }

  Ok(Json(ApiResponse::ok(ApiOutPointResult {
    result: Some(
      assets
        .into_iter()
        .map(|(satpoint, asset)| ApiTransferableAsset {
          inscription_id: asset.inscription_id,
          inscription_number: asset.inscription_number,
          amount: asset.amount.to_string(),
          tick: asset.ticker,
          owner: asset.owner.into(),
          location: satpoint,
        })
        .collect(),
    ),
    latest_height: latest_height.n(),
    latest_blockhash: latest_blockhash.to_string(),
  })))
}
