use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiTickInfo {
  pub tick: BRC20Ticker,
  pub inscription_id: InscriptionId,
  pub inscription_number: i32,
  pub supply: String,
  pub burned_supply: String,
  pub self_mint: bool,
  pub limit_per_mint: String,
  pub minted: String,
  pub decimal: u8,
  pub deploy_by: ApiUtxoAddress,
  pub txid: Txid,
  pub deploy_height: u32,
  pub deploy_blocktime: u32,
}

impl From<BRC20TickerInfo> for ApiTickInfo {
  fn from(tick_info: BRC20TickerInfo) -> Self {
    Self {
      tick: tick_info.ticker,
      inscription_id: tick_info.inscription_id,
      inscription_number: tick_info.inscription_number,
      supply: tick_info.total_supply.to_string(),
      burned_supply: tick_info.burned.to_string(),
      limit_per_mint: tick_info.max_mint_limit.to_string(),
      minted: tick_info.minted.to_string(),
      decimal: tick_info.decimals,
      self_mint: tick_info.self_minted,
      deploy_by: tick_info.deployer.clone().into(),
      txid: tick_info.inscription_id.txid,
      deploy_height: tick_info.deployed_block_height,
      deploy_blocktime: tick_info.deployed_timestamp,
    }
  }
}

/// Get the ticker info.
///
/// Retrieve detailed information about the ticker.

pub(crate) async fn brc20_tick_info(
  Extension(index): Extension<Arc<Index>>,
  Path(ticker): Path<String>,
) -> ApiResult<ApiTickInfo> {
  log::debug!("rpc: get brc20_tick_info: {}", ticker);

  let rtx = index.begin_read()?;

  let brc20_ticker =
    BRC20Ticker::from_str(&ticker).map_err(|_| BRC20ApiError::InvalidTicker(ticker.clone()))?;

  let tick_info = Index::brc20_get_ticker_info(&brc20_ticker, &rtx)?
    .ok_or(BRC20ApiError::UnknownTicker(ticker.clone()))?;

  log::debug!("rpc: get brc20_tick_info: {:?} {:?}", ticker, tick_info);

  Ok(Json(ApiResponse::ok(tick_info.into())))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiTickInfos {
  pub tokens: Vec<ApiTickInfo>,
}

/// Get all tickers info.
///
/// Retrieve detailed information about all tickers.

pub(crate) async fn brc20_all_tick_info(
  Extension(index): Extension<Arc<Index>>,
) -> ApiResult<ApiTickInfos> {
  log::debug!("rpc: get brc20_all_tick_info");

  let rtx = index.begin_read()?;
  let all_tick_info = Index::brc20_get_all_ticker_info(&rtx)?;
  log::debug!("rpc: get brc20_all_tick_info: {:?}", all_tick_info);

  Ok(Json(ApiResponse::ok(ApiTickInfos {
    tokens: all_tick_info.into_iter().map(|t| t.into()).collect(),
  })))
}
