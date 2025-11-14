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
  tracing::debug!("rpc: get brc20_balance: {} {}", ticker, address);
  task::block_in_place(|| {
    let ticker = BRC20Ticker::from_str(&ticker).map_err(ApiError::bad_request)?;

    let utxo_address =
      UtxoAddress::from_str(&address, settings.chain().network()).map_err(ApiError::bad_request)?;

    let rtx = index.begin_read()?;
    trace_db_call!("check_brc20_ticker_exists", {
      Index::brc20_get_ticker_info(&ticker, &rtx)?
        .ok_or(BRC20ApiError::UnknownTicker(ticker.to_string()))
    })?;

    let balance = trace_db_call!("get_brc20_balance", {
      Index::brc20_get_balance_by_address_ticker(&utxo_address, &ticker, &rtx)?
        .unwrap_or(BRC20Balance::new_with_ticker(&ticker))
    });

    tracing::debug!(
      "rpc: get brc20_balance: {} {} {:?}",
      ticker,
      address,
      balance
    );

    Ok(Json(ApiResponse::ok(ApiBalance {
      tick: balance.ticker,
      available_balance: balance.available.to_string(),
      transferable_balance: (balance.total - balance.available).to_string(),
      overall_balance: balance.total.to_string(),
    })))
  })
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
  tracing::debug!("rpc: get brc20_all_balance: {}", address);
  task::block_in_place(|| {
    let utxo_address =
      UtxoAddress::from_str(&address, settings.chain().network()).map_err(ApiError::bad_request)?;

    let rtx = index.begin_read()?;
    let all_balance = trace_db_call!("get_all_brc20_balances", {
      Index::brc20_get_balances_by_address(&utxo_address, &rtx)
    })?;
    tracing::debug!("rpc: get brc20_all_balance: {} {:?}", address, all_balance);

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
  })
}
