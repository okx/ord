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
  tracing::debug!("rpc: get brc20_outpoint: {outpoint}");
  task::block_in_place(|| {
    let rtx = index.begin_read()?;
    let (height, blockhash) = trace_db_call!("get_latest_block", {
      Index::latest_block(&rtx)?.ok_or(BRC20ApiError::DataBaseNotReady)
    })?;

    let assets = trace_db_call!("get_brc20_assets_by_outpoint", {
      Index::brc20_get_transferring_assets_with_location_by_outpoint(outpoint, &rtx)
    })?;
    // If there are no inscriptions on the output, return None and parsed block states.

    Ok(Json(ApiResponse::ok(ApiOutPointResult {
      result: (!assets.is_empty()).then_some(
        assets
          .into_iter()
          .map(|(location, asset)| ApiTransferableAsset {
            inscription_id: asset.inscription_id,
            inscription_number: asset.inscription_number,
            amount: asset.amount.to_string(),
            tick: asset.ticker,
            owner: asset.owner.into(),
            location,
          })
          .collect(),
      ),
      latest_height: height.n(),
      latest_blockhash: blockhash.to_string(),
    })))
  })
}
