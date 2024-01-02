use crate::{
  envelope::ParsedEnvelope,
  okx::{
    datastore::{
      ord::{Action, InscriptionOp, OrdReader},
      ScriptKey,
    },
    protocol::{
      context::Context,
      zeroindexer::{
        datastore::{ZeroIndexerReader, ZeroIndexerReaderWriter},
        error::JSONError,
        error::LedgerError,
        zerodata::{InscriptionContext, ZeroIndexerTx},
      },
    },
  },
  Inscription, Result,
};
use anyhow::anyhow;
use bitcoin::{BlockHash, Network, OutPoint, Transaction};
use serde_json::Value;

pub(crate) mod datastore;
pub(crate) mod error;
pub(crate) mod zerodata;

pub fn resolve_zero_inscription(
  context: &mut Context,
  block_hash: &BlockHash,
  tx: &Transaction,
  operations: &Vec<InscriptionOp>,
) -> Result<Vec<ZeroIndexerTx>> {
  log::debug!(
    "Resolve Inscription indexed transaction {}, operations size: {}, data: {:?}",
    tx.txid(),
    operations.len(),
    operations
  );
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
          let des_res = deserialize_zeroindexer_inscription(&inscription_struct);
          match des_res {
            Ok((content, _, inscription_op)) => {
              if inscription_op == "transfer" {
                context
                  .insert_inscription(&operation.inscription_id, &inscription_struct)
                  .map_err(|e| LedgerError::LedgerError(e))?;
              }
              content
            }
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

            let inscription_struct = match context.get_inscription(&operation.inscription_id) {
              Ok(inner) => match inner {
                None => continue,
                Some(inner) => {
                  context
                    .remove_inscription(&operation.inscription_id)
                    .map_err(|e| LedgerError::LedgerError(e))?;
                  inner
                }
              },
              Err(err) => {
                return Err(anyhow!(
                  "failed to get inscription because btc is down:{}",
                  err
                ));
              }
            };
            let des_res = deserialize_zeroindexer_inscription(&inscription_struct);
            match des_res {
              Ok((content, _, _)) => {
                sender = context
                  .get_script_key_on_satpoint(&operation.old_satpoint, context.chain.network)?
                  .to_string();
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
            match get_script_key_from_transaction(&tx, &sat_point.outpoint, context.chain.network) {
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
        block_height: context.chain.blockheight as u64,
        block_time: context.chain.blocktime,
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

fn deserialize_zeroindexer_inscription(
  inscription: &Inscription,
) -> Result<(String, String, String)> {
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

  let protocol_op = match value.get("op") {
    None => return Err(JSONError::OpNotExist.into()),
    Some(op) => op.to_string().replace("\"", ""),
  };

  return Ok((
    serde_json::to_string(&value).unwrap(),
    protocol_name,
    protocol_op,
  ));
}
