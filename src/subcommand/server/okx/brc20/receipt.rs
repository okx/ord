use {
  super::*,
  crate::okx::brc20::{
    event::{BRC20Event, BRC20OpType},
    BRC20Receipt,
  },
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
#[serde(rename_all = "camelCase")]
pub enum ApiTxEvent {
  PredeployEvent(ApiPredeployEvent),
  Deploy(ApiDeployEvent),
  Mint(ApiMintEvent),
  InscribeTransfer(ApiInscribeTransferEvent),
  Transfer(ApiTransferEvent),
  InscribeWithdraw(ApiInscribeWithdrawEvent),
  Withdraw(ApiWithdrawEvent),
  InscribeProgDeploy(ApiInscribeProgDeployEvent),
  InscribeProgCall(ApiInscribeProgCallEvent),
  InscribeProgTransact(ApiInscribeProgTransactEvent),
  ProgDeploy(ApiProgDeployEvent),
  ProgCall(ApiProgCallEvent),
  ProgTransact(ApiProgTransactEvent),
  Error(ApiErrorEvent),
}

impl From<BRC20Receipt> for ApiTxEvent {
  fn from(event: BRC20Receipt) -> Self {
    match event.result {
      Ok(BRC20Event::Predeploy(predeploy_event)) => Self::PredeployEvent(ApiPredeployEvent {
        inscription_id: event.inscription_id,
        inscription_number: event.inscription_number,
        old_satpoint: event.old_satpoint,
        new_satpoint: event.new_satpoint,
        from: event.sender.into(),
        to: event.receiver.into(),
        valid: true,
        hash: hex::encode(predeploy_event.hash),
        msg: "ok".to_string(),
        event: event.op_type,
      }),
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
      Ok(BRC20Event::InscribeWithdraw(inscribe_withdraw_event)) => {
        Self::InscribeTransfer(ApiInscribeTransferEvent {
          inscription_id: event.inscription_id,
          inscription_number: event.inscription_number,
          old_satpoint: event.old_satpoint,
          new_satpoint: event.new_satpoint,
          from: event.sender.into(),
          to: event.receiver.into(),
          valid: true,
          tick: inscribe_withdraw_event.ticker,
          amount: inscribe_withdraw_event.amount.to_string(),
          msg: "ok".to_string(),
          event: event.op_type,
        })
      }
      Ok(BRC20Event::Withdraw(withdraw_event)) => Self::Withdraw(ApiWithdrawEvent {
        inscription_id: event.inscription_id,
        inscription_number: event.inscription_number,
        old_satpoint: event.old_satpoint,
        new_satpoint: event.new_satpoint,
        from: event.sender.into(),
        to: event.receiver.into(),
        tick: withdraw_event.ticker,
        amount: withdraw_event.amount.to_string(),
        msg: "ok".to_string(),
        event: event.op_type,
      }),
      Ok(BRC20Event::InscribeProgDeploy(_)) => {
        Self::InscribeProgDeploy(ApiInscribeProgDeployEvent {
          inscription_id: event.inscription_id,
          inscription_number: event.inscription_number,
          old_satpoint: event.old_satpoint,
          new_satpoint: event.new_satpoint,
          from: event.sender.into(),
          to: event.receiver.into(),
          valid: true,
          msg: "ok".to_string(),
          event: event.op_type,
        })
      }
      Ok(BRC20Event::ProgDeploy(_)) => Self::ProgDeploy(ApiProgDeployEvent {
        inscription_id: event.inscription_id,
        inscription_number: event.inscription_number,
        old_satpoint: event.old_satpoint,
        new_satpoint: event.new_satpoint,
        from: event.sender.into(),
        to: event.receiver.into(),
        valid: true,
        msg: "ok".to_string(),
        event: event.op_type,
      }),
      Ok(BRC20Event::InscribeProgCall(_)) => Self::InscribeProgCall(ApiInscribeProgCallEvent {
        inscription_id: event.inscription_id,
        inscription_number: event.inscription_number,
        old_satpoint: event.old_satpoint,
        new_satpoint: event.new_satpoint,
        from: event.sender.into(),
        to: event.receiver.into(),
        valid: true,
        msg: "ok".to_string(),
        event: event.op_type,
      }),
      Ok(BRC20Event::ProgCall(_)) => Self::ProgCall(ApiProgCallEvent {
        inscription_id: event.inscription_id,
        inscription_number: event.inscription_number,
        old_satpoint: event.old_satpoint,
        new_satpoint: event.new_satpoint,
        from: event.sender.into(),
        to: event.receiver.into(),
        valid: true,
        msg: "ok".to_string(),
        event: event.op_type,
      }),
      Ok(BRC20Event::InscribeProgTransact(_)) => {
        Self::InscribeProgTransact(ApiInscribeProgTransactEvent {
          inscription_id: event.inscription_id,
          inscription_number: event.inscription_number,
          old_satpoint: event.old_satpoint,
          new_satpoint: event.new_satpoint,
          from: event.sender.into(),
          to: event.receiver.into(),
          valid: true,
          msg: "ok".to_string(),
          event: event.op_type,
        })
      }
      Ok(BRC20Event::ProgTransact(_)) => Self::ProgTransact(ApiProgTransactEvent {
        inscription_id: event.inscription_id,
        inscription_number: event.inscription_number,
        old_satpoint: event.old_satpoint,
        new_satpoint: event.new_satpoint,
        from: event.sender.into(),
        to: event.receiver.into(),
        valid: true,
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
pub struct ApiPredeployEvent {
  #[serde(rename = "type")]
  pub event: BRC20OpType,
  pub inscription_id: InscriptionId,
  pub inscription_number: i32,
  pub old_satpoint: SatPoint,
  pub new_satpoint: SatPoint,
  pub from: ApiUtxoAddress,
  pub to: ApiUtxoAddress,
  pub hash: String,
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
pub struct ApiInscribeWithdrawEvent {
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
pub struct ApiWithdrawEvent {
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
  pub msg: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiInscribeProgDeployEvent {
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
pub struct ApiProgDeployEvent {
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
pub struct ApiInscribeProgCallEvent {
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
pub struct ApiProgCallEvent {
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
pub struct ApiInscribeProgTransactEvent {
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
pub struct ApiProgTransactEvent {
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
pub struct ApiTxEvents {
  pub events: Vec<ApiTxEvent>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub original_tx_index: Option<usize>,
  pub txid: Txid,
}

/// Get transaction events by txid.
///
/// Retrieve all BRC20 events associated with a transaction.
pub(crate) async fn brc20_tx_events(
  Extension(index): Extension<Arc<Index>>,
  Path(txid): Path<String>,
) -> ApiResult<ApiTxEvents> {
  tracing::debug!("rpc: get brc20_tx_events: {}", txid);
  task::block_in_place(|| {
    let txid = Txid::from_str(&txid).map_err(ApiError::bad_request)?;
    let rtx = index.begin_read()?;

    let receipts = trace_db_call!("get_brc20_receipts", {
      Index::brc20_get_raw_receipts(&txid, &rtx)
    })?;

    let receipts = match receipts {
      Some(receipts) => receipts,
      None => {
        let tx_info = trace_rpc_call!("get_raw_transaction_info", {
          index
            .client
            .get_raw_transaction_info(&txid, None)
            .map_err(ApiError::internal)
        })?;

        if let Some(blockhash) = tx_info.blockhash {
          let block_info = trace_rpc_call!("get_block_info", {
            index
              .client
              .get_block_info(&blockhash)
              .map_err(ApiError::internal)
          })?;

          let db_blockhash = trace_db_call!("get_block_hash", {
            rtx
              .block_hash(Some(u32::try_from(block_info.height).unwrap()))?
              .ok_or(BRC20ApiError::TransactionReceiptNotFound(txid))
          })?;

          if db_blockhash == blockhash {
            Vec::new()
          } else {
            return Err(BRC20ApiError::TransactionReceiptNotFound(txid).into());
          }
        } else {
          return Err(BRC20ApiError::TransactionReceiptNotFound(txid).into());
        }
      }
    };

    tracing::debug!("rpc: get brc20_tx_events: {} {:?}", txid, receipts);

    Ok(Json(ApiResponse::ok(ApiTxEvents {
      txid,
      original_tx_index: None,
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
  tracing::debug!("rpc: get brc20_block_events: {}", blockhash);
  task::block_in_place(|| {
    let blockhash = BlockHash::from_str(&blockhash).map_err(ApiError::bad_request)?;

    let rtx = index.begin_read()?;

    let block_info = trace_rpc_call!("get_block_info", {
      index
        .client
        .get_block_info(&blockhash)
        .map_err(ApiError::internal)
    })?;

    let db_blockhash = trace_db_call!("get_block_hash", {
      rtx
        .block_hash(Some(u32::try_from(block_info.height).unwrap()))?
        .ok_or(BRC20ApiError::BlockReceiptNotFound(block_info.hash))
    })?;

    // check of conflicting block.
    if block_info.hash != db_blockhash || blockhash != block_info.hash {
      return Err(
        BRC20ApiError::ConflictBlockByHeight(Height(u32::try_from(block_info.height).unwrap()))
          .into(),
      );
    }

    let mut block_receipts = Vec::new();
    trace_db_call!("get_brc20_block_receipts", {
      for (id, txid) in block_info.tx.into_iter().enumerate() {
        let Some(tx_receipts) = Index::brc20_get_raw_receipts(&txid, &rtx)? else {
          continue;
        };
        block_receipts.push((id, txid, tx_receipts));
      }
    });

    tracing::debug!(
      "rpc: get brc20_block_events: {} {:?}",
      blockhash,
      block_receipts
    );

    Ok(Json(ApiResponse::ok(ApiBlockEvents {
      block: block_receipts
        .into_iter()
        .map(|(id, txid, events)| ApiTxEvents {
          txid,
          original_tx_index: Some(id),
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
    let predeploy: ApiPredeployEvent = ApiPredeployEvent {
      event: BRC20OpType::Predeploy,
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
      hash: "abcdef".to_string(),
      valid: true,
      msg: "ok".to_string(),
    };

    assert_eq!(
      serde_json::to_string_pretty(&predeploy).unwrap(),
      r#"{
  "type": "predeploy",
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
  "hash": "abcdef",
  "valid": true,
  "msg": "ok"
}"#
    );

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
