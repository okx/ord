use bitcoin::address::NetworkChecked;
use {super::*, axum::Json, utoipa::ToSchema};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
#[schema(as = wallet::AvailableUnspentOutputs)]
pub struct AvailableUnspentOutputs {
  #[schema(value_type =  BTreeMap<OutPoint, u64>)]
  pub utxos: BTreeMap<OutPoint, u64>,
}

/// Get available unspent outputs.
///
/// Retrieve all available unspent outputs.
#[utoipa::path(
    get,
    path = "/api/v1/wallet/{address}/available_unspent_outputs",
    params(
        ("address" = String, Path, description = "Address")
    ),
    responses(
      (status = 200, description = "Obtain available unspent outputs.", body = WalletAvailableUnspentOutputs),
      (status = 400, description = "Bad query.", body = ApiError, example = json!(&ApiError::bad_request("bad request"))),
      (status = 404, description = "Not found.", body = ApiError, example = json!(&ApiError::not_found("not found"))),
      (status = 500, description = "Internal server error.", body = ApiError, example = json!(&ApiError::internal("internal error"))),
    )
  )]

pub(crate) async fn available_unspent_outputs(
  Extension(index): Extension<Arc<Index>>,
  Path(address): Path<String>,
) -> ApiResult<AvailableUnspentOutputs> {
  log::debug!("rpc: get available_unspent_outputs: {}", address);

  let address = Address::from_str(&address)
    .and_then(|address| address.require_network(index.get_chain_network()))
    .map_err(ApiError::bad_request)?;
  let address_ref: &[&Address<NetworkChecked>] = &[&address];
  let address_option: Option<&[&Address<NetworkChecked>]> = Some(address_ref);

  let utxos = index
    .list_unspent(address_option)
    .map_err(ApiError::bad_request)?
    .into_iter()
    .map(|utxo| {
      let outpoint = OutPoint::new(utxo.txid, utxo.vout);
      let amount = utxo.amount;

      (outpoint, amount)
    })
    .collect::<BTreeMap<OutPoint, Amount>>();

  index.check_sync(&utxos)?;

  let runic_utxos = index.get_runic_outputs(&utxos.keys().cloned().collect::<Vec<OutPoint>>())?;

  let wallet_inscriptions = index.get_inscriptions(&utxos)?;

  let inscribed_utxos = wallet_inscriptions
    .keys()
    .map(|satpoint| satpoint.outpoint)
    .collect::<BTreeSet<OutPoint>>();

  let available_utxos = utxos
    .iter()
    .filter(|(outpoint, amount)| {
      amount.to_sat() > 0 && !inscribed_utxos.contains(outpoint) && !runic_utxos.contains(outpoint)
    })
    .map(|(outpoint, amount)| (*outpoint, amount.to_sat()))
    .collect();

  Ok(Json(ApiResponse::ok(AvailableUnspentOutputs {
    utxos: available_utxos,
  })))
}
