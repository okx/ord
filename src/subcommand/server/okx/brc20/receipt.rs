use super::*;
use crate::okx::brc20::event::{
  BRC20Event, BRC20OpType, DeployEvent, InscribeTransferEvent, MintEvent, TransferEvent,
};
use crate::okx::brc20::{BRC20Error, BRC20Receipt};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[serde(rename_all = "camelCase")]
pub enum ApiTxEvent {
  Deploy(ApiDeployEvent),
  Mint(ApiMintEvent),
  InscribeTransfer(ApiInscribeTransferEvent),
  Transfer(ApiTransferEvent),
  Error(ApiErrorEvent),
}

impl From<BRC20Receipt> for ApiTxEvent {
  fn from(event: BRC20Receipt) -> Self {
    match event.result.as_ref() {
      Ok(BRC20Event::Deploy(deploy_event)) => {
        Self::Deploy(ApiDeployEvent::parse(&event, deploy_event))
      }
      Ok(BRC20Event::Mint(mint_event)) => Self::Mint(ApiMintEvent::parse(&event, mint_event)),
      Ok(BRC20Event::InscribeTransfer(inscribe_transfer_event)) => Self::InscribeTransfer(
        ApiInscribeTransferEvent::parse(&event, inscribe_transfer_event),
      ),
      Ok(BRC20Event::Transfer(transfer_event)) => {
        Self::Transfer(ApiTransferEvent::parse(&event, transfer_event))
      }
      Err(err) => Self::Error(ApiErrorEvent::parse(&event, err)),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiErrorEvent {
  #[serde(rename = "type")]
  pub event: BRC20OpType,
  pub inscription_id: InscriptionId,
  pub inscription_number: i32,
  pub old_satpoint: SatPoint,
  pub new_satpoint: SatPoint,
  pub from: ApiUtxoAddress,
  pub to: ApiUtxoAddress,
  pub valid: bool,
  pub msg: String,
}

impl ApiErrorEvent {
  fn parse(receipt: &BRC20Receipt, error: &BRC20Error) -> Self {
    Self {
      inscription_id: receipt.inscription_id,
      inscription_number: receipt.inscription_number,
      old_satpoint: receipt.old_satpoint,
      new_satpoint: receipt.new_satpoint,
      from: receipt.sender.clone().into(),
      to: receipt.receiver.clone().into(),
      valid: false,
      msg: error.to_string(),
      event: receipt.op_type,
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiDeployEvent {
  #[serde(rename = "type")]
  pub event: BRC20OpType,
  pub tick: String,
  pub inscription_id: InscriptionId,
  pub inscription_number: i32,
  pub old_satpoint: SatPoint,
  pub new_satpoint: SatPoint,
  pub supply: String,
  pub limit_per_mint: String,
  pub decimal: u8,
  pub self_mint: bool,
  pub from: ApiUtxoAddress,
  pub to: ApiUtxoAddress,
  pub valid: bool,
  pub msg: String,
}

impl ApiDeployEvent {
  fn parse(receipt: &BRC20Receipt, deploy_event: &DeployEvent) -> Self {
    Self {
      inscription_id: receipt.inscription_id,
      inscription_number: receipt.inscription_number,
      old_satpoint: receipt.old_satpoint,
      new_satpoint: receipt.new_satpoint,
      from: receipt.sender.clone().into(),
      to: receipt.receiver.clone().into(),
      tick: deploy_event.ticker.to_string(),
      supply: deploy_event.total_supply.to_string(),
      limit_per_mint: deploy_event.max_mint_limit.to_string(),
      decimal: deploy_event.decimals,
      self_mint: deploy_event.self_minted,
      valid: true,
      msg: "ok".to_string(),
      event: receipt.op_type,
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiMintEvent {
  #[serde(rename = "type")]
  pub event: BRC20OpType,
  pub tick: String,
  pub inscription_id: InscriptionId,
  pub inscription_number: i32,
  pub old_satpoint: SatPoint,
  pub new_satpoint: SatPoint,
  pub amount: String,
  pub from: ApiUtxoAddress,
  pub to: ApiUtxoAddress,
  pub valid: bool,
  pub msg: String,
}

impl ApiMintEvent {
  fn parse(receipt: &BRC20Receipt, mint_event: &MintEvent) -> Self {
    Self {
      inscription_id: receipt.inscription_id,
      inscription_number: receipt.inscription_number,
      old_satpoint: receipt.old_satpoint,
      new_satpoint: receipt.new_satpoint,
      from: receipt.sender.clone().into(),
      to: receipt.receiver.clone().into(),
      valid: true,
      tick: mint_event.ticker.to_string(),
      amount: mint_event.amount.to_string(),
      msg: "ok".to_string(),
      event: receipt.op_type,
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiInscribeTransferEvent {
  #[serde(rename = "type")]
  pub event: BRC20OpType,
  pub tick: String,
  pub inscription_id: InscriptionId,
  pub inscription_number: i32,
  pub old_satpoint: SatPoint,
  pub new_satpoint: SatPoint,
  pub amount: String,
  pub from: ApiUtxoAddress,
  pub to: ApiUtxoAddress,
  pub valid: bool,
  pub msg: String,
}

impl ApiInscribeTransferEvent {
  fn parse(receipt: &BRC20Receipt, transfer_event: &InscribeTransferEvent) -> Self {
    Self {
      inscription_id: receipt.inscription_id,
      inscription_number: receipt.inscription_number,
      old_satpoint: receipt.old_satpoint,
      new_satpoint: receipt.new_satpoint,
      from: receipt.sender.clone().into(),
      to: receipt.receiver.clone().into(),
      valid: true,
      tick: transfer_event.ticker.to_string(),
      amount: transfer_event.amount.to_string(),
      msg: "ok".to_string(),
      event: receipt.op_type,
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiTransferEvent {
  #[serde(rename = "type")]
  pub event: BRC20OpType,
  pub tick: String,
  pub inscription_id: InscriptionId,
  pub inscription_number: i32,
  pub old_satpoint: SatPoint,
  pub new_satpoint: SatPoint,
  pub amount: String,
  pub from: ApiUtxoAddress,
  pub to: ApiUtxoAddress,
  pub valid: bool,
  pub msg: String,
}

impl ApiTransferEvent {
  fn parse(receipt: &BRC20Receipt, transfer_event: &TransferEvent) -> Self {
    Self {
      inscription_id: receipt.inscription_id,
      inscription_number: receipt.inscription_number,
      old_satpoint: receipt.old_satpoint,
      new_satpoint: receipt.new_satpoint,
      from: receipt.sender.clone().into(),
      to: receipt.receiver.clone().into(),
      valid: true,
      tick: transfer_event.ticker.to_string(),
      amount: transfer_event.amount.to_string(),
      msg: "ok".to_string(),
      event: receipt.op_type,
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiTxEvents {
  pub events: Vec<ApiTxEvent>,
  pub txid: Txid,
}

/// Get transaction events by txid.
///
/// Retrieve all BRC20 events associated with a transaction.
pub(crate) async fn brc20_tx_events(
  Extension(index): Extension<Arc<Index>>,
  Path(txid): Path<String>,
) -> ApiResult<ApiTxEvents> {
  log::debug!("rpc: get brc20_tx_events: {}", txid);

  let txid = Txid::from_str(&txid).map_err(ApiError::bad_request)?;
  let rtx = index.begin_read()?;

  let tx_events = Index::brc20_get_transaction_receipts(txid, &rtx, &index.client)?
    .ok_or(BRC20ApiError::TransactionReceiptNotFound(txid))?;

  log::debug!("rpc: get brc20_tx_events: {} {:?}", txid, tx_events);

  Ok(Json(ApiResponse::ok(ApiTxEvents {
    txid,
    events: tx_events.into_iter().map(|e| e.into()).collect(),
  })))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiBlockEvents {
  pub block: Vec<ApiTxEvents>,
}

/// Get block events by blockhash.
///
/// Retrieve all BRC20 events associated with a block.

pub(crate) async fn brc20_block_events(
  Extension(index): Extension<Arc<Index>>,
  Path(blockhash): Path<String>,
) -> ApiResult<ApiBlockEvents> {
  log::debug!("rpc: get brc20_block_events: {}", blockhash);

  let blockhash = BlockHash::from_str(&blockhash).map_err(ApiError::bad_request)?;

  let rtx = index.begin_read()?;

  let block_events = Index::brc20_get_block_receipts(blockhash, &rtx, &index.client)?;

  log::debug!(
    "rpc: get brc20_block_events: {} {:?}",
    blockhash,
    block_events
  );

  Ok(Json(ApiResponse::ok(ApiBlockEvents {
    block: block_events
      .into_iter()
      .map(|(txid, events)| ApiTxEvents {
        txid,
        events: events.into_iter().map(|e| e.into()).collect(),
      })
      .filter(|e| !e.events.is_empty())
      .collect(),
  })))
}
