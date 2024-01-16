use crate::okx::datastore::ord::OrdReaderWriter;
use crate::okx::protocol::context::Context;
use bitcoin::Transaction;
use {
  super::*,
  crate::{
    index::BlockData,
    okx::{datastore::ord::operation::InscriptionOp, protocol::ord as ord_proto},
    Instant, Result,
  },
  bitcoin::Txid,
  std::collections::HashMap,
};

pub struct ProtocolManager {
  config: ProtocolConfig,
  call_man: CallManager,
  resolve_man: MsgResolveManager,
}

impl ProtocolManager {
  // Need three datastore, and they're all in the same write transaction.
  pub fn new(config: ProtocolConfig) -> Self {
    Self {
      config,
      call_man: CallManager::new(),
      resolve_man: MsgResolveManager::new(config),
    }
  }

  pub(crate) fn index_tx(
    &self,
    context: &mut Context,
    tx: &Transaction,
    txid: &Txid,
    tx_operations: &Vec<InscriptionOp>,
  ) -> Result {
    // save all transaction operations to ord database.
    if self.config.enable_ord_receipts
      && context.chain.blockheight >= self.config.first_inscription_height
    {
      let start = Instant::now();
      context.save_transaction_operations(txid, tx_operations)?;
      context.inscriptions_size += tx_operations.len();
      context.save_cost += start.elapsed().as_micros();
    }

    let start = Instant::now();
    // Resolve and execute messages.
    let messages = self
      .resolve_man
      .resolve_message(context, tx, tx_operations)?;
    context.resolve_cost += start.elapsed().as_micros();

    let start = Instant::now();
    self.call_man.execute_message(context, txid, &messages)?;
    context.execute_cost += start.elapsed().as_micros();
    context.messages_size += messages.len();

    Ok(())
  }

  pub(crate) fn index_block(
    &self,
    context: &mut Context,
    block: &BlockData,
    operations: HashMap<Txid, Vec<InscriptionOp>>,
  ) -> Result {
    let start = Instant::now();

    // skip the coinbase transaction.
    for (tx, txid) in block.txdata.iter() {
      // skip coinbase transaction.
      if tx
        .input
        .first()
        .is_some_and(|tx_in| tx_in.previous_output.is_null())
      {
        continue;
      }

      // index inscription operations.
      if let Some(tx_operations) = operations.get(txid) {
        self.index_tx(context, tx, txid, tx_operations)?;
      }
    }

    let bitmap_start = Instant::now();
    let mut bitmap_count = 0;
    if self.config.enable_index_bitmap {
      bitmap_count = ord_proto::bitmap::index_bitmap(context, &operations)?;
    }
    let bitmap_cost = bitmap_start.elapsed().as_millis();

    log::info!(
      "Protocol Manager indexed block {} with ord inscriptions {}, messages {}, bitmap {} in {} ms, {}/{}/{}/{}, hit/miss {}/{}",
      context.chain.blockheight,
      context.inscriptions_size,
      context.messages_size,
      bitmap_count,
      start.elapsed().as_millis(),
      context.save_cost/1000,
      context.resolve_cost/1000,
      context.execute_cost/1000,
      bitmap_cost,
      context.hit,
      context.miss,
    );
    Ok(())
  }
}
