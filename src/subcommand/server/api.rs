use super::error::ApiError;
use super::operation::Brc20Transaction;
use super::*;
use axum::Json;

pub(crate) type ApiResult<T> = Result<axum::Json<ApiResponse<T>>, ApiError>;

pub(super) trait ApiOptionExt<T> {
  fn ok_or_api_err<F: FnOnce() -> ApiError>(self, f: F) -> Result<T, ApiError>;
  fn ok_or_api_notfound<S: Into<String>>(self, s: S) -> Result<T, ApiError>;
}

impl<T> ApiOptionExt<T> for Option<T> {
  fn ok_or_api_err<F: FnOnce() -> ApiError>(self, f: F) -> Result<T, ApiError> {
    match self {
      Some(value) => Ok(value),
      None => Err(f()),
    }
  }
  fn ok_or_api_notfound<S: Into<String>>(self, s: S) -> Result<T, ApiError> {
    match self {
      Some(value) => Ok(value),
      None => Err(ApiError::not_found(s)),
    }
  }
}

const ERR_TICK_LENGTH: &str = "tick must be 4 bytes length";

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TickInfo {
  pub tick: String,
  pub inscription_id: String,
  pub inscription_number: String,
  pub supply: String,
  pub limit_per_mint: String,
  pub minted: String,
  pub decimal: u64,
  pub deploy_by: String,
  pub txid: String,
  pub deploy_height: u64,
  pub deploy_blocktime: u64,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllTickInfo {
  pub tokens: Vec<TickInfo>,
}

impl From<&brc20::TokenInfo> for TickInfo {
  fn from(tick_info: &brc20::TokenInfo) -> Self {
    Self {
      tick: std::str::from_utf8(tick_info.tick.as_bytes())
        .unwrap()
        .to_string(),
      inscription_id: tick_info.inscription_id.to_string(),
      inscription_number: tick_info.inscription_number.to_string(),
      supply: tick_info.supply.to_string(),
      limit_per_mint: tick_info.limit_per_mint.to_string(),
      minted: tick_info.minted.to_string(),
      decimal: tick_info.decimal as u64,
      deploy_by: tick_info.deploy_by.to_string(),
      txid: tick_info.inscription_id.txid.to_string(),
      deploy_height: tick_info.deployed_number,
      deploy_blocktime: tick_info.deployed_timestamp as u64,
    }
  }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Balance {
  pub tick: String,
  pub available_balance: String,
  pub transferable_balance: String,
  pub overall_balance: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AllBalance {
  pub balance: Vec<Balance>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TxEvents {
  pub events: Vec<TxEvent>,
  pub txid: String,
}

impl From<&brc20::ActionReceipt> for TxEvent {
  fn from(event: &brc20::ActionReceipt) -> Self {
    match &event.result {
      Ok(result) => match result {
        brc20::BRC20Event::Deploy(deploy_event) => Self::Deploy(DeployEvent {
          tick: std::str::from_utf8(deploy_event.tick.as_bytes())
            .unwrap()
            .to_string(),
          inscription_id: event.inscription_id.to_string(),
          inscription_number: event.inscription_number.to_string(),
          old_satpoint: event.old_satpoint,
          new_satpoint: event.new_satpoint,
          supply: deploy_event.supply.to_string(),
          limit_per_mint: deploy_event.limit_per_mint.to_string(),
          decimal: deploy_event.decimal as u64,
          msg_sender: event.from.to_string(),
          deploy_by: event.to.to_string(),
          valid: true,
          msg: "ok".to_string(),
          event: String::from("deploy"),
        }),
        brc20::BRC20Event::Mint(mint_event) => Self::Mint(MintEvent {
          tick: std::str::from_utf8(mint_event.tick.as_bytes())
            .unwrap()
            .to_string(),
          inscription_id: event.inscription_id.to_string(),
          inscription_number: event.inscription_number.to_string(),
          old_satpoint: event.old_satpoint,
          new_satpoint: event.new_satpoint,
          amount: mint_event.amount.to_string(),
          msg_sender: event.from.to_string(),
          to: event.to.to_string(),
          valid: true,
          msg: mint_event.msg.clone().unwrap_or("ok".to_string()),
          event: String::from("mint"),
        }),
        brc20::BRC20Event::TransferPhase1(trans1) => {
          Self::InscribeTransfer(InscribeTransferEvent {
            tick: std::str::from_utf8(trans1.tick.as_bytes())
              .unwrap()
              .to_string(),
            inscription_id: event.inscription_id.to_string(),
            inscription_number: event.inscription_number.to_string(),
            old_satpoint: event.old_satpoint,
            new_satpoint: event.new_satpoint,
            amount: trans1.amount.to_string(),
            msg_sender: event.from.to_string(),
            owner: event.to.to_string(),
            valid: true,
            msg: "ok".to_string(),
            event: String::from("inscribeTransfer"),
          })
        }
        brc20::BRC20Event::TransferPhase2(trans2) => Self::Transfer(TransferEvent {
          tick: std::str::from_utf8(trans2.tick.as_bytes())
            .unwrap()
            .to_string(),
          inscription_id: event.inscription_id.to_string(),
          inscription_number: event.inscription_number.to_string(),
          old_satpoint: event.old_satpoint,
          new_satpoint: event.new_satpoint,
          amount: trans2.amount.to_string(),
          from: event.from.to_string(),
          to: event.to.to_string(),
          valid: true,
          msg: trans2.msg.clone().unwrap_or("ok".to_string()),
          event: String::from("transfer"),
        }),
      },
      Err(err) => Self::Error(ErrorEvent {
        inscription_id: event.inscription_id.to_string(),
        inscription_number: event.inscription_number.to_string(),
        old_satpoint: event.old_satpoint,
        new_satpoint: event.new_satpoint,
        valid: false,
        from: event.from.to_string(),
        to: event.to.to_string(),
        msg: err.to_string(),
        event: match event.op {
          brc20::EventType::Deploy => "deploy",
          brc20::EventType::Mint => "mint",
          brc20::EventType::TransferPhase1 => "inscribeTransfer",
          brc20::EventType::TransferPhase2 => "transfer",
        }
        .to_string(),
      }),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op")]
#[serde(rename_all = "camelCase")]
pub enum TxEvent {
  Deploy(DeployEvent),
  Mint(MintEvent),
  InscribeTransfer(InscribeTransferEvent),
  Transfer(TransferEvent),
  Error(ErrorEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ErrorEvent {
  pub inscription_id: String,
  pub inscription_number: String,
  pub old_satpoint: SatPoint,
  pub new_satpoint: Option<SatPoint>,
  pub from: String,
  pub to: String,
  pub valid: bool,
  pub msg: String,
  pub event: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeployEvent {
  pub tick: String,
  pub inscription_id: String,
  pub inscription_number: String,
  pub old_satpoint: SatPoint,
  pub new_satpoint: Option<SatPoint>,
  pub supply: String,
  pub limit_per_mint: String,
  pub decimal: u64,
  pub msg_sender: String,
  pub deploy_by: String,
  pub valid: bool,
  pub msg: String,
  pub event: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MintEvent {
  pub tick: String,
  pub inscription_id: String,
  pub inscription_number: String,
  pub old_satpoint: SatPoint,
  pub new_satpoint: Option<SatPoint>,
  pub amount: String,
  pub msg_sender: String,
  pub to: String,
  pub valid: bool,
  pub msg: String,
  pub event: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InscribeTransferEvent {
  pub tick: String,
  pub inscription_id: String,
  pub inscription_number: String,
  pub old_satpoint: SatPoint,
  pub new_satpoint: Option<SatPoint>,
  pub amount: String,
  pub msg_sender: String,
  pub owner: String,
  pub valid: bool,
  pub msg: String,
  pub event: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferEvent {
  pub tick: String,
  pub inscription_id: String,
  pub inscription_number: String,
  pub old_satpoint: SatPoint,
  pub new_satpoint: Option<SatPoint>,
  pub amount: String,
  pub from: String,
  pub to: String,
  pub valid: bool,
  pub msg: String,
  pub event: String,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlockEvents {
  pub block: Vec<TxEvents>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferableInscriptions {
  pub inscriptions: Vec<TransferableInscription>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferableInscription {
  pub id: String,
  pub number: String,
  pub amount: String,
  pub tick: String,
  pub owner: String,
}

impl From<&brc20::TransferableLog> for TransferableInscription {
  fn from(trans: &brc20::TransferableLog) -> Self {
    Self {
      id: trans.inscription_id.to_string(),
      number: trans.inscription_number.to_string(),
      amount: trans.amount.to_string(),
      tick: trans.tick.as_str().to_string(),
      owner: trans.owner.to_string(),
    }
  }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HeightInfo<T: Serialize> {
  pub ord_height: Option<u64>,
  pub btc_chain_info: Option<T>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct HeightInfoQuery {
  btc: Option<bool>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrdInscription {
  pub id: String,
  pub number: String,
  pub content_type: Option<String>,
  pub content: Option<String>,
  pub owner: String,
  pub genesis_height: u64,
  pub location: String,
  pub sat: Option<u64>,
}

pub(crate) async fn node_info(
  Extension(index): Extension<Arc<Index>>,
  Query(query): Query<HeightInfoQuery>,
) -> ApiResult<HeightInfo<bitcoincore_rpc::json::GetBlockchainInfoResult>> {
  log::debug!("rpc: get node_info");

  let (ord_height, btc_info) = index.height_btc(query.btc.unwrap_or_default())?;

  let mut height_info = HeightInfo {
    ord_height: None,
    btc_chain_info: btc_info,
  };
  if !ord_height.is_none() {
    height_info.ord_height = Some(ord_height.unwrap().0);
  }
  Ok(Json(ApiResponse::ok(height_info)))
}

pub(crate) async fn brc20_tick_info(
  Extension(index): Extension<Arc<Index>>,
  Path(tick): Path<String>,
) -> ApiResult<TickInfo> {
  log::debug!("rpc: get brc20_tick_info: {}", tick);
  if tick.as_bytes().len() != 4 {
    return Err(ApiError::bad_request(ERR_TICK_LENGTH));
  }
  let tick = tick.to_lowercase();

  let tick_info = index.brc20_get_tick_info(&tick)?;

  log::debug!("rpc: get brc20_tick_info: {:?} {:?}", tick, tick_info);

  let tick_info = &tick_info.ok_or_api_notfound("tick not found")?;

  if tick_info.tick != brc20::Tick::from_str(&tick).unwrap() {
    return Err(ApiError::internal("db: not match"));
  }

  Ok(Json(ApiResponse::ok(tick_info.into())))
}

pub(crate) async fn brc20_all_tick_info(
  Extension(index): Extension<Arc<Index>>,
) -> ApiResult<AllTickInfo> {
  log::debug!("rpc: get brc20_all_tick_info");
  let all_tick_info = index.brc20_get_all_tick_info()?;
  log::debug!("rpc: get brc20_all_tick_info: {:?}", all_tick_info);

  Ok(Json(ApiResponse::ok(AllTickInfo {
    tokens: all_tick_info.iter().map(|t| t.into()).collect(),
  })))
}

pub(crate) async fn brc20_balance(
  Extension(index): Extension<Arc<Index>>,
  Path((tick, address)): Path<(String, String)>,
) -> ApiResult<Balance> {
  log::debug!("rpc: get brc20_balance: {} {}", tick, address);
  if tick.as_bytes().len() != 4 {
    return Err(ApiError::BadRequest(ERR_TICK_LENGTH.to_string()));
  }
  let tick = tick.to_lowercase();

  let address: bitcoin::Address = address
    .parse()
    .map_err(|e: bitcoin::util::address::Error| ApiError::bad_request(e.to_string()))?;

  let balance = index.brc20_get_balance_by_address(&tick, &address)?;

  let balance = balance.ok_or_api_notfound("balance not found")?;

  let available_balance = balance.overall_balance - balance.transferable_balance;
  if available_balance > balance.overall_balance {
    return Err(ApiError::internal("balance error"));
  }

  log::debug!("rpc: get brc20_balance: {} {} {:?}", tick, address, balance);

  Ok(Json(ApiResponse::ok(Balance {
    tick,
    available_balance: available_balance.to_string(),
    transferable_balance: balance.transferable_balance.to_string(),
    overall_balance: balance.overall_balance.to_string(),
  })))
}

pub(crate) async fn brc20_all_balance(
  Extension(index): Extension<Arc<Index>>,
  Path(address): Path<String>,
) -> ApiResult<AllBalance> {
  log::debug!("rpc: get brc20_all_balance: {}", address);

  let address: bitcoin::Address = address
    .parse()
    .map_err(|e: bitcoin::util::address::Error| ApiError::bad_request(e.to_string()))?;

  let all_balance = index.brc20_get_all_balance_by_address(&address)?;

  log::debug!("rpc: get brc20_all_balance: {} {:?}", address, all_balance);

  Ok(Json(ApiResponse::ok(AllBalance {
    balance: all_balance
      .iter()
      .map(|(tick, bal)| Balance {
        tick: std::str::from_utf8(tick.as_bytes()).unwrap().to_string(),
        available_balance: (bal.overall_balance - bal.transferable_balance).to_string(),
        transferable_balance: bal.transferable_balance.to_string(),
        overall_balance: bal.overall_balance.to_string(),
      })
      .collect(),
  })))
}

pub(crate) async fn brc20_tx(
  Extension(index): Extension<Arc<Index>>,
  Path(txid): Path<String>,
) -> ApiResult<Brc20Transaction> {
  log::debug!("rpc: get brc20_tx: {}", txid);
  let txid = bitcoin::Txid::from_str(&txid).map_err(|e| ApiError::bad_request(e.to_string()))?;

  let tx_info = index
    .get_transaction_info(&txid)?
    .ok_or_api_notfound("tx not found")?;

  let tx = tx_info
    .transaction()
    .map_err(|e| ApiError::internal(e.to_string()))?;

  let operations = operation::get_brc20_operations(Extension(index), &tx)?;

  if operations.is_empty() {
    return Err(ApiError::not_found("brc20 operation not found"));
  }
  log::debug!("rpc: get brc20_tx: {} {:?}", txid, operations);

  Ok(Json(ApiResponse::ok(Brc20Transaction {
    txid: txid.to_string(),
    isconfirmed: tx_info.confirmations.is_some(),
    operations,
  })))
}

pub(crate) async fn brc20_tx_events(
  Extension(index): Extension<Arc<Index>>,
  Path(txid): Path<String>,
) -> ApiResult<TxEvents> {
  log::debug!("rpc: get brc20_tx_events: {}", txid);
  let txid = bitcoin::Txid::from_str(&txid).map_err(|e| ApiError::bad_request(e.to_string()))?;
  let tx_events = index
    .brc20_get_tx_events_by_txid(&txid)?
    .ok_or_api_notfound("tx events not found")?;

  log::debug!("rpc: get brc20_tx_events: {} {:?}", txid, tx_events);

  Ok(Json(ApiResponse::ok(TxEvents {
    txid: txid.to_string(),
    events: tx_events.iter().map(|e| e.into()).collect(),
  })))
}

pub(crate) async fn brc20_block_events(
  Extension(index): Extension<Arc<Index>>,
  Path(block_hash): Path<String>,
) -> ApiResult<BlockEvents> {
  log::debug!("rpc: get brc20_block_events: {}", block_hash);

  let blockhash =
    bitcoin::BlockHash::from_str(&block_hash).map_err(|e| ApiError::bad_request(e.to_string()))?;

  let block_events = index
    .brc20_get_block_events_by_blockhash(blockhash)?
    .ok_or_api_notfound("block not found")?;

  log::debug!(
    "rpc: get brc20_block_events: {} {:?}",
    block_hash,
    block_events
  );

  Ok(Json(ApiResponse::ok(BlockEvents {
    block: block_events
      .iter()
      .map(|(txid, events)| TxEvents {
        txid: txid.to_string(),
        events: events.iter().map(|e| e.into()).collect(),
      })
      .collect(),
  })))
}

pub(crate) async fn brc20_transferable(
  Extension(index): Extension<Arc<Index>>,
  Path((tick, address)): Path<(String, String)>,
) -> ApiResult<TransferableInscriptions> {
  log::debug!("rpc: get brc20_transferable: {} {}", tick, address);
  if tick.as_bytes().len() != 4 {
    return Err(ApiError::bad_request(ERR_TICK_LENGTH));
  }
  let tick = tick.to_lowercase();

  let address: bitcoin::Address = address
    .parse()
    .map_err(|err: bitcoin::util::address::Error| ApiError::bad_request(err.to_string()))?;

  let transferable = index.brc20_get_tick_transferable_by_address(&tick, &address)?;
  log::debug!(
    "rpc: get brc20_transferable: {} {} {:?}",
    tick,
    address,
    transferable
  );

  Ok(Json(ApiResponse::ok(TransferableInscriptions {
    inscriptions: transferable.iter().map(|trans| trans.into()).collect(),
  })))
}

pub(crate) async fn brc20_all_transferable(
  Extension(index): Extension<Arc<Index>>,
  Path(address): Path<String>,
) -> ApiResult<TransferableInscriptions> {
  log::debug!("rpc: get brc20_all_transferable: {}", address);
  let address: bitcoin::Address = address
    .parse()
    .map_err(|err: bitcoin::util::address::Error| ApiError::bad_request(err.to_string()))?;

  let transferable = index.brc20_get_all_transferable_by_address(&address)?;
  log::debug!(
    "rpc: get brc20_all_transferable: {} {:?}",
    address,
    transferable
  );

  Ok(Json(ApiResponse::ok(TransferableInscriptions {
    inscriptions: transferable.iter().map(|trans| trans.into()).collect(),
  })))
}

fn ord_get_inscription_by_id(index: Arc<Index>, id: InscriptionId) -> ApiResult<OrdInscription> {
  let inscription_data = index
    .get_inscription_all_data_by_id(id)?
    .ok_or_api_notfound("inscription not found")?;

  Ok(Json(ApiResponse::ok(OrdInscription {
    id: id.to_string(),
    number: inscription_data.entry.number.to_string(),
    content_type: inscription_data
      .inscription
      .content_type()
      .map(|c| String::from(c)),
    content: inscription_data.inscription.body().map(|c| hex::encode(c)),
    owner: brc20::ScriptKey::from_script(
      &inscription_data.tx.output[0].script_pubkey,
      index.get_chain_network(),
    )
    .to_string(),
    genesis_height: inscription_data.entry.height,
    location: inscription_data.sat_point.to_string(),
    sat: inscription_data.entry.sat.map(|s| s.0),
  })))
}

pub(crate) async fn ord_inscription_id(
  Extension(index): Extension<Arc<Index>>,
  Path(id): Path<String>,
) -> ApiResult<OrdInscription> {
  log::debug!("rpc: get ord_inscription_id: {}", id);
  let id = InscriptionId::from_str(&id).map_err(|e| ApiError::bad_request(e.to_string()))?;

  ord_get_inscription_by_id(index, id)
}

pub(crate) async fn ord_inscription_number(
  Extension(index): Extension<Arc<Index>>,
  Path(number): Path<u64>,
) -> ApiResult<OrdInscription> {
  log::debug!("rpc: get ord_inscription_number: {}", number);

  let id = index
    .get_inscription_id_by_inscription_number(number)?
    .ok_or_api_notfound("inscription not found")?;

  ord_get_inscription_by_id(index, id)
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutPointData {
  pub txid: String,
  pub script_pub_key: String,
  pub address: Option<String>,
  pub value: u64,
  pub inscription_digest: Vec<InscriptionDigest>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InscriptionDigest {
  pub id: String,
  pub number: u64,
  pub location: String,
}

pub(crate) async fn ord_outpoint(
  Extension(index): Extension<Arc<Index>>,
  Path(outpoint): Path<OutPoint>,
) -> ApiResult<OutPointData> {
  log::debug!("rpc: get ord_outpoint: {}", outpoint);

  let inscription_ids = index.get_inscriptions_on_output(outpoint)?;
  if inscription_ids.is_empty() {
    return Err(ApiError::not_found("inscription not found"));
  }

  let tx = index
    .get_transaction(outpoint.txid)?
    .ok_or_api_notfound("transaction not found")?;

  let vout = tx
    .output
    .get(outpoint.vout as usize)
    .ok_or_api_notfound("vout not found")?;

  let mut inscription_digests = Vec::with_capacity(inscription_ids.len());
  for id in &inscription_ids {
    let ins_data = index
      .get_inscription_entry(id.clone())?
      .ok_or_api_notfound("inscription not found")?;

    let satpoint = index
      .get_inscription_satpoint_by_id(id.clone())?
      .ok_or_api_notfound("inscription not found")?;

    inscription_digests.push(InscriptionDigest {
      id: id.to_string(),
      number: ins_data.number,
      location: satpoint.to_string(),
    });
  }

  Ok(Json(ApiResponse::ok(OutPointData {
    txid: outpoint.txid.to_string(),
    script_pub_key: vout.script_pubkey.asm(),
    address: match brc20::ScriptKey::from_script(&vout.script_pubkey, index.get_chain_network()) {
      brc20::ScriptKey::Address(address) => Some(address.to_string()),
      _ => None,
    },
    value: vout.value,
    inscription_digest: inscription_digests,
  })))
}
