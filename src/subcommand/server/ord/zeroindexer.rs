use crate::okx::protocol::BlockContext;
use {
  super::{error::ApiError, *},
  crate::okx::datastore::{
    ord::{Action, InscriptionOp},
    ScriptKey,
  },
  crate::okx::protocol::zeroindexer::{
    error::JSONError,
    zerodata::{InscriptionContext, ZeroData, ZeroIndexerTx},
  },
  axum::Json,
  serde_json::Value,
};

// ord/block/:blockhash/inscriptions
/// Retrieve the inscription actions from the given block.
#[utoipa::path(
get,
path = "/api/v1/crawler/zeroindexer/:height",
params(
("height" = u32, Path, description = "block height")
),
responses(
(status = 200, description = "Obtain inscription actions by blockhash", body = OrdBlockInscriptions),
(status = 400, description = "Bad query.", body = ApiError, example = json!(&ApiError::bad_request("bad request"))),
(status = 404, description = "Not found.", body = ApiError, example = json!(&ApiError::not_found("not found"))),
(status = 500, description = "Internal server error.", body = ApiError, example = json!(&ApiError::internal("internal error"))),
)
)]
pub(crate) async fn crawler_zeroindexer_new(
  Extension(index): Extension<Arc<Index>>,
  Path(height): Path<u32>,
) -> ApiResult<ZeroData> {
  log::debug!("rpc: get crawler_zeroindexer: {}", height);

  let blockhash = index
    .block_hash(Some(height))
    .map_err(ApiError::internal)?
    .ok_or_api_not_found(OrdError::BlockNotFound)?;

  // get block from btc client.
  let blockinfo = index
    .get_block_info_by_hash(blockhash)
    .map_err(ApiError::internal)?
    .ok_or_api_not_found(OrdError::BlockNotFound)?;

  // get blockhash from redb.
  let blockhash = index
    .block_hash(Some(u32::try_from(blockinfo.height).unwrap()))
    .map_err(ApiError::internal)?
    .ok_or_api_not_found(OrdError::BlockNotFound)?;

  // check of conflicting block.
  if blockinfo.hash != blockhash {
    return Err(ApiError::NotFound(OrdError::BlockNotFound.to_string()));
  }

  let block_inscriptions = index
    .ord_get_txs_inscriptions(&blockinfo.tx)
    .map_err(ApiError::internal)?;

  log::debug!("rpc: get ord_block_inscriptions: {:?}", block_inscriptions);
  let mut txs: Vec<ZeroIndexerTx> = Vec::new();
  for (txid, ops) in block_inscriptions {
    match resolve_zero_inscription(
      BlockContext {
        network: index.get_chain_network(),
        blockheight: height,
        blocktime: blockinfo.time as u32,
      },
      &blockinfo.hash,
      txid,
      ops,
      &index,
    ) {
      Ok(mut message) => txs.append(&mut message),
      Err(e) => return Err(ApiError::internal(e)),
    };
  }
  let zero_data = ZeroData {
    block_height: height as u64,
    block_hash: blockinfo.hash.to_string(),
    prev_block_hash: match blockinfo.previousblockhash {
      None => "".to_string(),
      Some(hash) => hash.to_string(),
    },
    block_time: blockinfo.time as u32,
    txs,
  };
  Ok(Json(ApiResponse::ok(zero_data)))
}

// ord/block/:blockhash/inscriptions
/// Retrieve the inscription actions from the given block.
#[utoipa::path(
get,
path = "/api/v1/crawler/zeroindexer/:height",
params(
("height" = u64, Path, description = "block height")
),
responses(
(status = 200, description = "Obtain inscription actions by blockhash", body = OrdBlockInscriptions),
(status = 400, description = "Bad query.", body = ApiError, example = json!(&ApiError::bad_request("bad request"))),
(status = 404, description = "Not found.", body = ApiError, example = json!(&ApiError::not_found("not found"))),
(status = 500, description = "Internal server error.", body = ApiError, example = json!(&ApiError::internal("internal error"))),
)
)]
pub(crate) async fn crawler_zeroindexer(
  Extension(index): Extension<Arc<Index>>,
  Path(height): Path<u64>,
) -> ApiResult<ZeroData> {
  log::debug!("rpc: get crawler_zeroindexer: {}", height);

  match index.zero_indexer_get_txs(height) {
    Ok(data) => match data {
      Some(data) => Ok(Json(ApiResponse::ok(data))),
      None => Ok(Json(ApiResponse::err("block not found".to_string()))),
    },
    Err(e) => Err(ApiError::not_found(e)),
  }
}

fn resolve_zero_inscription(
  context: BlockContext,
  block_hash: &BlockHash,
  txid: Txid,
  operations: Vec<InscriptionOp>,
  index: &Arc<Index>,
) -> Result<Vec<ZeroIndexerTx>> {
  log::debug!(
    "Resolve Inscription indexed transaction {}, operations size: {}, data: {:?}",
    txid,
    operations.len(),
    operations
  );
  let tx = match index.get_transaction(txid) {
    Ok(tx) => match tx {
      Some(tx) => tx,
      None => return Err(anyhow!("tx not found")),
    },
    Err(e) => return Err(e),
  };
  let mut zero_indexer_txs: Vec<ZeroIndexerTx> = Vec::new();
  let mut operation_iter = operations.into_iter().peekable();
  let new_inscriptions = ParsedEnvelope::from_transaction(&tx);
  for input in &tx.input {
    // "operations" is a list of all the operations in the current block, and they are ordered.
    // We just need to find the operation corresponding to the current transaction here.
    while let Some(operation) = operation_iter.peek() {
      if operation.old_satpoint.outpoint != input.previous_output {
        break;
      }
      let operation = operation_iter.next().unwrap();

      let sat_in_outputs = operation
        .new_satpoint
        .map(|satpoint| satpoint.outpoint.txid == operation.txid)
        .unwrap_or(false);

      let mut is_transfer = false;
      let mut sender = "".to_string();
      let inscription = match operation.action {
        // New inscription is not `cursed` or `unbound`.
        Action::New {
          cursed: false,
          unbound: false,
          ..
        } => {
          let inscription_struct = new_inscriptions
            .get(usize::try_from(operation.inscription_id.index).unwrap())
            .unwrap()
            .clone()
            .payload;
          let des_res = deserialize_inscription(&inscription_struct);
          match des_res {
            Ok(content) => content,
            Err(_) => {
              continue;
            }
          }
        }
        // Transfer inscription operation.
        Action::Transfer => {
          if operation.inscription_id.txid == operation.old_satpoint.outpoint.txid
            && operation.inscription_id.index == operation.old_satpoint.outpoint.vout
          {
            is_transfer = true;

            let inscription_struct = match index.get_inscription_by_id(operation.inscription_id) {
              Ok(inner) => match inner {
                None => continue,
                Some(innner) => innner,
              },
              Err(err) => {
                return Err(anyhow!(
                  "failed to get inscription because btc is down:{}",
                  err
                ));
              }
            };
            let des_res = deserialize_inscription(&inscription_struct);
            match des_res {
              Ok(content) => {
                sender =
                  get_script_key_on_outpoint(&index, operation.old_satpoint.outpoint)?.to_string();
                content
              }
              Err(_) => {
                continue;
              }
            }
          } else {
            continue;
          }
        }
        _ => {
          continue;
        }
      };

      let new_sat_point = match operation.new_satpoint {
        None => "".to_string(),
        Some(sat_point) => sat_point.to_string(),
      };
      let receiver = if sat_in_outputs {
        match operation.new_satpoint {
          None => "".to_string(),
          Some(sat_point) => {
            match get_script_key_from_transaction(
              &tx,
              &sat_point.outpoint,
              index.get_chain_network(),
            ) {
              None => "".to_string(),
              Some(script_key) => script_key.to_string(),
            }
          }
        }
      } else {
        "".to_string()
      };
      let inscription_context = InscriptionContext {
        txid: operation.txid.to_string(),
        inscription_id: operation.inscription_id.to_string(),
        inscription_number: 0,
        old_sat_point: operation.old_satpoint.to_string(),
        new_sat_point,
        sender,
        receiver,
        is_transfer,
        block_height: context.blockheight as u64,
        block_time: context.blocktime,
        block_hash: block_hash.to_string(),
      };
      zero_indexer_txs.push(ZeroIndexerTx {
        protocol_name: "brc-20".to_string(),
        inscription,
        inscription_context: serde_json::to_string(&inscription_context).unwrap(),
        btc_txid: operation.txid.to_string(),
        btc_fee: "10000000".to_string(),
      });
    }
  }
  Ok(zero_indexer_txs)
}

#[utoipa::path(
get,
path = "/api/v1/crawler/height",
responses(
(status = 200, description = "Obtain inscription actions by blockhash", body = OrdBlockInscriptions),
(status = 400, description = "Bad query.", body = ApiError, example = json!(&ApiError::bad_request("bad request"))),
(status = 404, description = "Not found.", body = ApiError, example = json!(&ApiError::not_found("not found"))),
(status = 500, description = "Internal server error.", body = ApiError, example = json!(&ApiError::internal("internal error"))),
)
)]
pub(crate) async fn crawler_height(
  Extension(index): Extension<Arc<Index>>,
) -> ApiResult<FastSyncInfo> {
  log::debug!("rpc: get crawler_height");


  let (ord_height, _) = index.height_btc(false)?;

  let fast_sync_info = FastSyncInfo {
    crawler_height: ord_height.map(|h| h.0 as u64 ),
  };

  Ok(Json(ApiResponse::ok(fast_sync_info)))
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FastSyncInfo {
  #[schema(format = "uint64")]
  pub crawler_height: Option<u64>,
}

fn get_script_key_on_outpoint(index: &Arc<Index>, outpoint: OutPoint) -> Result<ScriptKey> {
  match index.get_outpoint_entry(outpoint) {
    Ok(tx_out) => match tx_out {
      None => Err(anyhow!("failed get_script_key_on_outpoint is none ")),
      Some(tx) => Ok(ScriptKey::from_script(
        &tx.script_pubkey,
        index.get_chain_network(),
      )),
    },
    Err(e) => Err(anyhow!("failed get_script_key_on_outpoint.error : {e} ")),
  }
}
//Some(ScriptKey::from_script(&tx_out.script_pubkey,network))
fn get_script_key_from_transaction(
  tx: &Transaction,
  outpoint: &OutPoint,
  network: Network,
) -> Option<ScriptKey> {
  if !tx.txid().eq(&outpoint.txid) {
    return None;
  }
  match tx.output.get(outpoint.vout as usize) {
    None => None,
    Some(tx_out) => Some(ScriptKey::from_script(&tx_out.script_pubkey, network)),
  }
}

fn deserialize_inscription(inscription: &Inscription) -> Result<String> {
  let content_body = std::str::from_utf8(inscription.body().ok_or(JSONError::InvalidJson)?)?;
  if content_body.len() == 0 {
    return Err(JSONError::InvalidJson.into());
  }

  let content_type = inscription
    .content_type()
    .ok_or(JSONError::InvalidContentType)?;

  if content_type != "text/plain"
    && content_type != "text/plain;charset=utf-8"
    && content_type != "text/plain;charset=UTF-8"
    && content_type != "application/json"
    && !content_type.starts_with("text/plain;")
  {
    return Err(JSONError::UnSupportContentType.into());
  }

  let value: Value = serde_json::from_str(content_body).map_err(|_| JSONError::InvalidJson)?;
  if value.get("p") == None || !value["p"].is_string() {
    return Err(JSONError::InvalidJson.into());
  }
  let protocol_name = match value.get("p") {
    None => return Err(JSONError::NotZeroIndexerJson.into()),
    Some(v) => v.to_string().replace("\"", ""),
  };
  if protocol_name != "brc-20".to_string() {
    return Err(JSONError::InvalidJson.into());
  }

  return Ok(serde_json::to_string(&value).unwrap());
}
