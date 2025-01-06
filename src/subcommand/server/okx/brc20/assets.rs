use super::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiTransferableAsset {
  pub inscription_id: InscriptionId,
  pub inscription_number: i32,
  pub amount: String,
  pub tick: BRC20Ticker,
  pub owner: ApiUtxoAddress,
  pub location: SatPoint,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiTransferableAssets {
  pub inscriptions: Vec<ApiTransferableAsset>,
}

/// Helper function to process assets into `ApiTransferableAsset`
fn process_assets(
  assets: Vec<(SatPoint, BRC20TransferAsset)>,
  utxo_address: &UtxoAddress,
) -> Vec<ApiTransferableAsset> {
  let mut api_transferable_assets = assets
    .into_iter()
    .map(|(satpoint, asset)| ApiTransferableAsset {
      inscription_id: asset.inscription_id,
      inscription_number: asset.inscription_number,
      amount: asset.amount.to_string(),
      tick: asset.ticker,
      owner: ApiUtxoAddress::from(utxo_address),
      location: satpoint,
    })
    .collect::<Vec<_>>();

  // Sort by inscription number
  api_transferable_assets.sort_by(|a, b| a.inscription_number.cmp(&b.inscription_number));

  api_transferable_assets
}

/// Get the transferable inscriptions of the address.
///
/// Retrieve the transferable inscriptions with the ticker from the given address.
pub(crate) async fn brc20_transferable(
  Extension(settings): Extension<Arc<Settings>>,
  Extension(index): Extension<Arc<Index>>,
  Path((ticker, address)): Path<(String, String)>,
) -> ApiResult<ApiTransferableAssets> {
  log::debug!("rpc: get brc20_transferable: {ticker} {address}");

  let rtx = index.begin_read()?;

  let brc20_ticker = BRC20Ticker::from_str(&ticker).map_err(ApiError::internal)?;

  let utxo_address =
    UtxoAddress::from_str(&address, settings.chain().network()).map_err(ApiError::internal)?;

  Index::brc20_get_ticker_info(&brc20_ticker, &rtx)?
    .ok_or(BRC20ApiError::UnknownTicker(ticker.clone()))?;

  let assets = Index::brc20_get_transferring_assets_with_location_by_address_ticker(
    &utxo_address,
    &brc20_ticker,
    &rtx,
  )?;

  log::debug!(
    "rpc: get brc20_transferable: {ticker} {address}, assets count: {}",
    assets.len()
  );

  let api_transferable_assets = process_assets(assets, &utxo_address);

  Ok(Json(ApiResponse::ok(ApiTransferableAssets {
    inscriptions: api_transferable_assets,
  })))
}

/// Get the balance of ticker of the address.
///
/// Retrieve the balance of the ticker from the given address.
pub(crate) async fn brc20_all_transferable(
  Extension(settings): Extension<Arc<Settings>>,
  Extension(index): Extension<Arc<Index>>,
  Path(address): Path<String>,
) -> ApiResult<ApiTransferableAssets> {
  log::debug!("rpc: get brc20_all_transferable: {address}");

  let rtx = index.begin_read()?;

  let utxo_address =
    UtxoAddress::from_str(&address, settings.chain().network()).map_err(ApiError::internal)?;

  let assets = Index::get_brc20_transferring_assets_location_by_address(&utxo_address, &rtx)?;

  log::debug!(
    "rpc: get brc20_all_transferable: {address}, assets count: {}",
    assets.len()
  );

  let api_transferable_assets = process_assets(assets, &utxo_address);

  Ok(Json(ApiResponse::ok(ApiTransferableAssets {
    inscriptions: api_transferable_assets,
  })))
}
