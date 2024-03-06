use super::*;
use crate::{
  okx::{
    datastore::{
      brc20::Brc20ReaderWriter,
      ord::{collections::CollectionKind, OrdReaderWriter},
    },
    protocol::{brc20 as brc20_proto, context::Context},
  },
  Result,
};
use anyhow::anyhow;
use bitcoin::Txid;
use std::collections::HashSet;

pub struct CallManager {}

impl CallManager {
  pub fn new() -> Self {
    Self {}
  }

  pub fn execute_message(&self, context: &mut Context, txid: &Txid, msgs: &[Message]) -> Result {
    let mut receipts = vec![];
    // execute message
    for msg in msgs {
      match msg {
        Message::BRC20(brc_msg) => {
          let msg =
            brc20_proto::ExecutionMessage::from_message(context, brc_msg, context.chain.network)?;
          let receipt = brc20_proto::execute(context, &msg)?;
          receipts.push(receipt);
        }
      };
    }

    context
      .save_transaction_receipts(txid, &receipts)
      .map_err(|e| anyhow!("failed to add transaction receipt to state! error: {e}"))?;

    let brc20_inscriptions = receipts
      .into_iter()
      .map(|receipt| receipt.inscription_id)
      .collect::<HashSet<_>>();

    for inscription_id in brc20_inscriptions {
      context
        .add_inscription_attributes(&inscription_id, CollectionKind::BRC20)
        .map_err(|e| anyhow!("failed to add inscription attributes to state! error: {e}"))?;
    }
    Ok(())
  }
}
