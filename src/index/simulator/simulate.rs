use crate::height::Height;
use crate::index::entry::Entry;
use crate::index::simulator::error::SimulateError;
use crate::index::simulator::processor::{IndexWrapper, StorageProcessor};
use crate::index::simulator::types::ExecuteTxResponse;
use crate::index::updater::pending_updater::PendingUpdater;
use crate::index::{
  BlockData, InscriptionIdValue, BRC20_BALANCES, BRC20_EVENTS, BRC20_INSCRIBE_TRANSFER,
  BRC20_TOKEN, BRC20_TRANSFERABLELOG, COLLECTIONS_INSCRIPTION_ID_TO_KINDS,
  COLLECTIONS_KEY_TO_INSCRIPTION_ID, HOME_INSCRIPTIONS, INSCRIPTION_ID_TO_SEQUENCE_NUMBER,
  INSCRIPTION_NUMBER_TO_SEQUENCE_NUMBER, ORD_TX_TO_OPERATIONS, OUTPOINT_TO_ENTRY,
  OUTPOINT_TO_SAT_RANGES, SATPOINT_TO_SEQUENCE_NUMBER, SAT_TO_SATPOINT, SAT_TO_SEQUENCE_NUMBER,
  SEQUENCE_NUMBER_TO_CHILDREN, SEQUENCE_NUMBER_TO_INSCRIPTION_ENTRY, SEQUENCE_NUMBER_TO_SATPOINT,
  SIMULATE_TRACE_TABLE, STATISTIC_TO_COUNT, TRANSACTION_ID_TO_TRANSACTION,
};
use crate::okx::datastore::brc20::redb::table::get_transaction_receipts;
use crate::okx::datastore::brc20::{Brc20Reader, Receipt};
use crate::okx::datastore::cache::CacheTableIndex;
use crate::okx::datastore::ord::{InscriptionOp, OrdReader};
use crate::okx::lru::SimpleLru;
use crate::okx::protocol::simulate::SimulateContext;
use crate::okx::protocol::trace::TraceNode;
use crate::okx::protocol::{ProtocolConfig, ProtocolManager};
use crate::{Index, Options, Sat, SatPoint};
use anyhow::anyhow;
use bitcoin::{Block, OutPoint, Transaction, TxOut, Txid};
use indexer_sdk::client::drect::DirectClient;
use indexer_sdk::client::event::ClientEvent;
use indexer_sdk::client::Client;
use indexer_sdk::configuration::base::{IndexerConfiguration, NetConfiguration, ZMQConfiguration};
use indexer_sdk::factory::common::async_create_and_start_processor;
use indexer_sdk::storage::db::memory::MemoryDB;
use indexer_sdk::storage::db::thread_safe::ThreadSafeDB;
use indexer_sdk::storage::kv::KVStorageProcessor;
use indexer_sdk::wait_exit_signal;
use log::{error, info};
use redb::{Database, ReadableTable, RedbValue, WriteTransaction};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::path::Path;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use tokio::runtime::Runtime;
use tokio::sync::watch;
use tokio::task::JoinHandle;

pub struct Simulator<'a, 'db, 'tx> {
  pub internal_index: IndexWrapper,
  pub client: Option<DirectClient<KVStorageProcessor<ThreadSafeDB<MemoryDB>>>>,
  _marker_a: PhantomData<&'a ()>,
  _marker_b: PhantomData<&'db ()>,
  _marker_tx: PhantomData<&'tx ()>,
}

#[derive(Clone)]
pub struct SimulatorServer {
  tx_out_cache: Rc<RefCell<SimpleLru<OutPoint, TxOut>>>,
  pub internal_index: IndexWrapper,
  pub simulate_index: Arc<Database>,
  pub client: DirectClient<KVStorageProcessor<ThreadSafeDB<MemoryDB>>>,
}

unsafe impl Send for SimulatorServer {}

unsafe impl Sync for SimulatorServer {}

impl SimulatorServer {
  pub async fn start(&self, rt: Arc<Runtime>, exit: watch::Receiver<()>) -> JoinHandle<()> {
    let internal = self.clone();
    rt.spawn(async move {
      internal.on_start(exit).await;
    })
  }
  async fn on_start(self, mut exit: watch::Receiver<()>) {
    loop {
      tokio::select! {
       event=self.get_client_event()=>{
           match event{
              Ok(event) => {
                  if let Err(e)= self.handle_event(&event).await{
                          log::error!("handle event error: {:?}", e);
                  }
              }
              Err(e) => {
                  log::error!("receive event error: {:?}", e);
                  continue;
              }
          }
       },
       _ = exit.changed() => {
              log::info!("simulator receive exit signal");
              break;
         }
      }
    }
  }

  fn get_current_height(&self) -> crate::Result<u32> {
    let height = self
      .internal_index
      .internal
      .block_height()?
      .unwrap_or(Height(0));
    Ok(height.0)
  }
  async fn handle_event(&self, event: &ClientEvent) -> crate::Result<()> {
    info!("sim receive event:{:?}", event);
    match event {
      ClientEvent::Transaction(tx) => {
        self.execute_tx(tx, true)?;
      }
      ClientEvent::GetHeight => {
        let height = self.get_current_height()?;
        self.client.report_height(height).await?;
      }
      ClientEvent::TxDroped(tx) => {
        let tx = tx.clone().into();
        self.remove_tx_traces(&tx)?;
      }
      ClientEvent::TxConfirmed(tx) => {
        let tx = tx.clone().into();
        self.remove_tx_traces(&tx)?;
      }
    }
    Ok(())
  }
  fn remove_tx_traces(&self, txid: &Txid) -> crate::Result<()> {
    let wtx = self.simulate_index.begin_write()?;
    let commit = self.do_remove_traces(txid, &wtx)?;
    if commit {
      wtx.commit()?;
    }
    Ok(())
  }
  fn do_remove_traces(&self, txid: &Txid, wtx: &WriteTransaction) -> crate::Result<bool> {
    let traces_table = wtx.open_table(SIMULATE_TRACE_TABLE)?;
    let value = txid.store();
    let traces = traces_table.get(&value)?;
    if traces.is_none() {
      return Ok(false);
    }

    let mut brc20_balances = wtx.open_table(BRC20_BALANCES)?;
    let mut brc20_token = wtx.open_table(BRC20_TOKEN)?;
    let mut events = wtx.open_table(BRC20_EVENTS)?;
    let mut brc20_transferlog = wtx.open_table(BRC20_TRANSFERABLELOG)?;
    let mut brc20_inscribe_transfer = wtx.open_table(BRC20_INSCRIBE_TRANSFER)?;
    let mut ord_tx_id_to_operations = wtx.open_table(ORD_TX_TO_OPERATIONS)?;
    let mut collections_key_to_inscription_id =
      wtx.open_table(COLLECTIONS_KEY_TO_INSCRIPTION_ID)?;
    let mut collection_inscription_id_to_kinds =
      wtx.open_table(COLLECTIONS_INSCRIPTION_ID_TO_KINDS)?;
    let binding = traces.unwrap();
    let traces = binding.value();
    let traces: Vec<TraceNode> = rmp_serde::from_slice(traces)?;
    for node in traces {
      let key = node.key;
      match node.trace_type {
        CacheTableIndex::TXID_TO_INSCRIPTION_RECEIPTS => {}
        CacheTableIndex::SEQUENCE_NUMBER_TO_SATPOINT => {}
        CacheTableIndex::SAT_TO_SEQUENCE_NUMBER => {}
        CacheTableIndex::HOME_INSCRIPTIONS => {}
        CacheTableIndex::INSCRIPTION_ID_TO_SEQUENCE_NUMBER => {}
        CacheTableIndex::SEQUENCE_NUMBER_TO_CHILDREN => {}
        CacheTableIndex::SEQUENCE_NUMBER_TO_INSCRIPTION_ENTRY => {}
        CacheTableIndex::INSCRIPTION_NUMBER_TO_SEQUENCE_NUMBER => {}
        CacheTableIndex::OUTPOINT_TO_ENTRY => {}
        CacheTableIndex::BRC20_BALANCES => {
          let key = String::from_utf8(key).unwrap();
          let key = key.as_str();
          brc20_balances.remove(key)?;
        }
        CacheTableIndex::BRC20_TOKEN => {
          let key = String::from_utf8(key).unwrap();
          let key = key.as_str();
          brc20_token.remove(key)?;
        }
        CacheTableIndex::BRC20_EVENTS => {
          let key = key.as_slice();
          let key: &Txid = &rmp_serde::from_slice(key).unwrap();
          events.remove(&key.store())?;
        }
        CacheTableIndex::BRC20_TRANSFERABLELOG => {
          let key = String::from_utf8(key).unwrap();
          brc20_transferlog.remove(key.as_str())?;
        }
        CacheTableIndex::BRC20_INSCRIBE_TRANSFER => {
          let key = InscriptionIdValue::from_bytes(key.as_slice());
          brc20_inscribe_transfer.remove(&key)?;
        }
        CacheTableIndex::ORD_TX_TO_OPERATIONS => {
          let key: [u8; 32] = key.as_slice().try_into().unwrap();
          ord_tx_id_to_operations.remove(&key)?;
        }
        CacheTableIndex::COLLECTIONS_KEY_TO_INSCRIPTION_ID => {
          let key = String::from_utf8(key).unwrap();
          collections_key_to_inscription_id.remove(key.as_str())?;
        }
        CacheTableIndex::COLLECTIONS_INSCRIPTION_ID_TO_KINDS => {
          let key = InscriptionIdValue::from_bytes(&key);
          collection_inscription_id_to_kinds.remove(&key)?;
        }
      }
    }
    Ok(true)
  }
  async fn get_client_event(&self) -> crate::Result<ClientEvent, SimulateError> {
    let ret = self.client.block_get_event().await?;
    Ok(ret)
  }
  pub fn execute_tx(
    &self,
    tx: &Transaction,
    commit: bool,
  ) -> crate::Result<ExecuteTxResponse, SimulateError> {
    let wtx = self.simulate_index.begin_write()?;
    let traces = Rc::new(RefCell::new(vec![]));
    let ret = self.simulate_tx(tx, &wtx, traces)?;
    if commit {
      wtx.commit()?;
    }

    Ok(ret)
  }

  pub fn get_receipt(&self, tx_id: Txid) -> Result<Vec<Receipt>, anyhow::Error> {
    let rx = self.simulate_index.begin_read()?;
    let tab = rx.open_table(BRC20_EVENTS)?;
    let ret = get_transaction_receipts(&tab, &tx_id)?;
    Ok(ret)
  }
  fn simulate_tx(
    &self,
    tx: &Transaction,
    wtx: &WriteTransaction,
    traces: Rc<RefCell<Vec<TraceNode>>>,
  ) -> crate::Result<ExecuteTxResponse, SimulateError> {
    let mut ret = ExecuteTxResponse::default();
    let brc20_receipts = Rc::new(RefCell::new(vec![]));
    let ord_operations = Rc::new(RefCell::new(vec![]));
    let height = self.get_current_height()?;
    let block = self
      .internal_index
      .internal
      .get_block_by_height(height)?
      .unwrap();
    let home_inscriptions = wtx.open_table(HOME_INSCRIPTIONS)?;
    let inscription_id_to_sequence_number = wtx.open_table(INSCRIPTION_ID_TO_SEQUENCE_NUMBER)?;
    let inscription_number_to_sequence_number =
      wtx.open_table(INSCRIPTION_NUMBER_TO_SEQUENCE_NUMBER)?;
    let sat_to_sequence_number = wtx.open_multimap_table(SAT_TO_SEQUENCE_NUMBER)?;
    let satpoint_to_sequence_number = wtx.open_multimap_table(SATPOINT_TO_SEQUENCE_NUMBER)?;
    let sequence_number_to_children = wtx.open_multimap_table(SEQUENCE_NUMBER_TO_CHILDREN)?;
    let sequence_number_to_inscription_entry = Rc::new(RefCell::new(
      wtx.open_table(SEQUENCE_NUMBER_TO_INSCRIPTION_ENTRY)?,
    ));
    let sequence_number_to_satpoint = wtx.open_table(SEQUENCE_NUMBER_TO_SATPOINT)?;
    let transaction_id_to_transaction = wtx.open_table(TRANSACTION_ID_TO_TRANSACTION)?;
    let outpoint_to_entry = Rc::new(RefCell::new(wtx.open_table(OUTPOINT_TO_ENTRY)?));
    let outpoint_to_sat_ranges = wtx.open_table(OUTPOINT_TO_SAT_RANGES)?;
    let sat_to_point = wtx.open_table(SAT_TO_SATPOINT)?;
    let statis_to_count = wtx.open_table(STATISTIC_TO_COUNT)?;
    let traces_table = wtx.open_table(SIMULATE_TRACE_TABLE)?;

    let h = height;
    let ts = block.header.time;
    let ctx = SimulateContext {
      network: self.internal_index.internal.get_chain_network().clone(),
      current_height: h,
      current_block_time: ts as u32,
      internal_index: self.internal_index.clone(),
      ORD_TX_TO_OPERATIONS: Rc::new(RefCell::new(
        wtx.open_table(crate::index::ORD_TX_TO_OPERATIONS)?,
      )),
      COLLECTIONS_KEY_TO_INSCRIPTION_ID: Rc::new(RefCell::new(
        wtx.open_table(COLLECTIONS_KEY_TO_INSCRIPTION_ID)?,
      )),
      COLLECTIONS_INSCRIPTION_ID_TO_KINDS: Rc::new(RefCell::new(
        wtx.open_table(COLLECTIONS_INSCRIPTION_ID_TO_KINDS)?,
      )),
      SEQUENCE_NUMBER_TO_INSCRIPTION_ENTRY: sequence_number_to_inscription_entry.clone(),
      OUTPOINT_TO_ENTRY: outpoint_to_entry.clone(),
      BRC20_BALANCES: Rc::new(RefCell::new(wtx.open_table(BRC20_BALANCES)?)),
      BRC20_TOKEN: Rc::new(RefCell::new(wtx.open_table(BRC20_TOKEN)?)),
      BRC20_EVENTS: Rc::new(RefCell::new(wtx.open_table(BRC20_EVENTS)?)),
      BRC20_TRANSFERABLELOG: Rc::new(RefCell::new(wtx.open_table(BRC20_TRANSFERABLELOG)?)),
      BRC20_INSCRIBE_TRANSFER: Rc::new(RefCell::new(wtx.open_table(BRC20_INSCRIBE_TRANSFER)?)),
      traces: traces.clone(),
      brc20_receipts: brc20_receipts.clone(),
      ord_operations: ord_operations.clone(),
      _marker_a: Default::default(),
    };

    let db_receipts = ctx.get_transaction_receipts(&tx.txid())?;
    let operations = ctx.get_transaction_operations(&tx.txid())?;
    if !db_receipts.is_empty() || !operations.is_empty() {
      ret.ord_operations = operations;
      ret.brc20_receipts = db_receipts;
      info!("tx:{:?} already simulated", tx.txid());
      return Ok(ret);
    }

    let processor = StorageProcessor {
      internal: self.internal_index.clone(),
      // wtx: &mut wtx,
      home_inscriptions: Rc::new(RefCell::new(home_inscriptions)),
      id_to_sequence_number: Rc::new(RefCell::new(inscription_id_to_sequence_number)),
      inscription_number_to_sequence_number: Rc::new(RefCell::new(
        inscription_number_to_sequence_number,
      )),
      outpoint_to_entry,
      transaction_id_to_transaction: Rc::new(RefCell::new(transaction_id_to_transaction)),
      sat_to_sequence_number: Rc::new(RefCell::new(sat_to_sequence_number)),
      satpoint_to_sequence_number: Rc::new(RefCell::new(satpoint_to_sequence_number)),
      sequence_number_to_children: Rc::new(RefCell::new(sequence_number_to_children)),
      sequence_number_to_satpoint: Rc::new(RefCell::new(sequence_number_to_satpoint)),
      sequence_number_to_inscription_entry,
      outpoint_to_sat_ranges: Rc::new(RefCell::new(outpoint_to_sat_ranges)),
      sat_to_satpoint: Rc::new(RefCell::new(sat_to_point)),
      statistic_to_count: Rc::new(RefCell::new(statis_to_count)),
      trace_table: Rc::new(RefCell::new(traces_table)),
      _marker_a: Default::default(),
      client: Some(self.client.clone()),
      traces: traces.clone(),
      context: ctx,
    };

    let mut depth = 0;
    self.loop_simulate_tx(h, &block, &processor, &tx, &mut depth)?;

    let receipts = brc20_receipts.borrow();
    let receipts = receipts.deref().clone();
    let ord_operations = ord_operations.borrow();
    let ord_operations = ord_operations.deref().clone();
    ret.brc20_receipts = receipts;
    ret.ord_operations = ord_operations;

    Ok(ret)
  }
  pub fn loop_simulate_tx(
    &self,
    h: u32,
    block: &Block,
    processor: &StorageProcessor,
    tx: &Transaction,
    depth: &mut i32,
  ) -> crate::Result<(), SimulateError> {
    if *depth >= 20 {
      error!("simulate tx:{:?} depth:{:?} over 20", tx.txid(), depth);
      return Err(SimulateError::StackOverFlow);
    }
    *depth += 1;
    let tx_id = tx.txid();
    let mut need_handle_first = vec![];
    for input in &tx.input {
      if input.previous_output.is_null() {
        continue;
      }
      let prev_tx_id = &input.previous_output.txid;
      let prev_out = processor.get_txout_by_outpoint(&input.previous_output)?;
      if prev_out.is_none() {
        need_handle_first.push(prev_tx_id.clone());
      }
    }
    if need_handle_first.is_empty() {
      info!("parent suits is ready,start to simulate tx:{:?}", &tx_id);
    }
    for (index, parent) in need_handle_first.iter().enumerate() {
      let parent_tx = processor.get_transaction(parent)?;
      if parent_tx.is_none() {
        error!(
          "parent tx not exist,tx_hash:{:?},child_hash:{:?}",
          *parent, &tx_id
        );
        return Err(SimulateError::TxNotFound(parent.clone()));
      }
      let parent_tx = parent_tx.unwrap();
      info!(
        "parent tx :{:?},exist,but not in utxo data,child_hash:{:?},need to simulate parent tx",
        &parent, &tx_id
      );
      self.loop_simulate_tx(h, block, processor, &parent_tx, depth)?;
      if index == need_handle_first.len() - 1 {
        info!(
          "all parent txs {:?} simulate done,start to simulate child_hash:{:?}",
          &need_handle_first, &tx_id
        );
      } else {
        info!(
          "parent tx {:?} simulate done,start to simulate next parent:{:?}",
          &parent,
          need_handle_first[index + 1]
        );
      }
    }
    self.do_simulate_tx(h, block, processor, &tx)?;

    Ok(())
  }

  pub fn do_simulate_tx(
    &self,
    h: u32,
    block: &Block,
    processor: &StorageProcessor,
    tx: &Transaction,
  ) -> crate::Result<(), SimulateError> {
    let mut sim = Simulator {
      internal_index: self.internal_index.clone(),
      client: None,
      _marker_a: Default::default(),
      _marker_b: Default::default(),
      _marker_tx: Default::default(),
    };
    let height = h;
    let block = block.clone();
    let mut cache = self.tx_out_cache.borrow_mut();
    let cache = cache.deref_mut();

    let mut operations: HashMap<Txid, Vec<InscriptionOp>> = HashMap::new();
    let block = BlockData {
      header: block.header,
      txdata: vec![(tx.clone(), tx.txid())],
    };
    sim.index_block(block.clone(), height, cache, &processor, &mut operations)?;
    processor.save_traces(&tx.txid())?;

    Ok(())
  }

  pub fn new(
    path: impl AsRef<Path>,
    internal_index: Arc<Index>,
    client: DirectClient<KVStorageProcessor<ThreadSafeDB<MemoryDB>>>,
  ) -> crate::Result<Self> {
    let simulate_index = Database::create(path)?;
    let simulate_index = Arc::new(simulate_index);
    Ok(Self {
      tx_out_cache: Rc::new(RefCell::new(SimpleLru::new(500))),
      internal_index: IndexWrapper::new(internal_index),
      simulate_index,
      client,
    })
  }
}

impl<'a, 'db, 'tx> Simulator<'a, 'db, 'tx> {
  fn index_block(
    &mut self,
    block: BlockData,
    height: u32,
    tx_out_cache: &'a mut SimpleLru<OutPoint, TxOut>,
    processor: &StorageProcessor<'a, 'db, 'tx>,
    operations: &'a mut HashMap<Txid, Vec<InscriptionOp>>,
  ) -> crate::Result<()> {
    let mut sat_ranges_written = 0;
    let mut outputs_in_block = 0;

    let index_inscriptions = true;

    let fetching_outputs_count = AtomicUsize::new(0);
    let total_outputs_count = AtomicUsize::new(0);
    let cache_outputs_count = AtomicUsize::new(0);
    let miss_outputs_count = AtomicUsize::new(0);
    let meet_outputs_count = AtomicUsize::new(0);
    if index_inscriptions {
      // Send all missing input outpoints to be fetched right away
      let txids = block
        .txdata
        .iter()
        .map(|(_, txid)| txid)
        .collect::<HashSet<_>>();
      let mut tx_outs = Vec::new();
      for (tx, _) in block.txdata.iter() {
        for input in tx.input.iter() {
          total_outputs_count.fetch_add(1, Ordering::Relaxed);
          let prev_output = input.previous_output;
          if prev_output.is_null() {
            continue;
          } else if txids.contains(&prev_output.txid) {
            meet_outputs_count.fetch_add(1, Ordering::Relaxed);
            continue;
          } else if tx_out_cache.contains(&prev_output) {
            cache_outputs_count.fetch_add(1, Ordering::Relaxed);
            continue;
          } else if let Some(txout) = processor.get_txout_by_outpoint(&prev_output)? {
            miss_outputs_count.fetch_add(1, Ordering::Relaxed);
            tx_outs.push((prev_output, Some(txout)));
          } else {
            fetching_outputs_count.fetch_add(1, Ordering::Relaxed);
            tx_outs.push((prev_output, None));
          }
        }
      }
      for (out_point, value) in tx_outs.into_iter() {
        if let Some(tx_out) = value {
          tx_out_cache.insert(out_point, tx_out);
        } else {
          let tx = processor.get_transaction(&out_point.txid)?;
          if tx.is_none() {
            return Err(anyhow!("missing transaction {}", out_point.txid.clone()));
          }
          let tx = tx.unwrap();
          let out = tx.output[out_point.vout as usize].clone();
          let tx_out = TxOut {
            value: out.value,
            script_pubkey: out.script_pubkey.clone(),
          };
          tx_out_cache.insert(out_point, tx_out);
        }
      }
    }

    let mut lost_sats = processor.get_lost_sats()?;
    let cursed_inscription_count = processor.get_cursed_inscription_count()?;
    let blessed_inscription_count = processor.get_blessed_inscription_count()?;
    let unbound_inscriptions = processor.get_unbound_inscriptions()?;
    let next_sequence_number = processor.next_sequence_number()?;

    let mut inscription_updater = PendingUpdater::new(
      operations,
      blessed_inscription_count,
      self.internal_index.internal.options.chain(),
      cursed_inscription_count,
      height,
      self.internal_index.internal.index_transactions,
      next_sequence_number,
      lost_sats,
      block.header.time,
      unbound_inscriptions,
      tx_out_cache,
      processor.clone(),
    )?;

    if processor.internal.internal.index_sats {
      let mut coinbase_inputs = VecDeque::new();

      let h = Height(height);
      if h.subsidy() > 0 {
        let start = h.starting_sat();
        coinbase_inputs.push_front((start.n(), (start + h.subsidy()).n()));
      }

      for (tx_offset, (tx, txid)) in block.txdata.iter().enumerate().skip(1) {
        log::trace!("Indexing transaction {tx_offset}…");

        let mut input_sat_ranges = VecDeque::new();

        self.index_transaction_sats(
          tx,
          *txid,
          &mut input_sat_ranges,
          &mut sat_ranges_written,
          &mut outputs_in_block,
          &mut inscription_updater,
          index_inscriptions,
        )?;

        coinbase_inputs.extend(input_sat_ranges);
      }

      if let Some((tx, txid)) = block.txdata.first() {
        self.index_transaction_sats(
          tx,
          *txid,
          &mut coinbase_inputs,
          &mut sat_ranges_written,
          &mut outputs_in_block,
          &mut inscription_updater,
          index_inscriptions,
        )?;
      }

      if !coinbase_inputs.is_empty() {
        let mut lost_sat_ranges = processor
          .outpoint_to_sat_ranges_remove(&OutPoint::null().store())?
          .map(|ranges| ranges.to_vec())
          .unwrap_or_default();

        for (start, end) in coinbase_inputs {
          if !Sat(start).common() {
            processor.sat_to_satpoint_insert(
              &start,
              &SatPoint {
                outpoint: OutPoint::null(),
                offset: lost_sats,
              }
              .store(),
            )?;
          }

          lost_sat_ranges.extend_from_slice(&(start, end).store());

          lost_sats += end - start;
        }
        processor
          .outpoint_to_sat_ranges_insert(&OutPoint::null().store(), lost_sat_ranges.as_slice())?;
      }
    } else if index_inscriptions {
      for (tx, txid) in block.txdata.iter().skip(1).chain(block.txdata.first()) {
        inscription_updater.index_envelopes(tx, *txid, None)?;
      }
    }
    inscription_updater.flush_cache()?;

    let mut context = processor.create_context()?;
    let config = ProtocolConfig::new_with_options(&self.internal_index.internal.options);
    ProtocolManager::new(config).index_block(&mut context, &block, operations.clone())?;

    Ok(())
  }

  fn index_transaction_sats(
    &mut self,
    tx: &Transaction,
    txid: Txid,
    input_sat_ranges: &mut VecDeque<(u64, u64)>,
    sat_ranges_written: &mut u64,
    outputs_traversed: &mut u64,
    inscription_updater: &mut PendingUpdater,
    index_inscriptions: bool,
  ) -> crate::Result {
    if index_inscriptions {
      inscription_updater.index_envelopes(tx, txid, Some(input_sat_ranges))?;
    }

    for (vout, output) in tx.output.iter().enumerate() {
      let outpoint = OutPoint {
        vout: vout.try_into().unwrap(),
        txid,
      };
      let mut sats = Vec::new();

      let mut remaining = output.value;
      while remaining > 0 {
        let range = input_sat_ranges
          .pop_front()
          .ok_or_else(|| anyhow!("insufficient inputs for transaction outputs"))?;

        if !Sat(range.0).common() {
          inscription_updater.processor.sat_to_satpoint_insert(
            &range.0,
            &SatPoint {
              outpoint,
              offset: 0,
            }
            .store(),
          )?;
        }

        let count = range.1 - range.0;

        let assigned = if count > remaining {
          let middle = range.0 + remaining;
          input_sat_ranges.push_front((middle, range.1));
          (range.0, middle)
        } else {
          range
        };

        sats.extend_from_slice(&assigned.store());

        remaining -= assigned.1 - assigned.0;

        *sat_ranges_written += 1;
      }

      *outputs_traversed += 1;
    }

    Ok(())
  }
}

pub fn start_simulator(ops: Options, internal: Arc<Index>) -> Option<SimulatorServer> {
  if !ops.simulate_enable {
    return None;
  }

  let rt = Arc::new(Runtime::new().unwrap());

  let ret = rt.block_on(async { do_start_simulator(ops, internal.clone()).await });
  ret
}

async fn do_start_simulator(ops: Options, internal: Arc<Index>) -> Option<SimulatorServer> {
  let rt = Arc::new(Runtime::new().unwrap());
  let zmq_url = ops.simulate_zmq_url.clone().unwrap();
  let sim_rpc = ops.simulate_bitcoin_rpc_url.clone().unwrap();
  let sim_user = ops.simulate_bitcoin_rpc_user.clone().unwrap();
  let sim_pass = ops.simulate_bitcoin_rpc_pass.clone().unwrap();
  let notify_rx = ops.rx.clone().unwrap();

  let config = IndexerConfiguration {
    mq: ZMQConfiguration {
      zmq_url,
      zmq_topic: vec!["sequence".to_string(), "rawblock".to_string()],
    },
    net: NetConfiguration {
      url: sim_rpc,
      username: sim_user,
      password: sim_pass,
    },
    ..Default::default()
  };

  let (tx, rx) = watch::channel(());
  let (client, mut handlers) =
    async_create_and_start_processor(rx.clone(), config.clone(), rt.clone()).await;

  let server = SimulatorServer::new(ops.simulate_index.unwrap(), internal.clone(), client).unwrap();
  let start_server = server.clone();
  handlers.push(start_server.start(rt.clone(), rx.clone()).await);
  thread::spawn(move || {
    rt.block_on(async {
      wait_exit_signal().await.unwrap();
      tx.send(()).unwrap();
      for h in handlers {
        let _ = h.await;
      }
      let ret = notify_rx.recv().await;
      if ret.is_ok() {
        let ret = ret.unwrap();
        let _ = ret.send(());
      }
    });
  });

  Some(server)
}
