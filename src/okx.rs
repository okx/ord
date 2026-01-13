use {
  self::{
    brc20::{
      event_hash::BRC20BlockEventHash,
      evm_prog_client::{Brc20ProgClient, ToB256ED},
      opi_validator::{OpiValidationMode, OpiValidator},
      BRC20ExecutionMessage,
    },
    context::TableContext,
    entry::{CollectionType, InscriptionReceipt},
  },
  super::*,
  crate::{
    index::{
      bundle_message::{BundleMessage, InscriptionAction, SubType},
      reorg::Reorg,
      BlockData,
    },
    metrics::{BlockStatistic, IndexingPhase},
  },
  std::collections::HashMap,
};

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

const BRC20_PROG_MINE_BATCH_SIZE: u64 = 5000;

pub struct Brc20IndexingConfig<'a> {
  pub(crate) brc20_prog_client: &'a Brc20ProgClient,
  pub(crate) opi_validation_mode: OpiValidationMode,
}

pub(crate) struct OkxUpdater<'a> {
  pub(crate) height: u64,
  pub(crate) chain: Chain,
  pub(crate) timestamp: u32,
  pub(crate) block_hash: BlockHash,
  pub(crate) save_inscription_receipts: bool,
  pub(crate) index_bitmap: bool,
  pub(crate) index_btc_domain: bool,
  pub(crate) index_brc20: Option<Brc20IndexingConfig<'a>>,
}

impl<'a> OkxUpdater<'a> {
  pub(crate) fn index_block_bundle_messages(
    &mut self,
    context: &mut TableContext<'_, '_>,
    block_data: &BlockData,
    mut bundle_messages: HashMap<Txid, Vec<BundleMessage>>,
  ) -> Result<()> {
    let block_start = Instant::now();

    // Accumulated result for all transactions in block
    let mut block_result = ProcessingResult::default();

    log::info!(
      "[OKX] Block {} indexing started | Transactions: {}, Bundle messages: {}",
      self.height,
      block_data.txdata.len(),
      bundle_messages.len()
    );

    if let Some(brc20_indexing_config) = &self.index_brc20 {
      let first_brc20_prog_height = self.chain.first_brc20_prog_height() as u64;
      if self.height >= first_brc20_prog_height {
        let brc20_prog_client = brc20_indexing_config.brc20_prog_client;
        let mut prog_block_height = brc20_prog_client.eth_block_number()?;
        if prog_block_height == 0 {
          brc20_prog_client.brc20_initialise([0u8; 32].into(), 0, 0)?;
          // Refresh prog_block_height after initialization
          prog_block_height = brc20_prog_client.eth_block_number()?;
        }
        // Mine empty blocks if not yet at first BRC20 prog height
        // Avoid underflow when first_brc20_prog_height is 0
        if first_brc20_prog_height > 0 {
          while prog_block_height < first_brc20_prog_height - 1 {
            let next_prog_height =
              (prog_block_height + BRC20_PROG_MINE_BATCH_SIZE).min(first_brc20_prog_height - 1);
            brc20_prog_client.brc20_mine(next_prog_height - prog_block_height, 0)?;
            brc20_prog_client.brc20_commit_to_database()?;
            prog_block_height = next_prog_height;
          }
        }

        // Check and fix height consistency between ord and prog databases
        Reorg::detect_reorg_with_brc20(self.height as u32, brc20_prog_client)?;
      }
    }

    let mut prog_tx_idx: u64 = 0;
    let mut brc20_block_event_hasher = BRC20BlockEventHash::new();

    for (_tx_index, (_transaction, txid)) in block_data
      .txdata
      .iter()
      .enumerate()
      .skip(1)
      .chain(block_data.txdata.iter().enumerate().take(1))
    {
      let Some(transaction_bundle_messages) = bundle_messages.remove(txid) else {
        continue;
      };

      let tx_result = self.process_bundle_messages(
        context,
        *txid,
        &transaction_bundle_messages,
        &mut prog_tx_idx,
        &mut brc20_block_event_hasher,
      )?;

      // Accumulate results from this transaction
      block_result.add(&tx_result);
    }

    // Finalize block and validate with OPI
    if let Some(brc20_indexing_config) = &self.index_brc20 {
      let first_brc20_prog_height = self.chain.first_brc20_prog_height() as u64;
      if self.height >= self.chain.first_inscription_height() as u64 {
        let brc20_prog_client = brc20_indexing_config.brc20_prog_client;
        if self.height >= first_brc20_prog_height {
          brc20_prog_client.brc20_finalise_block(
            self.timestamp as u64,
            self.block_hash.to_b256_ed(),
            prog_tx_idx,
          )?;
        }

        // Validate BRC20 events with OPI
        let opi_validation_start = Instant::now();

        let mut validator = OpiValidator::new(
          self.chain.clone(),
          context,
          brc20_indexing_config.opi_validation_mode,
          brc20_prog_client,
        );

        if let Some(opi_block_validation) = validator.validate(
          self.height as u32,
          &self.block_hash,
          self.timestamp,
          brc20_block_event_hasher.get_block_event_hash(),
        )? {
          context.insert_opi_block_validation(self.height as u32, opi_block_validation)?;
        }

        metrics::record_phase(IndexingPhase::OpiValidation, opi_validation_start.elapsed());
      }
    }

    block_result.total_duration = block_start.elapsed();

    // Record OKX indexing sub-phase durations
    metrics::record_phase(
      IndexingPhase::InscriptionReceiptsIndexing,
      block_result.phase_durations.inscription_receipts,
    );
    metrics::record_phase(
      IndexingPhase::Brc20Indexing,
      block_result.phase_durations.brc20,
    );
    metrics::record_phase(
      IndexingPhase::BitmapIndexing,
      block_result.phase_durations.bitmap,
    );
    metrics::record_phase(
      IndexingPhase::BtcDomainIndexing,
      block_result.phase_durations.btc_domain,
    );
    metrics::record_phase(IndexingPhase::OkxTotalIndexing, block_result.total_duration);

    // Record block statistics
    metrics::record_stats(
      BlockStatistic::Inscriptions,
      block_result.inscription_count as u64,
    );
    metrics::record_stats(BlockStatistic::Brc20Events, block_result.brc20_count as u64);
    metrics::record_stats(BlockStatistic::Bitmaps, block_result.bitmap_count as u64);
    metrics::record_stats(
      BlockStatistic::BtcDomains,
      block_result.btc_domain_count as u64,
    );

    log::info!(
      "[OKX] Block {} indexed in {} | Stats: inscriptions={}, brc20={}, bitmaps={}, domains={} | Durations: inscription_receipts={}, brc20={}, bitmap={}, btc_domain={}",
      self.height,
      humantime::format_duration(block_result.total_duration),
      block_result.inscription_count,
      block_result.brc20_count,
      block_result.bitmap_count,
      block_result.btc_domain_count,
      humantime::format_duration(block_result.phase_durations.inscription_receipts),
      humantime::format_duration(block_result.phase_durations.brc20),
      humantime::format_duration(block_result.phase_durations.bitmap),
      humantime::format_duration(block_result.phase_durations.btc_domain),
    );

    Ok(())
  }

  fn process_bundle_messages(
    &self,
    context: &mut TableContext<'_, '_>,
    txid: Txid,
    bundle_messages: &[BundleMessage],
    prog_tx_idx: &mut u64,
    brc20_block_event_hasher: &mut BRC20BlockEventHash,
  ) -> Result<ProcessingResult> {
    let mut brc20_receipts = Vec::new();
    // Initialize result accumulator
    let mut result = ProcessingResult::default();
    let total_start = Instant::now();

    // Process each bundle message
    for bundle_message in bundle_messages {
      // Process BRC20 operation
      if let Some(brc20_indexing_config) = &self.index_brc20 {
        if let Some(brc20_execution_message) =
          BRC20ExecutionMessage::new_from_bundle_message(bundle_message, context)?
        {
          let brc20_start = Instant::now();
          if let Ok(receipt) = brc20_execution_message.execute(
            context,
            brc20_indexing_config.brc20_prog_client,
            &self.chain,
            self.height as u32,
            self.timestamp,
            &self.block_hash,
            prog_tx_idx,
          ) {
            brc20_receipts.push(receipt);
          }
          result.phase_durations.brc20 += brc20_start.elapsed();
          continue;
        }
      }

      // Process bitmap operation
      if self.index_bitmap {
        if let InscriptionAction::Created {
          sub_type: Some(SubType::Bitmap(bitmap_operation)),
          ..
        } = &bundle_message.inscription_action
        {
          let bitmap_start = Instant::now();
          result.bitmap_count += 1;
          bitmap_operation.execute(
            context,
            bundle_message.sequence_number,
            bundle_message.inscription_id,
            self.height as u32,
          )?;
          result.phase_durations.bitmap += bitmap_start.elapsed();
        }
      }

      // Process BTC domain operation
      if self.index_btc_domain {
        if let InscriptionAction::Created {
          sub_type: Some(SubType::BtcDomain(btc_domain)),
          ..
        } = &bundle_message.inscription_action
        {
          let domain_start = Instant::now();
          result.btc_domain_count += 1;
          btc_domain.execute(
            context,
            bundle_message.sequence_number,
            bundle_message.inscription_id,
          )?;
          result.phase_durations.btc_domain += domain_start.elapsed();
        }
      }
    }

    let brc20_receipts_count = brc20_receipts.len();

    // Save BRC20 receipts to database
    if brc20_receipts_count > 0 {
      let save_start = Instant::now();

      for receipt in &brc20_receipts {
        brc20_block_event_hasher.add_receipt(receipt.clone());
        context.insert_sequence_number_to_collection_type(
          receipt.sequence_number,
          CollectionType::BRC20,
        )?;
      }

      context.insert_brc20_tx_receipts(&txid, brc20_receipts)?;
      result.phase_durations.brc20 += save_start.elapsed();

      log::debug!(
        "[OKX] Saved {} BRC20 receipts for transaction {} in {}",
        brc20_receipts_count,
        txid,
        humantime::format_duration(save_start.elapsed())
      );
    }

    // Save inscription receipts to database
    result.inscription_count = bundle_messages.len();
    if self.save_inscription_receipts && !bundle_messages.is_empty() {
      let save_start = Instant::now();

      let inscription_receipts = bundle_messages
        .iter()
        .map(InscriptionReceipt::from)
        .collect();

      context.insert_inscription_tx_receipts(&txid, inscription_receipts)?;
      result.phase_durations.inscription_receipts += save_start.elapsed();

      log::debug!(
        "[OKX] Saved {} inscription receipts for transaction {} in {}",
        result.inscription_count,
        txid,
        humantime::format_duration(save_start.elapsed())
      );
    }

    // Set final BRC20 count
    result.brc20_count = brc20_receipts_count;
    result.total_duration = total_start.elapsed();

    Ok(result)
  }
}

/// Result of processing and saving bundle messages
#[derive(Default)]
struct ProcessingResult {
  /// Number of inscription receipts processed and saved
  inscription_count: usize,
  /// Number of BRC20 events processed and saved
  brc20_count: usize,
  /// Number of bitmap inscriptions processed
  bitmap_count: usize,
  /// Number of BTC domain inscriptions processed
  btc_domain_count: usize,
  /// Time spent on each indexing sub-phase
  phase_durations: IndexingDurations,
  /// Total time spent processing and saving bundle messages
  total_duration: Duration,
}

impl ProcessingResult {
  /// Add another processing result to this one (accumulate)
  fn add(&mut self, other: &ProcessingResult) {
    // Note: brc20_receipts are not accumulated (already saved per transaction)
    self.inscription_count += other.inscription_count;
    self.brc20_count += other.brc20_count;
    self.bitmap_count += other.bitmap_count;
    self.btc_domain_count += other.btc_domain_count;
    self.phase_durations.add(&other.phase_durations);
    self.total_duration += other.total_duration;
  }
}

/// Time spent on each OKX indexing sub-phase
#[derive(Default)]
struct IndexingDurations {
  /// Time spent saving inscription receipts to database
  inscription_receipts: Duration,
  /// Time spent indexing BRC20 tokens (including database save)
  brc20: Duration,
  /// Time spent indexing bitmap collection
  bitmap: Duration,
  /// Time spent indexing BTC domain collection
  btc_domain: Duration,
}

impl IndexingDurations {
  /// Add another duration set to this one (accumulate)
  fn add(&mut self, other: &IndexingDurations) {
    self.brc20 += other.brc20;
    self.inscription_receipts += other.inscription_receipts;
    self.bitmap += other.bitmap;
    self.btc_domain += other.btc_domain;
  }
}
