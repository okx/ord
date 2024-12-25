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

  let brc20_ticker =
    BRC20Ticker::from_str(&ticker).map_err(|_| BRC20ApiError::InvalidTicker(ticker.clone()))?;

  let utxo_address = UtxoAddress::from_str(&address, settings.chain().network())
    .map_err(|_| BRC20ApiError::InvalidAddress(address.clone()))?;

  Index::brc20_get_ticker_info(&brc20_ticker, &rtx)?
    .ok_or(BRC20ApiError::UnknownTicker(ticker.clone()))?;

  let assets = Index::brc20_get_transferring_assets_with_location_by_address_ticker(
    &utxo_address,
    &brc20_ticker,
    &rtx,
  )?;

  log::debug!(
    "rpc: get brc20_transferable: {ticker} {address} {:?}",
    assets
  );

  let mut api_transferable_assets = Vec::new();
  for (satpoint, asset) in assets {
    api_transferable_assets.push(ApiTransferableAsset {
      inscription_id: asset.inscription_id,
      inscription_number: asset.inscription_number,
      amount: asset.amount.to_string(),
      tick: asset.ticker,
      owner: ApiUtxoAddress::from(utxo_address.clone()),
      location: satpoint,
    });
  }

  api_transferable_assets.sort_by(|a, b| a.inscription_number.cmp(&b.inscription_number));

  Ok(Json(ApiResponse::ok(ApiTransferableAssets {
    inscriptions: api_transferable_assets,
  })))
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiTransferableAssets {
  pub inscriptions: Vec<ApiTransferableAsset>,
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

  let utxo_address = UtxoAddress::from_str(&address, settings.chain().network())
    .map_err(|_| BRC20ApiError::InvalidAddress(address.clone()))?;

  let assets = Index::get_brc20_transferring_assets_location_by_address(&utxo_address, &rtx)?;

  let mut api_transferable_assets = Vec::new();
  for (satpoint, asset) in assets {
    api_transferable_assets.push(ApiTransferableAsset {
      inscription_id: asset.inscription_id,
      inscription_number: asset.inscription_number,
      amount: asset.amount.to_string(),
      tick: asset.ticker,
      owner: ApiUtxoAddress::from(utxo_address.clone()),
      location: satpoint,
    });
  }

  api_transferable_assets.sort_by(|a, b| a.inscription_number.cmp(&b.inscription_number));

  Ok(Json(ApiResponse::ok(ApiTransferableAssets {
    inscriptions: api_transferable_assets,
  })))
}
