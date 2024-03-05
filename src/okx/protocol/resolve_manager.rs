use {
  super::*,
  crate::{
    index::entry::{Entry, SatPointValue},
    inscriptions::ParsedEnvelope,
    okx::{
      datastore::{
        brc20::{redb::table::get_transferable_assets_by_outpoint, TransferableLog},
        ord::operation::InscriptionOp,
      },
      protocol::{context::Context, Message},
    },
    Inscription, Result,
  },
  bitcoin::Transaction,
  std::collections::HashMap,
};

pub struct MsgResolveManager {
  config: ProtocolConfig,
}

impl MsgResolveManager {
  pub fn new(config: ProtocolConfig) -> Self {
    Self { config }
  }

  pub fn resolve_message(
    &self,
    context: &Context,
    tx: &Transaction,
    operations: &[InscriptionOp],
  ) -> Result<Vec<Message>> {
    log::debug!(
      "Resolve Manager indexed transaction {}, operations size: {}, data: {:?}",
      tx.txid(),
      operations.len(),
      operations
    );
    let mut messages = Vec::new();
    let mut operation_iter = operations.iter().peekable();
    let new_inscriptions = ParsedEnvelope::from_transaction(tx)
      .into_iter()
      .map(|v| v.payload)
      .collect::<Vec<Inscription>>();

    for input in &tx.input {
      // "operations" is a list of all the operations in the current block, and they are ordered.
      // We just need to find the operation corresponding to the current transaction here.
      while let Some(operation) = operation_iter.peek() {
        if operation.old_satpoint.outpoint != input.previous_output {
          break;
        }
        let operation = operation_iter.next().unwrap();

        // Parse BRC20 message through inscription operation.
        if self
          .config
          .first_brc20_height
          .map(|height| context.chain.blockheight >= height)
          .unwrap_or(false)
        {
          let satpoint_to_transfer_assets: HashMap<SatPointValue, TransferableLog> =
            get_transferable_assets_by_outpoint(
              context.BRC20_SATPOINT_TO_TRANSFERABLE_ASSETS,
              input.previous_output,
            )?
            .into_iter()
            .map(|(satpoint, asset)| (satpoint.store(), asset))
            .collect();

          if let Some(msg) =
            brc20::Message::resolve(operation, &new_inscriptions, satpoint_to_transfer_assets)?
          {
            log::debug!(
              "BRC20 resolved the message from {:?}, msg {:?}",
              operation,
              msg
            );
            messages.push(Message::BRC20(msg));
            continue;
          }
        }
      }
    }
    Ok(messages)
  }
}
