use {
  super::{error::ApiError, *},
  crate::okx::entry::{Action, InscriptionReceipt},
  axum::Json,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ApiInscriptionAction {
  New { cursed: bool, unbound: bool },
  Transfer,
}

impl From<Action> for ApiInscriptionAction {
  fn from(action: Action) -> Self {
    match action {
      Action::Created { charms } => ApiInscriptionAction::New {
        cursed: Charm::Cursed.is_set(charms),
        unbound: Charm::Unbound.is_set(charms),
      },
      Action::Transferred => ApiInscriptionAction::Transfer,
    }
  }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiTxInscription {
  pub action: ApiInscriptionAction,
  pub inscription_number: i32,
  pub inscription_id: InscriptionId,
  pub old_satpoint: SatPoint,
  pub new_satpoint: SatPoint,
  pub from: ApiUtxoAddress,
  pub to: Option<ApiUtxoAddress>,
}

impl From<InscriptionReceipt> for ApiTxInscription {
  fn from(value: InscriptionReceipt) -> Self {
    ApiTxInscription {
      from: ApiUtxoAddress::from(value.sender),
      to: value.receiver.map(ApiUtxoAddress::from),
      action: ApiInscriptionAction::from(value.action),
      inscription_number: value.inscription_number,
      inscription_id: value.inscription_id,
      old_satpoint: value.old_satpoint,
      new_satpoint: value.new_satpoint,
    }
  }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiTxInscriptions {
  pub inscriptions: Vec<ApiTxInscription>,
  pub txid: Txid,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiBlockInscriptions {
  pub block: Vec<ApiTxInscriptions>,
}

// ord/tx/:txid/inscriptions
/// Retrieve the inscription actions from the given transaction.

pub(crate) async fn ord_txid_inscriptions(
  Extension(index): Extension<Arc<Index>>,
  Path(txid): Path<String>,
) -> ApiResult<ApiTxInscriptions> {
  log::debug!("rpc: get ord_txid_inscriptions: {}", txid);
  task::block_in_place(|| {
    let txid = Txid::from_str(&txid).map_err(ApiError::bad_request)?;
    let rtx = index.begin_read()?;

    let inscription_receipts = Index::ord_get_raw_receipts(&txid, &rtx)?
      .ok_or(OrdApiError::TransactionReceiptNotFound(txid))?;
    log::debug!(
      "rpc: get ord_txid_inscriptions: {} {:?}",
      txid,
      inscription_receipts
    );

    Ok(Json(ApiResponse::ok(ApiTxInscriptions {
      inscriptions: inscription_receipts.into_iter().map(Into::into).collect(),
      txid,
    })))
  })
}

// ord/block/:blockhash/inscriptions
/// Retrieve the inscription actions from the given block.
pub(crate) async fn ord_block_inscriptions(
  Extension(index): Extension<Arc<Index>>,
  Path(blockhash): Path<String>,
) -> ApiResult<ApiBlockInscriptions> {
  log::debug!("rpc: get ord_block_inscriptions: {}", blockhash);
  task::block_in_place(|| {
    let blockhash = BlockHash::from_str(&blockhash).map_err(ApiError::bad_request)?;

    let rtx = index.begin_read()?;

    let block_info = index
      .client
      .get_block_info(&blockhash)
      .map_err(ApiError::internal)?;

    let Some(db_blockhash) = rtx.block_hash(Some(u32::try_from(block_info.height).unwrap()))?
    else {
      return Err(OrdApiError::BlockReceiptNotFound(block_info.hash).into());
    };

    // check of conflicting block.
    if block_info.hash != db_blockhash || blockhash != block_info.hash {
      return Err(
        OrdApiError::ConflictBlockByHeight(Height(u32::try_from(block_info.height).unwrap()))
          .into(),
      );
    }

    let mut block_receipts = Vec::new();
    for txid in block_info.tx {
      let Some(tx_receipts) = Index::ord_get_raw_receipts(&txid, &rtx)? else {
        continue;
      };
      block_receipts.push((txid, tx_receipts));
    }

    log::debug!(
      "rpc: get ord_block_inscriptions: {} {:?}",
      blockhash,
      block_receipts
    );

    Ok(Json(ApiResponse::ok(ApiBlockInscriptions {
      block: block_receipts
        .into_iter()
        .map(|(txid, receipts)| ApiTxInscriptions {
          inscriptions: receipts.into_iter().map(Into::into).collect(),
          txid,
        })
        .collect(),
    })))
  })
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::{txid, InscriptionId, SatPoint};
  use std::str::FromStr;

  #[test]
  fn serialize_ord_inscriptions() {
    let mut tx_inscription = ApiTxInscription {
      from: UtxoAddress::from_script(
        &Address::from_str("bc1qhvd6suvqzjcu9pxjhrwhtrlj85ny3n2mqql5w4")
          .unwrap()
          .assume_checked()
          .script_pubkey(),
        &Chain::Mainnet,
      )
      .into(),
      to: Some(
        UtxoAddress::from_script(
          &Address::from_str("bc1qhvd6suvqzjcu9pxjhrwhtrlj85ny3n2mqql5w4")
            .unwrap()
            .assume_checked()
            .script_pubkey(),
          &Chain::Mainnet,
        )
        .into(),
      ),
      action: ApiInscriptionAction::New {
        cursed: false,
        unbound: false,
      },
      inscription_number: 100,
      inscription_id: InscriptionId {
        txid: txid(1),
        index: 0xFFFFFFFF,
      },
      old_satpoint: SatPoint::from_str(
        "5660d06bd69326c18ec63127b37fb3b32ea763c3846b3334c51beb6a800c57d3:1:3000",
      )
      .unwrap(),
      new_satpoint: SatPoint::from_str(
        "5660d06bd69326c18ec63127b37fb3b32ea763c3846b3334c51beb6a800c57d3:1:3000",
      )
      .unwrap(),
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
