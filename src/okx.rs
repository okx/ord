use super::*;
use crate::index::{
  event::{Action, OkxInscriptionEvent},
  BlockData, BundleMessage,
};
use bitmap::BitmapMessage;
use brc20::{BRC20ExecutionMessage, BRC20Message, BRC20Receipt};
use context::TableContext;
use std::collections::HashMap;

pub(crate) mod bitmap;
pub(crate) mod brc20;
mod composite_key;
pub(crate) mod context;
pub(crate) mod entry;
mod utxo_address;

pub(crate) use self::{
  composite_key::{AddressEndpoint, AddressTickerKey},
  utxo_address::{UtxoAddress, UtxoAddressRef},
};

#[derive(Debug, Clone)]
pub enum SubMessage {
  BRC20(BRC20Message),
  BITMAP(BitmapMessage),
}

pub(crate) struct OkxUpdater {
  pub(crate) height: u32,
  pub(crate) timestamp: u32,
  pub(crate) save_inscription_receipts: bool,
}

impl OkxUpdater {
  pub(crate) fn index_block_bundle_messages(
    &mut self,
    context: &mut TableContext,
    block_data: &BlockData,
    mut bundle_messages_map: HashMap<Txid, Vec<BundleMessage>>,
  ) -> Result<()> {
    let start_time = Instant::now();
    let mut total_inscription_receipts = 0;
    let mut total_brc20_receipts = 0;
    let mut total_bitmap_messages = 0;

    log::info!(
            "[OKX] Starting to index block at height {} (timestamp: {}, transaction_count: {}, bundle_message_count: {})",
            self.height,
            self.timestamp,
            block_data.txdata.len(),
            bundle_messages_map.len()
        );

    for (_tx_index, (_transaction, txid)) in block_data
      .txdata
      .iter()
      .enumerate()
      .skip(1)
      .chain(block_data.txdata.iter().enumerate().take(1))
    {
      if let Some(mut transaction_bundle_messages) = bundle_messages_map.remove(txid) {
        transaction_bundle_messages.sort_by_key(|message| message.offset);

        let (brc20_receipts, bitmap_message_count) =
          self.process_bundle_messages(context, &transaction_bundle_messages)?;
        total_brc20_receipts += brc20_receipts.len();
        total_bitmap_messages += bitmap_message_count;

        if !brc20_receipts.is_empty() {
          let brc20_receipts_count = brc20_receipts.len();
          context.insert_brc20_tx_receipts(txid, brc20_receipts)?;
          log::debug!(
            "[OKX] Saved {} BRC20 receipts for transaction {}",
            brc20_receipts_count,
            txid
          );
        }

        if self.save_inscription_receipts {
          let transaction_bundle_messages_count = transaction_bundle_messages.len();
          total_inscription_receipts += transaction_bundle_messages_count;
          let inscription_receipts = transaction_bundle_messages
            .into_iter()
            .map(Into::into)
            .collect();
          context.insert_inscription_tx_receipts(txid, inscription_receipts)?;
          log::debug!(
            "[OKX] Saved {} inscription receipts for transaction {}",
            transaction_bundle_messages_count,
            txid
          );
        }
      }
    }

    log::info!(
            "[OKX] Finished indexing block at height {}: {{ total_inscriptions: {}, total_brc20: {}, total_bitmaps: {}, duration: {} ms }}",
            self.height,
            total_inscription_receipts,
            total_brc20_receipts,
            total_bitmap_messages,
            (Instant::now() - start_time).as_millis(),
        );

    Ok(())
  }

  fn process_bundle_messages(
    &self,
    context: &mut TableContext,
    bundle_messages: &[BundleMessage],
  ) -> Result<(Vec<BRC20Receipt>, usize)> {
    let mut brc20_execution_receipts = Vec::new();
    let mut bitmap_message_count = 0;

    for bundle_message in bundle_messages.iter() {
      if let Some(brc20_message) = Option::<BRC20ExecutionMessage>::from(bundle_message) {
        if let Ok(receipt) = brc20_message.execute(context, self.height, self.timestamp) {
          brc20_execution_receipts.push(receipt);
        }
      } else if let Some(SubMessage::BITMAP(bitmap_message)) = &bundle_message.sub_message {
        bitmap_message_count += 1;
        bitmap_message.execute(context, self.height)?;
      }
    }

    Ok((brc20_execution_receipts, bitmap_message_count))
  }
}
