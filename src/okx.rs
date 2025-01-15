use super::*;
use crate::{
  index::{
    bundle_message::{BundleMessage, InscriptionAction, SubType},
    event::{Action, OkxInscriptionEvent},
    BlockData,
  },
  metrics::MetricsExt,
};
use brc20::{BRC20ExecutionMessage, BRC20Receipt};
use context::TableContext;
use entry::CollectionType;
use std::collections::HashMap;

pub(crate) mod bitmap;
pub(crate) mod brc20;
pub(crate) mod btc_domain;
mod composite_key;
pub(crate) mod context;
pub(crate) mod entry;
mod utxo_address;

pub(crate) use self::{
  composite_key::{AddressEndpoint, AddressTickerKey},
  utxo_address::{UtxoAddress, UtxoAddressRef},
};

pub(crate) struct OkxUpdater {
  pub(crate) height: u32,
  pub(crate) timestamp: u32,
}

impl OkxUpdater {
  pub(crate) fn index_block_bundle_messages(
    &mut self,
    context: &mut TableContext,
    index: &Index,
    block_data: &BlockData,
    mut bundle_messages_map: HashMap<Txid, Vec<BundleMessage>>,
  ) -> Result<()> {
    let start_time = Instant::now();
    let mut total_inscription_receipts = 0;
    let mut total_brc20_receipts = 0;
    let mut total_bitmap_messages = 0;
    let mut total_btc_domain_messages = 0;

    log::info!(
      "[OKX] Starting to index block {} at {}, transaction_count: {}, bundle_message_count: {})",
      self.height,
      timestamp(self.timestamp.into()),
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
      if let Some(transaction_bundle_messages) = bundle_messages_map.remove(txid) {
        let (brc20_receipts, bitmap_message_count, btc_domain_message_count) =
          self.process_bundle_messages(context, index, &transaction_bundle_messages)?;
        total_brc20_receipts += brc20_receipts.len();
        total_bitmap_messages += bitmap_message_count;
        total_btc_domain_messages += btc_domain_message_count;

        if !brc20_receipts.is_empty() {
          let brc20_receipts_count = brc20_receipts.len();
          let start_insert_time = Instant::now();

          let sequence_number_list = brc20_receipts
            .iter()
            .map(|receipt| receipt.sequence_number)
            .collect::<HashSet<_>>();

          for sequence_number in sequence_number_list {
            context
              .insert_sequence_number_to_collection_type(sequence_number, CollectionType::BRC20)?;
          }

          context.insert_brc20_tx_receipts(txid, brc20_receipts)?;
          log::debug!(
            "[OKX] Saved {} BRC20 receipts for transaction {} in {} ms",
            brc20_receipts_count,
            txid,
            (Instant::now() - start_insert_time).as_millis()
          );
        }
        if index.has_inscription_receipts() {
          let transaction_bundle_messages_count = transaction_bundle_messages.len();
          total_inscription_receipts += transaction_bundle_messages_count;
          let inscription_receipts = transaction_bundle_messages
            .into_iter()
            .map(Into::into)
            .collect();
          let start_insert_time = Instant::now();
          context.insert_inscription_tx_receipts(txid, inscription_receipts)?;
          log::debug!(
            "[OKX] Saved {} inscription receipts for transaction {} in {} ms",
            transaction_bundle_messages_count,
            txid,
            (Instant::now() - start_insert_time).as_millis()
          );
        }
      }
    }

    if index.has_brc20_index() {
      index
        .metrics
        .increment_brc20_event_count(u32::try_from(total_brc20_receipts).unwrap());
    }
    if index.has_inscription_receipts() {
      index
        .metrics
        .increment_inscription_event_count(u32::try_from(total_inscription_receipts).unwrap());
    }

    log::info!(
            "[OKX] Finished indexing block {} {{ total_inscriptions: {}, total_brc20: {}, total_bitmaps: {}, total_btc_domains: {} }} in {} ms",
            self.height,
            total_inscription_receipts,
            total_brc20_receipts,
            total_bitmap_messages,
            total_btc_domain_messages,
            (Instant::now() - start_time).as_millis(),
        );

    Ok(())
  }

  fn process_bundle_messages(
    &self,
    context: &mut TableContext,
    index: &Index,
    bundle_messages: &[BundleMessage],
  ) -> Result<(Vec<BRC20Receipt>, usize, usize)> {
    let mut brc20_execution_receipts = Vec::new();
    let mut bitmap_message_count = 0;
    let mut btc_domain_message_count = 0;

    for bundle_message in bundle_messages.iter() {
      // process brc20 operation
      if index.has_brc20_index() {
        if let Some(brc20_execution_message) =
          BRC20ExecutionMessage::new_from_bundle_message(bundle_message, context)?
        {
          if let Ok(receipt) = brc20_execution_message.execute(context, self.height, self.timestamp)
          {
            brc20_execution_receipts.push(receipt);
          }
          continue;
        }
      }

      // process bitmap operation
      if index.has_bitmap_index() {
        if let InscriptionAction::Created {
          sub_type: Some(SubType::Bitmap(bitmap_operation)),
          ..
        } = &bundle_message.inscription_action
        {
          bitmap_message_count += 1;
          bitmap_operation.execute(
            context,
            bundle_message.sequence_number,
            bundle_message.inscription_id,
            self.height,
          )?;
        }
      }

      // process btc domain operation
      if index.has_btc_domain_index() {
        if let InscriptionAction::Created {
          sub_type: Some(SubType::BtcDomain(btc_domain)),
          ..
        } = &bundle_message.inscription_action
        {
          btc_domain_message_count += 1;
          btc_domain.execute(
            context,
            bundle_message.sequence_number,
            bundle_message.inscription_id,
          )?;
        }
      }
    }

    Ok((
      brc20_execution_receipts,
      bitmap_message_count,
      btc_domain_message_count,
    ))
  }
}
