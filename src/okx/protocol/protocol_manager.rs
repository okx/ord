use crate::okx::datastore::ord::redb::table::save_transaction_operations;
use crate::okx::protocol::context::Context;
use crate::okx::protocol::zeroindexer::datastore::ZeroIndexerReaderWriter;
use crate::okx::protocol::zeroindexer::resolve_zero_inscription;
use crate::okx::protocol::zeroindexer::zerodata::{ZeroData, ZeroIndexerTx};
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

  pub(crate) fn index_block(
    &self,
    context: &mut Context,
    block: &BlockData,
    operations: HashMap<Txid, Vec<InscriptionOp>>,
  ) -> Result {
    let start = Instant::now();
    let mut inscriptions_size = 0;
    let mut messages_size = 0;
    let mut cost1 = 0u128;
    let mut cost2 = 0u128;
    let mut cost3 = 0u128;
    let mut zero_indexer_txs: Vec<ZeroIndexerTx> = Vec::new();
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
        // save all transaction operations to ord database.
        if self.config.enable_ord_receipts
          && context.chain.blockheight >= self.config.first_inscription_height
        {
          let start = Instant::now();
          save_transaction_operations(&mut context.ORD_TX_TO_OPERATIONS, txid, tx_operations)?;
          inscriptions_size += tx_operations.len();
          cost1 += Instant::now().saturating_duration_since(start).as_millis();
        }

        let start = Instant::now();
        // Resolve and execute messages.
        let messages = self
          .resolve_man
          .resolve_message(context, tx, tx_operations)?;
        cost2 += Instant::now().saturating_duration_since(start).as_millis();

        let start = Instant::now();
        for msg in messages.iter() {
          self.call_man.execute_message(context, msg)?;
        }
        cost3 += Instant::now().saturating_duration_since(start).as_millis();
        messages_size += messages.len();

        if context.chain.blockheight >= 779832 {
          match resolve_zero_inscription(context, &block.header.block_hash(), tx, tx_operations) {
            Ok(mut results) => zero_indexer_txs.append(&mut results),
            Err(e) => {
              log::error!("resolve_zero_inscription error:{}", e);
              return Err(e);
            }
          };
        }
      }
    }
    let mut bitmap_count = 0;
    if self.config.enable_index_bitmap {
      bitmap_count = ord_proto::bitmap::index_bitmap(context, &operations)?;
    }

    if context.chain.blockheight >= 779832 {
      match context.insert_zero_indexer_txs(
        context.chain.blockheight as u64,
        &ZeroData {
          block_height: context.chain.blockheight as u64,
          block_hash: block.header.block_hash().to_string(),
          prev_block_hash: block.header.prev_blockhash.to_string(),
          block_time: block.header.time,
          txs: zero_indexer_txs,
        },
      ) {
        Ok(_) => {}
        Err(e) => {
          log::info!("insert_zer_indexer_tx failed: {e}")
        }
      };
    }
    log::info!(
      "Protocol Manager indexed block {} with ord inscriptions {}, messages {}, bitmap {} in {} ms, {}, {}, {}",
      context.chain.blockheight,
      inscriptions_size,
      messages_size,
      bitmap_count,
      (Instant::now() - start).as_millis(),
      cost1,
      cost2,
      cost3,
    );
    Ok(())
  }
}
