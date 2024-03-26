use {
  super::{error::ApiError, types::ScriptPubkey, *},
  crate::{
    index::rtx::Rtx,
    okx::datastore::{
      ord::{Action, InscriptionOp},
      ScriptKey,
    },
  },
  axum::Json,
  utoipa::ToSchema,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[schema(as = ord::ApiInscriptionAction)]
#[serde(rename_all = "camelCase")]
pub enum ApiInscriptionAction {
  /// New inscription
  New { cursed: bool, unbound: bool },
  /// Transfer inscription
  Transfer,
}

impl From<Action> for ApiInscriptionAction {
  fn from(action: Action) -> Self {
    match action {
      Action::New {
        cursed, unbound, ..
      } => ApiInscriptionAction::New { cursed, unbound },
      Action::Transfer => ApiInscriptionAction::Transfer,
    }
  }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[schema(as = ord::ApiTxInscription)]
#[serde(rename_all = "camelCase")]
pub struct ApiTxInscription {
  /// The action of the inscription.
  #[schema(value_type = ord::ApiInscriptionAction)]
  pub action: ApiInscriptionAction,
  /// The inscription number.
  pub inscription_number: Option<i32>,
  /// The inscription id.
  pub inscription_id: String,
  /// The inscription satpoint of the transaction input.
  pub old_satpoint: String,
  /// The inscription satpoint of the transaction output.
  pub new_satpoint: Option<String>,
  /// The message sender which is an address or script pubkey hash.
  pub from: ScriptPubkey,
  /// The message receiver which is an address or script pubkey hash.
  pub to: Option<ScriptPubkey>,
}

impl ApiTxInscription {
  pub(super) fn parse_from_operation(
    operation: InscriptionOp,
    rtx: &Rtx,
    client: &Client,
    chain: Chain,
    index_transactions: bool,
  ) -> Result<Self> {
    let prevout = Index::fetch_vout(
      rtx,
      client,
      operation.old_satpoint.outpoint,
      chain,
      index_transactions,
    )?
    .ok_or(OrdApiError::Internal(format!(
      "Failed to get inscription prevout: {}",
      operation.old_satpoint.outpoint
    )))?;

    let output = match operation.new_satpoint {
      Some(new_satpoint) if new_satpoint.outpoint != unbound_outpoint() => Index::fetch_vout(
        rtx,
        client,
        new_satpoint.outpoint,
        chain,
        index_transactions,
      )?,
      _ => None,
    };

    Ok(ApiTxInscription {
      from: ScriptKey::from_script(&prevout.script_pubkey, chain).into(),
      to: output.map(|v| ScriptKey::from_script(&v.script_pubkey, chain).into()),
      action: operation.action.into(),
      inscription_number: operation.inscription_number,
      inscription_id: operation.inscription_id.to_string(),
      old_satpoint: operation.old_satpoint.to_string(),
      new_satpoint: operation.new_satpoint.map(|v| v.to_string()),
    })
  }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[schema(as = ord::ApiTxInscriptions)]
#[serde(rename_all = "camelCase")]
pub struct ApiTxInscriptions {
  #[schema(value_type = Vec<ord::ApiTxInscription>)]
  pub inscriptions: Vec<ApiTxInscription>,
  pub txid: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
#[schema(as = ord::ApiBlockInscriptions)]
#[serde(rename_all = "camelCase")]
pub struct ApiBlockInscriptions {
  #[schema(value_type = Vec<ord::ApiTxInscriptions>)]
  pub block: Vec<ApiTxInscriptions>,
}

// ord/tx/:txid/inscriptions
/// Retrieve the inscription actions from the given transaction.
#[utoipa::path(
  get,
  path = "/api/v1/ord/tx/{txid}/inscriptions",
  params(
      ("txid" = String, Path, description = "transaction ID")
),
  responses(
    (status = 200, description = "Obtain inscription actions by txid", body = OrdTxInscriptions),
    (status = 400, description = "Bad query.", body = ApiError, example = json!(&ApiError::bad_request("bad request"))),
    (status = 404, description = "Not found.", body = ApiError, example = json!(&ApiError::not_found("not found"))),
    (status = 500, description = "Internal server error.", body = ApiError, example = json!(&ApiError::internal("internal error"))),
  )
)]
pub(crate) async fn ord_txid_inscriptions(
  Extension(index): Extension<Arc<Index>>,
  Path(txid): Path<String>,
) -> ApiResult<ApiTxInscriptions> {
  log::debug!("rpc: get ord_txid_inscriptions: {}", txid);
  let txid = Txid::from_str(&txid).map_err(ApiError::bad_request)?;
  let rtx = index.begin_read()?;
  let client = index.bitcoin_rpc_client()?;
  let index_transactions = index.has_transactions_index();

  let operations = Index::get_ord_inscription_operations(txid, &rtx, &client)?
    .ok_or(OrdApiError::TransactionReceiptNotFound(txid))?;
  log::debug!("rpc: get ord_txid_inscriptions: {:?}", operations);

  let mut api_tx_inscriptions = Vec::new();
  for operation in operations.into_iter() {
    let tx_inscription = ApiTxInscription::parse_from_operation(
      operation,
      &rtx,
      &client,
      index.get_chain(),
      index_transactions,
    )?;
    api_tx_inscriptions.push(tx_inscription);
  }

  Ok(Json(ApiResponse::ok(ApiTxInscriptions {
    inscriptions: api_tx_inscriptions,
    txid: txid.to_string(),
  })))
}

// ord/block/:blockhash/inscriptions
/// Retrieve the inscription actions from the given block.
#[utoipa::path(
  get,
  path = "/api/v1/ord/block/{blockhash}/inscriptions",
  params(
      ("blockhash" = String, Path, description = "block hash")
),
  responses(
    (status = 200, description = "Obtain inscription actions by blockhash", body = OrdBlockInscriptions),
    (status = 400, description = "Bad query.", body = ApiError, example = json!(&ApiError::bad_request("bad request"))),
    (status = 404, description = "Not found.", body = ApiError, example = json!(&ApiError::not_found("not found"))),
    (status = 500, description = "Internal server error.", body = ApiError, example = json!(&ApiError::internal("internal error"))),
  )
)]
pub(crate) async fn ord_block_inscriptions(
  Extension(index): Extension<Arc<Index>>,
  Path(blockhash): Path<String>,
) -> ApiResult<ApiBlockInscriptions> {
  log::debug!("rpc: get ord_block_inscriptions: {}", blockhash);

  let blockhash = bitcoin::BlockHash::from_str(&blockhash).map_err(ApiError::bad_request)?;
  let rtx = index.begin_read()?;
  let client = index.bitcoin_rpc_client()?;
  let index_transactions = index.has_transactions_index();

  let block_operations = Index::get_ord_block_inscription_operations(blockhash, &rtx, &client)?;
  log::debug!("rpc: get ord_block_inscriptions: {:?}", block_operations);

  let mut api_block_operations = Vec::new();
  for (txid, tx_operations) in block_operations.into_iter() {
    let mut api_tx_operations = Vec::new();
    for operation in tx_operations.into_iter() {
      let tx_inscription = ApiTxInscription::parse_from_operation(
        operation,
        &rtx,
        &client,
        index.get_chain(),
        index_transactions,
      )?;
      api_tx_operations.push(tx_inscription);
    }
    api_block_operations.push(ApiTxInscriptions {
      inscriptions: api_tx_operations,
      txid: txid.to_string(),
    });
  }

  Ok(Json(ApiResponse::ok(ApiBlockInscriptions {
    block: api_block_operations,
  })))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{txid, InscriptionId, SatPoint};
  use std::str::FromStr;

  #[test]
  fn serialize_ord_inscriptions() {
    let mut tx_inscription = ApiTxInscription {
      from: ScriptKey::from_script(
        &Address::from_str("bc1qhvd6suvqzjcu9pxjhrwhtrlj85ny3n2mqql5w4")
          .unwrap()
          .assume_checked()
          .script_pubkey(),
        Chain::Mainnet,
      )
      .into(),
      to: Some(
        ScriptKey::from_script(
          &Address::from_str("bc1qhvd6suvqzjcu9pxjhrwhtrlj85ny3n2mqql5w4")
            .unwrap()
            .assume_checked()
            .script_pubkey(),
          Chain::Mainnet,
        )
        .into(),
      ),
      action: ApiInscriptionAction::New {
        cursed: false,
        unbound: false,
      },
      inscription_number: Some(100),
      inscription_id: InscriptionId {
        txid: txid(1),
        index: 0xFFFFFFFF,
      }
      .to_string(),
      old_satpoint: SatPoint::from_str(
        "5660d06bd69326c18ec63127b37fb3b32ea763c3846b3334c51beb6a800c57d3:1:3000",
      )
      .unwrap()
      .to_string(),

      new_satpoint: Some(
        SatPoint::from_str(
          "5660d06bd69326c18ec63127b37fb3b32ea763c3846b3334c51beb6a800c57d3:1:3000",
        )
        .unwrap()
        .to_string(),
      ),
    };
    assert_eq!(
      serde_json::to_string_pretty(&tx_inscription).unwrap(),
      r#"{
  "action": {
    "new": {
      "cursed": false,
      "unbound": false
    }
  },
  "inscriptionNumber": 100,
  "inscriptionId": "1111111111111111111111111111111111111111111111111111111111111111i4294967295",
  "oldSatpoint": "5660d06bd69326c18ec63127b37fb3b32ea763c3846b3334c51beb6a800c57d3:1:3000",
  "newSatpoint": "5660d06bd69326c18ec63127b37fb3b32ea763c3846b3334c51beb6a800c57d3:1:3000",
  "from": {
    "address": "bc1qhvd6suvqzjcu9pxjhrwhtrlj85ny3n2mqql5w4"
  },
  "to": {
    "address": "bc1qhvd6suvqzjcu9pxjhrwhtrlj85ny3n2mqql5w4"
  }
}"#,
    );
    tx_inscription.action = ApiInscriptionAction::Transfer;
    assert_eq!(
      serde_json::to_string_pretty(&tx_inscription).unwrap(),
      r#"{
  "action": "transfer",
  "inscriptionNumber": 100,
  "inscriptionId": "1111111111111111111111111111111111111111111111111111111111111111i4294967295",
  "oldSatpoint": "5660d06bd69326c18ec63127b37fb3b32ea763c3846b3334c51beb6a800c57d3:1:3000",
  "newSatpoint": "5660d06bd69326c18ec63127b37fb3b32ea763c3846b3334c51beb6a800c57d3:1:3000",
  "from": {
    "address": "bc1qhvd6suvqzjcu9pxjhrwhtrlj85ny3n2mqql5w4"
  },
  "to": {
    "address": "bc1qhvd6suvqzjcu9pxjhrwhtrlj85ny3n2mqql5w4"
  }
}"#,
    );
  }
}
