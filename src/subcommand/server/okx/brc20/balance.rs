use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiBalance {
  pub tick: BRC20Ticker,
  pub available_balance: String,
  pub transferable_balance: String,
  pub overall_balance: String,
}

/// Get the ticker balance of the address.
///
/// Retrieve the asset balance of the 'ticker' for the address.

pub(crate) async fn brc20_balance(
  Extension(settings): Extension<Arc<Settings>>,
  Extension(index): Extension<Arc<Index>>,
  Path((ticker, address)): Path<(String, String)>,
) -> ApiResult<ApiBalance> {
  log::debug!("rpc: get brc20_balance: {} {}", ticker, address);

  let rtx = index.begin_read()?;

  let ticker = BRC20Ticker::from_str(&ticker).map_err(ApiError::internal)?;

  let utxo_address =
    UtxoAddress::from_str(&address, settings.chain().network()).map_err(ApiError::internal)?;

  Index::brc20_get_ticker_info(&ticker, &rtx)?
    .ok_or(BRC20ApiError::UnknownTicker(ticker.to_string()))?;

  let balance = Index::brc20_get_balance_by_address_ticker(&utxo_address, &ticker, &rtx)?
    .unwrap_or(BRC20Balance::new_with_ticker(&ticker));

  log::debug!(
    "rpc: get brc20_balance: {} {} {:?}",
    ticker,
    address,
    balance
  );

  Ok(Json(ApiResponse::ok(ApiBalance {
    tick: balance.ticker,
    available_balance: balance.total.to_string(),
    transferable_balance: (balance.total - balance.available).to_string(),
    overall_balance: balance.available.to_string(),
  })))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiBalances {
  pub balance: Vec<ApiBalance>,
}

/// Get all ticker balances of the address.
///
/// Retrieve all BRC20 protocol asset balances associated with a address.
pub(crate) async fn brc20_all_balance(
  Extension(settings): Extension<Arc<Settings>>,
  Extension(index): Extension<Arc<Index>>,
  Path(address): Path<String>,
) -> ApiResult<ApiBalances> {
  log::debug!("rpc: get brc20_all_balance: {}", address);

  let rtx = index.begin_read()?;

  let utxo_address =
    UtxoAddress::from_str(&address, settings.chain().network()).map_err(ApiError::internal)?;

  let all_balance = Index::brc20_get_balances_by_address(&utxo_address, &rtx)?;
  log::debug!("rpc: get brc20_all_balance: {} {:?}", address, all_balance);

  Ok(Json(ApiResponse::ok(ApiBalances {
    balance: all_balance
      .into_iter()
      .map(|balance| ApiBalance {
        tick: balance.ticker,
        available_balance: balance.available.to_string(),
        transferable_balance: (balance.total - balance.available).to_string(),
        overall_balance: balance.total.to_string(),
      })
      .collect(),
  })))
}
