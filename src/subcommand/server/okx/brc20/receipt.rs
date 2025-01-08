use super::*;
use crate::okx::brc20::{
  event::{BRC20Event, BRC20OpType},
  BRC20Receipt,
};

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
    match event.result {
      Ok(BRC20Event::Deploy(deploy_event)) => Self::Deploy(ApiDeployEvent {
        inscription_id: event.inscription_id,
        inscription_number: event.inscription_number,
        old_satpoint: event.old_satpoint,
        new_satpoint: event.new_satpoint,
        from: event.sender.into(),
        to: event.receiver.into(),
        tick: deploy_event.ticker,
        supply: deploy_event.total_supply.to_string(),
        limit_per_mint: deploy_event.max_mint_limit.to_string(),
        decimal: deploy_event.decimals,
        self_mint: deploy_event.self_minted,
        valid: true,
        msg: "ok".to_string(),
        event: event.op_type,
      }),
      Ok(BRC20Event::Mint(mint_event)) => Self::Mint(ApiMintEvent {
        inscription_id: event.inscription_id,
        inscription_number: event.inscription_number,
        old_satpoint: event.old_satpoint,
        new_satpoint: event.new_satpoint,
        from: event.sender.into(),
        to: event.receiver.into(),
        valid: true,
        tick: mint_event.ticker,
        amount: mint_event.amount.to_string(),
        msg: "ok".to_string(),
        event: event.op_type,
      }),
      Ok(BRC20Event::InscribeTransfer(inscribe_transfer_event)) => {
        Self::InscribeTransfer(ApiInscribeTransferEvent {
          inscription_id: event.inscription_id,
          inscription_number: event.inscription_number,
          old_satpoint: event.old_satpoint,
          new_satpoint: event.new_satpoint,
          from: event.sender.into(),
          to: event.receiver.into(),
          valid: true,
          tick: inscribe_transfer_event.ticker,
          amount: inscribe_transfer_event.amount.to_string(),
          msg: "ok".to_string(),
          event: event.op_type,
        })
      }
      Ok(BRC20Event::Transfer(transfer_event)) => Self::Transfer(ApiTransferEvent {
        inscription_id: event.inscription_id,
        inscription_number: event.inscription_number,
        old_satpoint: event.old_satpoint,
        new_satpoint: event.new_satpoint,
        from: event.sender.into(),
        to: event.receiver.into(),
        valid: true,
        tick: transfer_event.ticker,
        amount: transfer_event.amount.to_string(),
        msg: "ok".to_string(),
        event: event.op_type,
      }),
      Err(err) => Self::Error(ApiErrorEvent {
        inscription_id: event.inscription_id,
        inscription_number: event.inscription_number,
        old_satpoint: event.old_satpoint,
        new_satpoint: event.new_satpoint,
        from: event.sender.into(),
        to: event.receiver.into(),
        valid: false,
        msg: err.to_string(),
        event: event.op_type,
      }),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiDeployEvent {
  #[serde(rename = "type")]
  pub event: BRC20OpType,
  pub tick: BRC20Ticker,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiMintEvent {
  #[serde(rename = "type")]
  pub event: BRC20OpType,
  pub tick: BRC20Ticker,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiInscribeTransferEvent {
  #[serde(rename = "type")]
  pub event: BRC20OpType,
  pub tick: BRC20Ticker,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiTransferEvent {
  #[serde(rename = "type")]
  pub event: BRC20OpType,
  pub tick: BRC20Ticker,
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
  task::block_in_place(|| {
    let txid = Txid::from_str(&txid).map_err(ApiError::bad_request)?;
    let rtx = index.begin_read()?;

    let receipts = Index::brc20_get_raw_receipts(&txid, &rtx)?
      .ok_or(BRC20ApiError::TransactionReceiptNotFound(txid))?;

    log::debug!("rpc: get brc20_tx_events: {} {:?}", txid, receipts);

    Ok(Json(ApiResponse::ok(ApiTxEvents {
      txid,
      events: receipts.into_iter().map(|e| e.into()).collect(),
    })))
  })
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
  task::block_in_place(|| {
    let blockhash = BlockHash::from_str(&blockhash).map_err(ApiError::bad_request)?;

    let rtx = index.begin_read()?;

    let block_info = index
      .client
      .get_block_info(&blockhash)
      .map_err(ApiError::internal)?;

    let Some(db_blockhash) = rtx.block_hash(Some(u32::try_from(block_info.height).unwrap()))?
    else {
      return Err(BRC20ApiError::BlockReceiptNotFound(block_info.hash).into());
    };

    // check of conflicting block.
    if block_info.hash != db_blockhash || blockhash != block_info.hash {
      return Err(
        BRC20ApiError::ConflictBlockByHeight(Height(u32::try_from(block_info.height).unwrap()))
          .into(),
      );
    }

    let mut block_receipts = Vec::new();
    for txid in block_info.tx {
      let Some(tx_receipts) = Index::brc20_get_raw_receipts(&txid, &rtx)? else {
        continue;
      };
      block_receipts.push((txid, tx_receipts));
    }

    log::debug!(
      "rpc: get brc20_block_events: {} {:?}",
      blockhash,
      block_receipts
    );

    Ok(Json(ApiResponse::ok(ApiBlockEvents {
      block: block_receipts
        .into_iter()
        .map(|(txid, events)| ApiTxEvents {
          txid,
          events: events.into_iter().map(|e| e.into()).collect(),
        })
        .filter(|e| !e.events.is_empty())
        .collect(),
    })))
  })
}
#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn test_serialize_api_event() {
    let deploy = ApiDeployEvent {
      event: BRC20OpType::Deploy,
      tick: BRC20Ticker::from_str("ordi").unwrap(),
      inscription_id: Default::default(),
      inscription_number: 0,
      old_satpoint: Default::default(),
      new_satpoint: Default::default(),
      supply: 100.to_string(),
      limit_per_mint: 10.to_string(),
      decimal: 0,
      self_mint: false,
      from: UtxoAddress::from_str(
        "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq",
        Network::Bitcoin,
      )
      .unwrap()
      .into(),
      to: UtxoAddress::from_str(
        "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq",
        Network::Bitcoin,
      )
      .unwrap()
      .into(),
      valid: true,
      msg: "ok".to_string(),
    };

    assert_eq!(
      serde_json::to_string_pretty(&deploy).unwrap(),
      r#"{
  "type": "deploy",
  "tick": "ordi",
  "inscriptionId": "0000000000000000000000000000000000000000000000000000000000000000i0",
  "inscriptionNumber": 0,
  "oldSatpoint": "0000000000000000000000000000000000000000000000000000000000000000:4294967295:0",
  "newSatpoint": "0000000000000000000000000000000000000000000000000000000000000000:4294967295:0",
  "supply": "100",
  "limitPerMint": "10",
  "decimal": 0,
  "selfMint": false,
  "from": {
    "address": "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq"
  },
  "to": {
    "address": "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq"
  },
  "valid": true,
  "msg": "ok"
}"#
    );

    let mint = ApiMintEvent {
      event: BRC20OpType::Mint,
      tick: BRC20Ticker::from_str("ordi").unwrap(),
      inscription_id: Default::default(),
      inscription_number: 0,
      old_satpoint: Default::default(),
      new_satpoint: Default::default(),
      amount: 10.to_string(),
      from: UtxoAddress::from_str(
        "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq",
        Network::Bitcoin,
      )
      .unwrap()
      .into(),
      to: UtxoAddress::from_str(
        "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq",
        Network::Bitcoin,
      )
      .unwrap()
      .into(),
      valid: true,
      msg: "ok".to_string(),
    };

    assert_eq!(
      serde_json::to_string_pretty(&mint).unwrap(),
      r#"{
  "type": "mint",
  "tick": "ordi",
  "inscriptionId": "0000000000000000000000000000000000000000000000000000000000000000i0",
  "inscriptionNumber": 0,
  "oldSatpoint": "0000000000000000000000000000000000000000000000000000000000000000:4294967295:0",
  "newSatpoint": "0000000000000000000000000000000000000000000000000000000000000000:4294967295:0",
  "amount": "10",
  "from": {
    "address": "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq"
  },
  "to": {
    "address": "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq"
  },
  "valid": true,
  "msg": "ok"
}"#
    );

    let inscribe_transfer = ApiInscribeTransferEvent {
      event: BRC20OpType::InscribeTransfer,
      tick: BRC20Ticker::from_str("ordi").unwrap(),
      inscription_id: Default::default(),
      inscription_number: 0,
      old_satpoint: Default::default(),
      new_satpoint: Default::default(),
      amount: 10.to_string(),
      from: UtxoAddress::from_str(
        "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq",
        Network::Bitcoin,
      )
      .unwrap()
      .into(),
      to: UtxoAddress::from_str(
        "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq",
        Network::Bitcoin,
      )
      .unwrap()
      .into(),
      valid: true,
      msg: "ok".to_string(),
    };

    assert_eq!(
      serde_json::to_string_pretty(&inscribe_transfer).unwrap(),
      r#"{
  "type": "inscribeTransfer",
  "tick": "ordi",
  "inscriptionId": "0000000000000000000000000000000000000000000000000000000000000000i0",
  "inscriptionNumber": 0,
  "oldSatpoint": "0000000000000000000000000000000000000000000000000000000000000000:4294967295:0",
  "newSatpoint": "0000000000000000000000000000000000000000000000000000000000000000:4294967295:0",
  "amount": "10",
  "from": {
    "address": "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq"
  },
  "to": {
    "address": "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq"
  },
  "valid": true,
  "msg": "ok"
}"#
    );

    let transfer = ApiTransferEvent {
      event: BRC20OpType::Transfer,
      tick: BRC20Ticker::from_str("ordi").unwrap(),
      inscription_id: Default::default(),
      inscription_number: 0,
      old_satpoint: Default::default(),
      new_satpoint: Default::default(),
      amount: 10.to_string(),
      from: UtxoAddress::from_str(
        "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq",
        Network::Bitcoin,
      )
      .unwrap()
      .into(),
      to: UtxoAddress::from_str(
        "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq",
        Network::Bitcoin,
      )
      .unwrap()
      .into(),
      valid: true,
      msg: "ok".to_string(),
    };

    assert_eq!(
      serde_json::to_string_pretty(&transfer).unwrap(),
      r#"{
  "type": "transfer",
  "tick": "ordi",
  "inscriptionId": "0000000000000000000000000000000000000000000000000000000000000000i0",
  "inscriptionNumber": 0,
  "oldSatpoint": "0000000000000000000000000000000000000000000000000000000000000000:4294967295:0",
  "newSatpoint": "0000000000000000000000000000000000000000000000000000000000000000:4294967295:0",
  "amount": "10",
  "from": {
    "address": "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq"
  },
  "to": {
    "address": "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq"
  },
  "valid": true,
  "msg": "ok"
}"#
    );

    let error = ApiErrorEvent {
      event: BRC20OpType::Deploy,
      inscription_id: Default::default(),
      inscription_number: 0,
      old_satpoint: Default::default(),
      new_satpoint: Default::default(),
      from: UtxoAddress::from_str(
        "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq",
        Network::Bitcoin,
      )
      .unwrap()
      .into(),
      to: UtxoAddress::from_str(
        "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq",
        Network::Bitcoin,
      )
      .unwrap()
      .into(),
      valid: false,
      msg: "err".to_string(),
    };

    assert_eq!(
      serde_json::to_string_pretty(&error).unwrap(),
      r#"{
  "type": "deploy",
  "inscriptionId": "0000000000000000000000000000000000000000000000000000000000000000i0",
  "inscriptionNumber": 0,
  "oldSatpoint": "0000000000000000000000000000000000000000000000000000000000000000:4294967295:0",
  "newSatpoint": "0000000000000000000000000000000000000000000000000000000000000000:4294967295:0",
  "from": {
    "address": "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq"
  },
  "to": {
    "address": "bc1qar0srrr7xfkvy5l643lydnw9re59gtzzwf5mdq"
  },
  "valid": false,
  "msg": "err"
}"#
    );
  }
}
