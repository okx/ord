use {super::*, crate::okx::datastore::brc20::Tick, axum::Json, utoipa::ToSchema};

#[derive(Default, Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(as = brc20::TransferableInscription)]
#[serde(rename_all = "camelCase")]
pub struct TransferableInscription {
  /// The inscription id.
  pub inscription_id: String,
  /// The inscription number.
  pub inscription_number: i32,
  /// The amount of the ticker that will be transferred.
  #[schema(format = "uint64")]
  pub amount: String,
  /// The ticker name that will be transferred.
  pub tick: String,
  /// The address to which the transfer will be made.
  pub owner: String,
  /// The inscription location.
  pub location: SatPoint,
}

/// Get the transferable inscriptions of the address.
///
/// Retrieve the transferable inscriptions with the ticker from the given address.
#[utoipa::path(
  get,
  path = "/api/v1/brc20/tick/{ticker}/address/{address}/transferable",
  params(
      ("ticker" = String, Path, description = "Token ticker", min_length = 4, max_length = 4),
      ("address" = String, Path, description = "Address")
),
  responses(
    (status = 200, description = "Obtain account transferable inscriptions of ticker.", body = BRC20Transferable),
    (status = 400, description = "Bad query.", body = ApiError, example = json!(&ApiError::bad_request("bad request"))),
    (status = 404, description = "Not found.", body = ApiError, example = json!(&ApiError::not_found("not found"))),
    (status = 500, description = "Internal server error.", body = ApiError, example = json!(&ApiError::internal("internal error"))),
  )
)]
pub(crate) async fn brc20_transferable(
  Extension(index): Extension<Arc<Index>>,
  Path((tick, address)): Path<(String, String)>,
) -> ApiResult<TransferableInscriptions> {
  log::debug!("rpc: get brc20_transferable: {tick} {address}");

  let rtx = index.begin_read()?;
  let network = index.get_chain_network();

  let ticker = Tick::from_str(&tick).map_err(|_| BRC20ApiError::InvalidTicker(tick.clone()))?;
  let script_key = utils::parse_and_validate_script_key_network(&address, network)
    .map_err(ApiError::bad_request)?;

  let brc20_transferable_assets =
    Index::get_brc20_transferable_utxo_by_tick_and_address(ticker, script_key, &rtx)?
      .ok_or(BRC20ApiError::UnknownTicker(tick.clone()))?;

  log::debug!(
    "rpc: get brc20_transferable: {tick} {address} {:?}",
    brc20_transferable_assets
  );

  let mut api_transferable_assets = Vec::new();
  for (satpoint, transferable_asset) in brc20_transferable_assets {
    api_transferable_assets.push(TransferableInscription {
      inscription_id: transferable_asset.inscription_id.to_string(),
      inscription_number: transferable_asset.inscription_number,
      amount: transferable_asset.amount.to_string(),
      tick: transferable_asset.tick.as_str().to_string(),
      owner: transferable_asset.owner.to_string(),
      location: satpoint,
    });
  }

  api_transferable_assets.sort_by(|a, b| a.inscription_number.cmp(&b.inscription_number));

  Ok(Json(ApiResponse::ok(TransferableInscriptions {
    inscriptions: api_transferable_assets,
  })))
}

#[derive(Default, Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(as = brc20::TransferableInscriptions)]
#[serde(rename_all = "camelCase")]
pub struct TransferableInscriptions {
  #[schema(value_type = Vec<brc20::TransferableInscription>)]
  pub inscriptions: Vec<TransferableInscription>,
}

/// Get the balance of ticker of the address.
///
/// Retrieve the balance of the ticker from the given address.
#[utoipa::path(
  get,
  path = "/api/v1/brc20/address/{address}/transferable",
  params(
      ("address" = String, Path, description = "Address")
),
  responses(
    (status = 200, description = "Obtain account all transferable inscriptions.", body = BRC20Transferable),
    (status = 400, description = "Bad query.", body = ApiError, example = json!(&ApiError::bad_request("bad request"))),
    (status = 404, description = "Not found.", body = ApiError, example = json!(&ApiError::not_found("not found"))),
    (status = 500, description = "Internal server error.", body = ApiError, example = json!(&ApiError::internal("internal error"))),
  )
)]
pub(crate) async fn brc20_all_transferable(
  Extension(index): Extension<Arc<Index>>,
  Path(account): Path<String>,
) -> ApiResult<TransferableInscriptions> {
  log::debug!("rpc: get brc20_all_transferable: {account}");

  let rtx = index.begin_read()?;
  let network = index.get_chain_network();

  let script_key = utils::parse_and_validate_script_key_network(&account, network)
    .map_err(ApiError::bad_request)?;

  let brc20_transferable_assets = rtx.brc20_get_all_transferable_by_address(script_key)?;
  log::debug!(
    "rpc: get brc20_all_transferable: {account} {:?}",
    brc20_transferable_assets
  );

  let mut api_transferable_assets = Vec::new();
  for (satpoint, transferable_asset) in brc20_transferable_assets {
    api_transferable_assets.push(TransferableInscription {
      inscription_id: transferable_asset.inscription_id.to_string(),
      inscription_number: transferable_asset.inscription_number,
      amount: transferable_asset.amount.to_string(),
      tick: transferable_asset.tick.as_str().to_string(),
      owner: transferable_asset.owner.to_string(),
      location: satpoint,
    });
  }

  api_transferable_assets.sort_by(|a, b| a.inscription_number.cmp(&b.inscription_number));

  Ok(Json(ApiResponse::ok(TransferableInscriptions {
    inscriptions: api_transferable_assets,
  })))
}
