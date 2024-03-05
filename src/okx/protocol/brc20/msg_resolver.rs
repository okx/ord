use super::*;
use crate::{
  index::entry::{Entry, SatPointValue},
  inscriptions::Inscription,
  okx::{
    datastore::{
      brc20::TransferableLog,
      ord::{Action, InscriptionOp},
    },
    protocol::brc20::{deserialize_brc20_operation, Operation},
  },
  Result,
};
use std::collections::HashMap;

impl Message {
  pub(crate) fn resolve(
    op: &InscriptionOp,
    new_inscriptions: &[Inscription],
    transfer_assets_cache: HashMap<SatPointValue, TransferableLog>,
  ) -> Result<Option<Message>> {
    log::debug!("BRC20 resolving the message from {:?}", op);
    let sat_in_outputs = op
      .new_satpoint
      .map(|satpoint| satpoint.outpoint.txid == op.txid)
      .unwrap_or(false);

    let brc20_operation = match op.action {
      // New inscription is not `cursed` or `unbound`.
      Action::New {
        cursed: false,
        unbound: false,
        vindicated: false,
        inscription: _,
      } if sat_in_outputs => {
        let Ok(brc20_opteration) = deserialize_brc20_operation(
          new_inscriptions
            .get(usize::try_from(op.inscription_id.index).unwrap())
            .unwrap(),
          &op.action,
        ) else {
          return Ok(None);
        };
        brc20_opteration
      }
      // Transfered inscription operation.
      // Attempt to retrieve the `InscribeTransfer` Inscription information from the data store of BRC20S.
      Action::Transfer => {
        let Some(transfer_info) = transfer_assets_cache.get(&op.old_satpoint.store()) else {
          return Ok(None);
        };
        Operation::Transfer(Transfer {
          tick: transfer_info.tick.as_str().to_string(),
          amount: transfer_info.amount.to_string(),
        })
      }
      _ => return Ok(None),
    };
    Ok(Some(Self {
      txid: op.txid,
      sequence_number: op.sequence_number,
      inscription_id: op.inscription_id,
      old_satpoint: op.old_satpoint,
      new_satpoint: op.new_satpoint,
      op: brc20_operation,
      sat_in_outputs,
    }))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::okx::datastore::brc20::{Tick, TransferableLog};
  use bitcoin::{Address, OutPoint};
  use std::str::FromStr;

  fn create_inscription(str: &str) -> Inscription {
    Inscription::new(
      Some("text/plain;charset=utf-8".as_bytes().to_vec()),
      Some(str.as_bytes().to_vec()),
    )
  }

  fn create_inscribe_operation(str: &str) -> (Vec<Inscription>, InscriptionOp) {
    let inscriptions = vec![create_inscription(str)];
    let txid =
      Txid::from_str("b61b0172d95e266c18aea0c624db987e971a5d6d4ebc2aaed85da4642d635735").unwrap();
    let op = InscriptionOp {
      txid,
      action: Action::New {
        cursed: false,
        unbound: false,
        inscription: inscriptions.first().unwrap().clone(),
        vindicated: false,
      },
      sequence_number: 1,
      inscription_number: Some(1),
      inscription_id: InscriptionId { txid, index: 0 },
      old_satpoint: SatPoint {
        outpoint: OutPoint {
          txid: Txid::from_str("2111111111111111111111111111111111111111111111111111111111111111")
            .unwrap(),
          vout: 0,
        },
        offset: 0,
      },
      new_satpoint: Some(SatPoint {
        outpoint: OutPoint { txid, vout: 0 },
        offset: 0,
      }),
    };
    (inscriptions, op)
  }

  fn create_transfer_operation() -> InscriptionOp {
    let txid =
      Txid::from_str("b61b0172d95e266c18aea0c624db987e971a5d6d4ebc2aaed85da4642d635735").unwrap();

    let inscription_id = InscriptionId {
      txid: Txid::from_str("2111111111111111111111111111111111111111111111111111111111111111")
        .unwrap(),
      index: 0,
    };

    InscriptionOp {
      txid,
      action: Action::Transfer,
      sequence_number: 1,
      inscription_number: Some(1),
      inscription_id,
      old_satpoint: SatPoint {
        outpoint: OutPoint {
          txid: inscription_id.txid,
          vout: 0,
        },
        offset: 0,
      },
      new_satpoint: Some(SatPoint {
        outpoint: OutPoint { txid, vout: 0 },
        offset: 0,
      }),
    }
  }

  #[test]
  fn test_invalid_protocol() {
    let transfer_assets_cache = HashMap::new();
    let (inscriptions, op) = create_inscribe_operation(
      r#"{ "p": "brc-20s","op": "deploy", "tick": "ordi", "max": "1000", "lim": "10" }"#,
    );
    assert_matches!(
      Message::resolve(&op, &inscriptions, transfer_assets_cache),
      Ok(None)
    );
  }

  #[test]
  fn test_cursed_or_unbound_inscription() {
    let transfer_assets_cache = HashMap::new();

    let (inscriptions, op) = create_inscribe_operation(
      r#"{ "p": "brc-20","op": "deploy", "tick": "ordi", "max": "1000", "lim": "10" }"#,
    );
    let op = InscriptionOp {
      action: Action::New {
        cursed: true,
        unbound: false,
        inscription: inscriptions.first().unwrap().clone(),
        vindicated: false,
      },
      ..op
    };
    assert_matches!(
      Message::resolve(&op, &inscriptions, transfer_assets_cache.clone()),
      Ok(None)
    );

    let op2 = InscriptionOp {
      action: Action::New {
        cursed: false,
        unbound: true,
        inscription: inscriptions.first().unwrap().clone(),
        vindicated: false,
      },
      ..op
    };
    assert_matches!(
      Message::resolve(&op2, &inscriptions, transfer_assets_cache.clone()),
      Ok(None)
    );
    let op3 = InscriptionOp {
      action: Action::New {
        cursed: true,
        unbound: true,
        inscription: inscriptions.first().unwrap().clone(),
        vindicated: false,
      },
      ..op
    };
    assert_matches!(
      Message::resolve(&op3, &inscriptions, transfer_assets_cache),
      Ok(None)
    );
  }

  #[test]
  fn test_valid_inscribe_operation() {
    let transfer_assets_cache = HashMap::new();
    let (inscriptions, op) = create_inscribe_operation(
      r#"{ "p": "brc-20","op": "deploy", "tick": "ordi", "max": "1000", "lim": "10" }"#,
    );
    let _result_msg = Message {
      txid: op.txid,
      sequence_number: op.sequence_number,
      inscription_id: op.inscription_id,
      old_satpoint: op.old_satpoint,
      new_satpoint: op.new_satpoint,
      op: Operation::Deploy(Deploy {
        tick: "ordi".to_string(),
        max_supply: "1000".to_string(),
        mint_limit: Some("10".to_string()),
        decimals: None,
      }),
      sat_in_outputs: true,
    };
    assert_matches!(
      Message::resolve(&op, &inscriptions, transfer_assets_cache),
      Ok(Some(_result_msg))
    );
  }

  #[test]
  fn test_invalid_transfer() {
    let transfer_assets_cache = HashMap::new();

    // inscribe transfer not found
    let op = create_transfer_operation();
    assert_matches!(
      Message::resolve(&op, &[], transfer_assets_cache.clone()),
      Ok(None)
    );

    // non-first transfer operations.
    let op1 = InscriptionOp {
      old_satpoint: SatPoint {
        outpoint: OutPoint {
          txid: Txid::from_str("3111111111111111111111111111111111111111111111111111111111111111")
            .unwrap(),
          vout: 0,
        },
        offset: 0,
      },
      ..op
    };
    assert_matches!(Message::resolve(&op1, &[], transfer_assets_cache), Ok(None));
  }

  #[test]
  fn test_valid_transfer() {
    let mut transfer_assets_cache = HashMap::new();
    // inscribe transfer not found
    let op = create_transfer_operation();
    transfer_assets_cache.insert(
      op.old_satpoint.store(),
      TransferableLog {
        tick: Tick::from_str("ordi").unwrap(),
        amount: 100,
        inscription_id: op.inscription_id,
        inscription_number: op.inscription_number.unwrap(),
        owner: ScriptKey::Address(
          Address::from_str("bc1qhvd6suvqzjcu9pxjhrwhtrlj85ny3n2mqql5w4").unwrap(),
        ),
      },
    );

    let _msg = Message {
      txid: op.txid,
      sequence_number: op.sequence_number,
      inscription_id: op.inscription_id,
      old_satpoint: op.old_satpoint,
      new_satpoint: op.new_satpoint,
      op: Operation::Transfer(Transfer {
        tick: "ordi".to_string(),
        amount: "100".to_string(),
      }),
      sat_in_outputs: true,
    };

    assert_matches!(
      Message::resolve(&op, &[], transfer_assets_cache),
      Ok(Some(_msg))
    );
  }
}
