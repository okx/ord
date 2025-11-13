use super::*;
use crate::{
  index::{
    bundle_message::{BundleMessage, InscriptionAction, SubType},
    event::{Action, OkxInscriptionEvent},
    BlockData, OpiValidationMode,
  },
  metrics::MetricsExt,
  okx::brc20::event_hash::calculate_brc20_prog_traces_hash,
};
use brc20::{
  event_hash::{get_opi_cumulative_hashes_with_retries, BRC20BlockEventHash},
  BRC20ExecutionMessage, BRC20Receipt,
};
use brc20_prog::Brc20ProgApiClient;
use context::TableContext;
use core::panic;
use entry::CollectionType;
use jsonrpsee::http_client::HttpClient;
use once_cell::sync::Lazy;
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

static RT: Lazy<Runtime> = Lazy::new(|| {
  tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .build()
    .expect("rt")
});

static BRC20_PROG_MINE_BATCH_SIZE: u32 = 5000;

pub(crate) struct OkxUpdater {
  pub(crate) height: u32,
  pub(crate) timestamp: u32,
  pub(crate) block_hash: [u8; 32],
  pub(crate) first_inscription_height: u32,
  pub(crate) first_brc20_prog_height: u32,
}

impl OkxUpdater {
  pub(crate) fn index_block_bundle_messages(
    &mut self,
    context: &mut TableContext<'_, '_>,
    brc20_prog_client: &HttpClient,
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

    if index.has_brc20_index() && self.height >= self.first_brc20_prog_height {
      let current_height = self.height as u32;
      RT.block_on(async {
        let prog_block_height = brc20_prog_client
          .eth_block_number()
          .await
          .expect("Check BRC2.0 server");
        let mut prog_block_height =
          u32::from_str_radix(&prog_block_height.trim_start_matches("0x"), 16)
            .expect("Invalid hex string");
        // Initialise if not initialised
        if prog_block_height == 0 {
          brc20_prog_client
            .brc20_initialise([0u8; 32].into(), 0, 0)
            .await
            .expect("BRC20 initialise failed");
        }
        // Mine empty blocks if not yet at first BRC20 prog height
        while prog_block_height < self.first_brc20_prog_height - 1 {
          let next_prog_height =
            (prog_block_height + BRC20_PROG_MINE_BATCH_SIZE).min(self.first_brc20_prog_height - 1);
          brc20_prog_client
            .brc20_mine((next_prog_height - prog_block_height) as u64, 0)
            .await
            .expect("BRC20 mine failed");
          brc20_prog_client
            .brc20_commit_to_database()
            .await
            .expect("BRC20 commit failed");
          prog_block_height = next_prog_height;
        }
        // Handle reorg if prog block height is ahead of current okx block height
        if prog_block_height >= current_height && prog_block_height >= self.first_brc20_prog_height
        {
          log::warn!(
            "[OKX] BRC20 Prog block height {} is ahead of OKX-ORD block height {}",
            prog_block_height,
            current_height
          );
          brc20_prog_client
            .brc20_reorg(current_height as u64)
            .await
            .expect("BRC20 reorg unrecoverable error");
        }
        if prog_block_height < current_height - 1 {
          log::error!(
            "[OKX] BRC20 Prog block height {} is behind OKX-ORD block height {}",
            prog_block_height,
            current_height
          );
          panic!("BRC20 Prog block height is behind OKX-ORD block height");
        }
        log::info!(
          "[OKX] BRC20 Prog block height {} is synced with OKX-ORD block height {}",
          prog_block_height,
          current_height
        );
      });
    }

    let mut prog_tx_idx: u64 = 0;

    let mut brc20_event_hasher = BRC20BlockEventHash::new();

    for (_tx_index, (_transaction, txid)) in block_data
      .txdata
      .iter()
      .enumerate()
      .skip(1)
      .chain(block_data.txdata.iter().enumerate().take(1))
    {
      if let Some(transaction_bundle_messages) = bundle_messages_map.remove(txid) {
        let (brc20_receipts, bitmap_message_count, btc_domain_message_count) =
          RT.block_on(async {
            self
              .process_bundle_messages(
                context,
                brc20_prog_client,
                index,
                &transaction_bundle_messages,
                prog_tx_idx,
              )
              .await
          })?;

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

          prog_tx_idx += brc20_receipts
            .iter()
            .map(|receipt| receipt.prog_tx_count)
            .sum::<u64>();

          for brc20_receipt in &brc20_receipts {
            brc20_event_hasher.add_receipt(brc20_receipt.clone());
          }

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

    log::info!("[OKX] Current block height: {}", self.height);
    log::info!(
      "[OKX] First brc20 prog height: {}",
      self.first_brc20_prog_height
    );
    log::info!("[OKX] Prog tx idx: {}", prog_tx_idx);

    if index.has_brc20_index() && self.height >= self.first_inscription_height {
      RT.block_on(async {
        if self.height >= self.first_brc20_prog_height {
          let mut block_hash = self.block_hash;
          block_hash.reverse();
          brc20_prog_client
            .brc20_finalise_block(self.timestamp as u64, block_hash.into(), prog_tx_idx)
            .await
            .expect("BRC20 finalise block failed");
        }

        let opi_validation_mode = &index.opi_validation_mode();
        if matches!(*opi_validation_mode, OpiValidationMode::None) {
          return;
        }

        // Verify cumulative event hash and trace hash
        let prev_opi_cumulative_event_hashes =
          match get_opi_cumulative_hashes_with_retries(self.height - 1).await {
            Ok(hash) => hash,
            Err(e) => {
              log::error!(
                "[OKX] Failed to get previous OPI cumulative event hash at block {}: {}",
                self.height - 1,
                e
              );
              if matches!(*opi_validation_mode, OpiValidationMode::Strict) {
                panic!("Failed to get previous OPI cumulative event hash");
              }
              return;
            }
          };
        let event_hash = brc20_event_hasher.get_block_event_hash();
        let cumulative_event_hash = if prev_opi_cumulative_event_hashes.event_hash.len() == 0 {
          event_hash.clone() // Initial hash is just the current hash
        } else {
          sha256::digest(prev_opi_cumulative_event_hashes.event_hash + &event_hash)
        };
        let trace_hash =
          match calculate_brc20_prog_traces_hash(brc20_prog_client, self.height as i32).await {
            Ok(hash) => hash,
            Err(e) => {
              log::error!(
                "[OKX] Failed to calculate BRC20 prog traces hash at block {}: {}",
                self.height,
                e
              );
              if matches!(*opi_validation_mode, OpiValidationMode::Strict) {
                panic!("Failed to calculate BRC20 prog traces hash");
              }
              return;
            }
          };
        let cumulative_trace_hash = if prev_opi_cumulative_event_hashes.trace_hash.len() == 0 {
          sha256::digest(trace_hash)
        } else {
          sha256::digest(prev_opi_cumulative_event_hashes.trace_hash + &trace_hash)
        };
        let current_opi_cumulative_event_hashes =
          match get_opi_cumulative_hashes_with_retries(self.height).await {
            Ok(hash) => hash,
            Err(e) => {
              log::error!(
                "[OKX] Failed to get current OPI cumulative event hash at block {}: {}",
                self.height,
                e
              );
              if matches!(*opi_validation_mode, OpiValidationMode::Strict) {
                panic!("Failed to get current OPI cumulative event hash");
              }
              return;
            }
          };

        if current_opi_cumulative_event_hashes.event_hash.len() > 0
          && cumulative_event_hash != current_opi_cumulative_event_hashes.event_hash
        {
          log::error!(
            "[OKX] BRC20 Block Event Hash mismatch at block {}: computed {}, stored {}",
            self.height,
            cumulative_event_hash,
            current_opi_cumulative_event_hashes.event_hash
          );
          if matches!(*opi_validation_mode, OpiValidationMode::Strict) {
            panic!("BRC20 Block Event Hash mismatch");
          }
          return;
        }
        log::warn!(
          "[OKX] BRC20 Block Event Hash for block {}: {}",
          self.height,
          event_hash
        );

        if current_opi_cumulative_event_hashes.trace_hash.len() > 0
          && cumulative_trace_hash != current_opi_cumulative_event_hashes.trace_hash
        {
          log::error!(
            "[OKX] BRC20 Trace Hash mismatch at block {}: computed {}, stored {}",
            self.height,
            cumulative_trace_hash,
            current_opi_cumulative_event_hashes.trace_hash
          );
          if matches!(*opi_validation_mode, OpiValidationMode::Strict) {
            panic!("BRC20 Trace Hash mismatch");
          }
          return;
        }
      });
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

  async fn process_bundle_messages(
    &self,
    context: &mut TableContext<'_, '_>,
    brc20_prog_client: &HttpClient,
    index: &Index,
    bundle_messages: &[BundleMessage],
    prog_tx_idx: u64,
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
          if let Ok(receipt) = brc20_execution_message
            .execute(
              context,
              brc20_prog_client,
              &index.chain(),
              self.height,
              self.timestamp,
              &self.block_hash,
              prog_tx_idx,
            )
            .await
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
