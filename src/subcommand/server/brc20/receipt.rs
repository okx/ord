use {
  self::okx::datastore::brc20::OperationType, super::*,
  crate::okx::datastore::brc20 as brc20_store, axum::Json, utoipa::ToSchema,
};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(as = brc20::TxEvent)]
#[serde(untagged)]
#[serde(rename_all = "camelCase")]
pub enum ApiTxEvent {
  /// Event generated by deployed ticker.
  #[schema(value_type = brc20::ApiDeployEvent)]
  Deploy(ApiDeployEvent),
  /// Event generated by mining.
  #[schema(value_type = brc20::ApiMintEvent)]
  Mint(ApiMintEvent),
  /// Event generated by pretransfer.
  #[schema(value_type = brc20::ApiInscribeTransferEvent)]
  InscribeTransfer(ApiInscribeTransferEvent),
  #[schema(value_type = brc20::ApiTransferEvent)]
  /// Event generated by transfer.
  Transfer(ApiTransferEvent),
  /// Event generated by the execution has failed.
  #[schema(value_type = brc20::ApiErrorEvent)]
  Error(ApiErrorEvent),
}

impl From<brc20_store::Receipt> for ApiTxEvent {
  fn from(event: brc20_store::Receipt) -> Self {
    match event.result.as_ref() {
      Ok(brc20_store::Event::Deploy(deploy_event)) => {
        Self::Deploy(ApiDeployEvent::parse(&event, deploy_event))
      }
      Ok(brc20_store::Event::Mint(mint_event)) => {
        Self::Mint(ApiMintEvent::parse(&event, mint_event))
      }
      Ok(brc20_store::Event::InscribeTransfer(inscribe_transfer_event)) => Self::InscribeTransfer(
        ApiInscribeTransferEvent::parse(&event, inscribe_transfer_event),
      ),
      Ok(brc20_store::Event::Transfer(transfer_event)) => {
        Self::Transfer(ApiTransferEvent::parse(&event, transfer_event))
      }
      Err(err) => Self::Error(ApiErrorEvent::parse(&event, err)),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(as = brc20::ApiErrorEvent)]
#[serde(rename_all = "camelCase")]
pub struct ApiErrorEvent {
  /// Event type.
  #[serde(rename = "type")]
  pub event: String,
  /// The inscription id.
  pub inscription_id: String,
  /// The inscription number.
  pub inscription_number: i32,
  /// The inscription satpoint of the transaction input.
  pub old_satpoint: String,
  /// The inscription satpoint of the transaction output.
  pub new_satpoint: String,
  /// The message sender which is an address or script pubkey hash.
  pub from: ScriptPubkey,
  /// The message receiver which is an address or script pubkey hash.
  pub to: ScriptPubkey,
  /// Executed state.
  pub valid: bool,
  /// Error message.
  pub msg: String,
}

impl ApiErrorEvent {
  fn parse(event: &brc20_store::Receipt, error: &brc20_store::BRC20Error) -> Self {
    Self {
      inscription_id: event.inscription_id.to_string(),
      inscription_number: event.inscription_number,
      old_satpoint: event.old_satpoint.to_string(),
      new_satpoint: event.new_satpoint.to_string(),
      from: event.from.clone().into(),
      to: event.to.clone().into(),
      valid: false,
      msg: error.to_string(),
      event: event.op.to_string(),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(as = brc20::ApiDeployEvent)]
#[serde(rename_all = "camelCase")]
pub struct ApiDeployEvent {
  /// Event type.
  #[serde(rename = "type")]
  pub event: String,
  /// The ticker deployed.
  pub tick: String,
  /// The inscription id.
  pub inscription_id: String,
  /// The inscription number.
  pub inscription_number: i32,
  /// The inscription satpoint of the transaction input.
  pub old_satpoint: String,
  /// The inscription satpoint of the transaction output.
  pub new_satpoint: String,
  /// The total supply of the deployed ticker.
  pub supply: String,
  /// The limit per mint of the deployed ticker.
  pub limit_per_mint: String,
  /// The decimal of the deployed ticker.
  pub decimal: u8,
  /// Whether the ticker is self minted.
  pub self_mint: bool,
  /// The message sender which is an address or script pubkey hash.
  pub from: ScriptPubkey,
  /// The message receiver which is an address or script pubkey hash.
  pub to: ScriptPubkey,
  /// Executed state.
  pub valid: bool,
  /// Message generated during execution.
  pub msg: String,
}

impl ApiDeployEvent {
  fn parse(event: &brc20_store::Receipt, deploy_event: &brc20_store::DeployEvent) -> Self {
    Self {
      tick: deploy_event.tick.to_string(),
      inscription_id: event.inscription_id.to_string(),
      inscription_number: event.inscription_number,
      old_satpoint: event.old_satpoint.to_string(),
      new_satpoint: event.new_satpoint.to_string(),
      supply: deploy_event.supply.to_string(),
      limit_per_mint: deploy_event.limit_per_mint.to_string(),
      decimal: deploy_event.decimal,
      self_mint: deploy_event.self_mint,
      from: event.from.clone().into(),
      to: event.to.clone().into(),
      valid: true,
      msg: "ok".to_string(),
      event: OperationType::Deploy.to_string(),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(as = brc20::ApiMintEvent)]
#[serde(rename_all = "camelCase")]
pub struct ApiMintEvent {
  #[serde(rename = "type")]
  /// Event type.
  pub event: String,
  /// The ticker minted.
  pub tick: String,
  /// The inscription id.
  pub inscription_id: String,
  /// The inscription number.
  pub inscription_number: i32,
  /// The inscription satpoint of the transaction input.
  pub old_satpoint: String,
  /// The inscription satpoint of the transaction output.
  pub new_satpoint: String,
  /// The amount minted.
  pub amount: String,
  /// The message sender which is an address or script pubkey hash.
  pub from: ScriptPubkey,
  /// The message receiver which is an address or script pubkey hash.
  pub to: ScriptPubkey,
  /// Executed state.
  pub valid: bool,
  /// Message generated during execution.
  pub msg: String,
}

impl ApiMintEvent {
  fn parse(event: &brc20_store::Receipt, mint_event: &brc20_store::MintEvent) -> Self {
    Self {
      tick: mint_event.tick.to_string(),
      inscription_id: event.inscription_id.to_string(),
      inscription_number: event.inscription_number,
      old_satpoint: event.old_satpoint.to_string(),
      new_satpoint: event.new_satpoint.to_string(),
      amount: mint_event.amount.to_string(),
      from: event.from.clone().into(),
      to: event.to.clone().into(),
      valid: true,
      msg: mint_event.msg.clone().unwrap_or("ok".to_string()),
      event: OperationType::Mint.to_string(),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(as = brc20::ApiInscribeTransferEvent)]
#[serde(rename_all = "camelCase")]
pub struct ApiInscribeTransferEvent {
  /// Event type.
  #[serde(rename = "type")]
  pub event: String,
  /// The ticker of pretransfer.
  pub tick: String,
  /// The inscription id.
  pub inscription_id: String,
  /// The inscription number.
  pub inscription_number: i32,
  /// The inscription satpoint of the transaction input.
  pub old_satpoint: String,
  /// The inscription satpoint of the transaction output.
  pub new_satpoint: String,
  /// The amount of pretransfer.
  pub amount: String,
  /// The message sender which is an address or script pubkey hash.
  pub from: ScriptPubkey,
  /// The message receiver which is an address or script pubkey hash.
  pub to: ScriptPubkey,
  /// Executed state.
  pub valid: bool,
  /// Message generated during execution.
  pub msg: String,
}

impl ApiInscribeTransferEvent {
  fn parse(
    event: &brc20_store::Receipt,
    transfer_event: &brc20_store::InscribeTransferEvent,
  ) -> Self {
    Self {
      tick: transfer_event.tick.to_string(),
      inscription_id: event.inscription_id.to_string(),
      inscription_number: event.inscription_number,
      old_satpoint: event.old_satpoint.to_string(),
      new_satpoint: event.new_satpoint.to_string(),
      amount: transfer_event.amount.to_string(),
      from: event.from.clone().into(),
      to: event.to.clone().into(),
      valid: true,
      msg: "ok".to_string(),
      event: OperationType::InscribeTransfer.to_string(),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(as = brc20::ApiTransferEvent)]
#[serde(rename_all = "camelCase")]
pub struct ApiTransferEvent {
  /// Event type.
  #[serde(rename = "type")]
  pub event: String,
  /// The ticker of transfer.
  pub tick: String,
  /// The inscription id.
  pub inscription_id: String,
  /// The inscription number.
  pub inscription_number: i32,
  /// The inscription satpoint of the transaction input.
  pub old_satpoint: String,
  /// The inscription satpoint of the transaction output.
  pub new_satpoint: String,
  /// The amount of transfer.
  pub amount: String,
  /// The message sender which is an address or script pubkey hash.
  pub from: ScriptPubkey,
  /// The message receiver which is an address or script pubkey hash.
  pub to: ScriptPubkey,
  /// Executed state.
  pub valid: bool,
  /// Message generated during execution.
  pub msg: String,
}

impl ApiTransferEvent {
  fn parse(event: &brc20_store::Receipt, transfer_event: &brc20_store::TransferEvent) -> Self {
    Self {
      tick: transfer_event.tick.to_string(),
      inscription_id: event.inscription_id.to_string(),
      inscription_number: event.inscription_number,
      old_satpoint: event.old_satpoint.to_string(),
      new_satpoint: event.new_satpoint.to_string(),
      amount: transfer_event.amount.to_string(),
      from: event.from.clone().into(),
      to: event.to.clone().into(),
      valid: true,
      msg: transfer_event.msg.clone().unwrap_or("ok".to_string()),
      event: OperationType::Transfer.to_string(),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(as = brc20::TxEvents)]
#[serde(rename_all = "camelCase")]
pub struct ApiTxEvents {
  #[schema(value_type = Vec<brc20::TxEvent>)]
  pub events: Vec<ApiTxEvent>,
  pub txid: String,
}

/// Get transaction events by txid.
///
/// Retrieve all BRC20 events associated with a transaction.
#[utoipa::path(
    get,
    path = "/api/v1/brc20/tx/{txid}/events",
    params(
        ("txid" = String, Path, description = "transaction ID")
  ),
    responses(
      (status = 200, description = "Obtain transaction events by txid", body = BRC20TxEvents),
      (status = 400, description = "Bad query.", body = ApiError, example = json!(&ApiError::bad_request("bad request"))),
      (status = 404, description = "Not found.", body = ApiError, example = json!(&ApiError::not_found("not found"))),
      (status = 500, description = "Internal server error.", body = ApiError, example = json!(&ApiError::internal("internal error"))),
    )
  )]
pub(crate) async fn brc20_tx_events(
  Extension(index): Extension<Arc<Index>>,
  Path(txid): Path<String>,
) -> ApiResult<ApiTxEvents> {
  log::debug!("rpc: get brc20_tx_events: {}", txid);

  let txid = bitcoin::Txid::from_str(&txid).map_err(ApiError::bad_request)?;
  let rtx = index.begin_read()?;
  let client = index.bitcoin_rpc_client()?;

  let tx_events = Index::get_brc20_transaction_receipts(txid, &rtx, &client)?
    .ok_or(BRC20ApiError::TransactionReceiptNotFound(txid))?;

  log::debug!("rpc: get brc20_tx_events: {} {:?}", txid, tx_events);

  Ok(Json(ApiResponse::ok(ApiTxEvents {
    txid: txid.to_string(),
    events: tx_events.into_iter().map(|e| e.into()).collect(),
  })))
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[schema(as = brc20::BlockEvents)]
#[serde(rename_all = "camelCase")]
pub struct ApiBlockEvents {
  #[schema(value_type = Vec<brc20::TxEvents>)]
  pub block: Vec<ApiTxEvents>,
}

/// Get block events by blockhash.
///
/// Retrieve all BRC20 events associated with a block.
#[utoipa::path(
    get,
    path = "/api/v1/brc20/block/{blockhash}/events",
    params(
        ("blockhash" = String, Path, description = "block hash")
  ),
    responses(
      (status = 200, description = "Obtain block events by block hash", body = BRC20BlockEvents),
      (status = 400, description = "Bad query.", body = ApiError, example = json!(&ApiError::bad_request("bad request"))),
      (status = 404, description = "Not found.", body = ApiError, example = json!(&ApiError::not_found("not found"))),
      (status = 500, description = "Internal server error.", body = ApiError, example = json!(&ApiError::internal("internal error"))),
    )
  )]
pub(crate) async fn brc20_block_events(
  Extension(index): Extension<Arc<Index>>,
  Path(blockhash): Path<String>,
) -> ApiResult<ApiBlockEvents> {
  log::debug!("rpc: get brc20_block_events: {}", blockhash);

  let blockhash = bitcoin::BlockHash::from_str(&blockhash).map_err(ApiError::bad_request)?;

  let rtx = index.begin_read()?;
  let client = index.bitcoin_rpc_client()?;

  let block_events = Index::get_brc20_block_receipts(blockhash, &rtx, &client)?;

  log::debug!(
    "rpc: get brc20_block_events: {} {:?}",
    blockhash,
    block_events
  );

  Ok(Json(ApiResponse::ok(ApiBlockEvents {
    block: block_events
      .into_iter()
      .map(|(txid, events)| ApiTxEvents {
        txid: txid.to_string(),
        events: events.into_iter().map(|e| e.into()).collect(),
      })
      .collect(),
  })))
}
