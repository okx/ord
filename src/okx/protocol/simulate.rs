use crate::index::entry::Entry;
use crate::index::simulator::processor::IndexWrapper;
use crate::index::{
  InscriptionEntryValue, InscriptionIdValue, OutPointValue, TxidValue, BRC20_BALANCES,
  BRC20_EVENTS, BRC20_INSCRIBE_TRANSFER, BRC20_TOKEN, BRC20_TRANSFERABLELOG,
  COLLECTIONS_INSCRIPTION_ID_TO_KINDS, COLLECTIONS_KEY_TO_INSCRIPTION_ID, OUTPOINT_TO_ENTRY,
  SEQUENCE_NUMBER_TO_INSCRIPTION_ENTRY,
};
use crate::okx::datastore::brc20::redb::table::{
  get_balance, get_balances, get_inscribe_transfer_inscription, get_token_info, get_tokens_info,
  get_transaction_receipts, get_transferable, get_transferable_by_id, get_transferable_by_tick,
  insert_inscribe_transfer_inscription, insert_token_info, insert_transferable,
  remove_inscribe_transfer_inscription, remove_transferable, save_transaction_receipts,
  update_token_balance,
};
use crate::okx::datastore::brc20::redb::{script_tick_id_key, script_tick_key};
use crate::okx::datastore::brc20::{
  Balance, Brc20Reader, Brc20ReaderWriter, Receipt, Tick, TokenInfo, TransferInfo, TransferableLog,
};
use crate::okx::datastore::cache::CacheTableIndex;
use crate::okx::datastore::ord::collections::CollectionKind;
use crate::okx::datastore::ord::redb::table::{
  get_collection_inscription_id, get_collections_of_inscription,
  get_inscription_number_by_sequence_number, get_transaction_operations, get_txout_by_outpoint,
  save_transaction_operations, set_inscription_attributes, set_inscription_by_collection_key,
};
use crate::okx::datastore::ord::{InscriptionOp, OrdReader, OrdReaderWriter};
use crate::okx::datastore::ScriptKey;
use crate::okx::protocol::trace::TraceNode;
use crate::okx::protocol::ContextTrait;
use crate::{InscriptionId, SatPoint};
use anyhow::anyhow;
use bitcoin::{Network, Txid};
use redb::{ReadOnlyTable, RedbKey, RedbValue, Table, TableDefinition};
use std::cell::RefCell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;

#[allow(non_snake_case)]
#[derive(Clone)]
pub struct SimulateContext<'a, 'db, 'txn> {
  pub network: Network,
  pub current_height: u32,
  pub current_block_time: u32,
  pub internal_index: IndexWrapper,
  pub(crate) ORD_TX_TO_OPERATIONS: Rc<RefCell<Table<'db, 'txn, &'static TxidValue, &'static [u8]>>>,
  pub(crate) COLLECTIONS_KEY_TO_INSCRIPTION_ID:
    Rc<RefCell<Table<'db, 'txn, &'static str, InscriptionIdValue>>>,
  pub(crate) COLLECTIONS_INSCRIPTION_ID_TO_KINDS:
    Rc<RefCell<Table<'db, 'txn, InscriptionIdValue, &'static [u8]>>>,
  pub(crate) SEQUENCE_NUMBER_TO_INSCRIPTION_ENTRY:
    Rc<RefCell<Table<'db, 'txn, u32, InscriptionEntryValue>>>,
  pub(crate) OUTPOINT_TO_ENTRY:
    Rc<RefCell<Table<'db, 'txn, &'static OutPointValue, &'static [u8]>>>,

  // BRC20 tables
  pub(crate) BRC20_BALANCES: Rc<RefCell<Table<'db, 'txn, &'static str, &'static [u8]>>>,
  pub(crate) BRC20_TOKEN: Rc<RefCell<Table<'db, 'txn, &'static str, &'static [u8]>>>,
  pub(crate) BRC20_EVENTS: Rc<RefCell<Table<'db, 'txn, &'static TxidValue, &'static [u8]>>>,
  pub(crate) BRC20_TRANSFERABLELOG: Rc<RefCell<Table<'db, 'txn, &'static str, &'static [u8]>>>,
  pub(crate) BRC20_INSCRIBE_TRANSFER:
    Rc<RefCell<Table<'db, 'txn, InscriptionIdValue, &'static [u8]>>>,
  pub traces: Rc<RefCell<Vec<TraceNode>>>,
  pub brc20_receipts: Rc<RefCell<Vec<Receipt>>>,
  pub ord_operations: Rc<RefCell<Vec<InscriptionOp>>>,
  pub _marker_a: PhantomData<&'a ()>,
}

impl<'a, 'db, 'txn> Brc20Reader for SimulateContext<'a, 'db, 'txn> {
  type Error = anyhow::Error;

  fn get_balances(&self, script_key: &ScriptKey) -> crate::Result<Vec<Balance>, Self::Error> {
    let balances = self.BRC20_BALANCES.borrow();
    let table = balances.deref();
    let simulate = get_balances(table, script_key)?;
    let internal = self.use_internal_table(BRC20_BALANCES, |v| get_balances(&v, script_key))?;
    let mut simulate_balances: HashMap<Tick, Balance> = simulate
      .into_iter()
      .map(|v| (v.tick.clone(), v.clone()))
      .collect();
    for node in internal {
      let v = simulate_balances
        .entry(node.tick.clone())
        .or_insert(node.clone());
      v.transferable_balance += node.transferable_balance;
      v.overall_balance += node.overall_balance
    }
    let ret = simulate_balances.into_values().collect();
    Ok(ret)
  }

  fn get_balance(
    &self,
    script_key: &ScriptKey,
    tick: &Tick,
  ) -> crate::Result<Option<Balance>, Self::Error> {
    let table = self.BRC20_BALANCES.borrow();
    let table = table.deref();
    let ret = get_balance(table, script_key, tick)?;
    if let Some(ret) = ret {
      return Ok(Some(ret));
    }
    self.use_internal_table(BRC20_BALANCES, |table| {
      get_balance(&table, script_key, tick)
    })
  }

  fn get_token_info(&self, tick: &Tick) -> crate::Result<Option<TokenInfo>, Self::Error> {
    let table = self.BRC20_TOKEN.borrow();
    let table = table.deref();
    let ret = get_token_info(table, tick)?;
    if let Some(ret) = ret {
      return Ok(Some(ret));
    }
    self.use_internal_table(BRC20_TOKEN, |table| get_token_info(&table, tick))
  }

  fn get_tokens_info(&self) -> crate::Result<Vec<TokenInfo>, Self::Error> {
    let binding = self.BRC20_TOKEN.borrow();
    let table = binding.deref();
    let ret = get_tokens_info(table)?;
    let mut token_map = ret
      .into_iter()
      .map(|v| (v.tick.clone(), v))
      .collect::<HashMap<Tick, TokenInfo>>();
    let internal = self.use_internal_table(BRC20_TOKEN, |table| get_tokens_info(&table))?;
    for node in internal {
      if !token_map.contains_key(&node.tick) {
        token_map.insert(node.tick.clone(), node.clone());
      }
    }
    let ret = token_map.into_values().collect();
    Ok(ret)
  }

  fn get_transaction_receipts(&self, txid: &Txid) -> crate::Result<Vec<Receipt>, Self::Error> {
    let binding = self.BRC20_EVENTS.borrow();
    let table = binding.deref();
    let ret = get_transaction_receipts(table, txid)?;
    let mut simulate_receipts = ret
      .into_iter()
      .map(|v| (v.inscription_id, v))
      .collect::<HashMap<InscriptionId, Receipt>>();
    let internal =
      self.use_internal_table(BRC20_EVENTS, |table| get_transaction_receipts(&table, txid))?;
    for node in internal {
      simulate_receipts
        .entry(node.inscription_id)
        .or_insert_with(|| node.clone());
    }
    let ret = simulate_receipts.into_values().collect();
    Ok(ret)
  }

  fn get_transferable(
    &self,
    script: &ScriptKey,
  ) -> crate::Result<Vec<TransferableLog>, Self::Error> {
    let binding = self.BRC20_TRANSFERABLELOG.borrow();
    let table = binding.deref();
    let ret = get_transferable(table, script)?;
    let mut simulate_transferable = ret
      .into_iter()
      .map(|v| (v.inscription_id, v))
      .collect::<HashMap<InscriptionId, TransferableLog>>();
    let internal = self.use_internal_table(BRC20_TRANSFERABLELOG, |table| {
      get_transferable(&table, script)
    })?;
    for node in internal {
      simulate_transferable
        .entry(node.inscription_id)
        .or_insert_with(|| node.clone());
    }
    let ret = simulate_transferable.into_values().collect();
    Ok(ret)
  }

  fn get_transferable_by_tick(
    &self,
    script: &ScriptKey,
    tick: &Tick,
  ) -> crate::Result<Vec<TransferableLog>, Self::Error> {
    let binding = self.BRC20_TRANSFERABLELOG.borrow();
    let table = binding.deref();
    let ret = get_transferable_by_tick(table, script, tick)?;
    let mut simulate_transferable = ret
      .into_iter()
      .map(|v| (v.inscription_id, v))
      .collect::<HashMap<InscriptionId, TransferableLog>>();
    let internal = self.use_internal_table(BRC20_TRANSFERABLELOG, |table| {
      get_transferable_by_tick(&table, script, tick)
    })?;
    for node in internal {
      simulate_transferable
        .entry(node.inscription_id)
        .or_insert_with(|| node.clone());
    }
    let ret = simulate_transferable.into_values().collect();
    Ok(ret)
  }

  fn get_transferable_by_id(
    &self,
    script: &ScriptKey,
    inscription_id: &InscriptionId,
  ) -> crate::Result<Option<TransferableLog>, Self::Error> {
    let binding = self.BRC20_TRANSFERABLELOG.borrow();
    let table = binding.deref();
    let ret = get_transferable_by_id(table, script, inscription_id)?;
    if let Some(ret) = ret {
      return Ok(Some(ret));
    }
    self.use_internal_table(BRC20_TRANSFERABLELOG, |table| {
      get_transferable_by_id(&table, script, inscription_id)
    })
  }

  fn get_inscribe_transfer_inscription(
    &self,
    inscription_id: &InscriptionId,
  ) -> crate::Result<Option<TransferInfo>, Self::Error> {
    let binding = self.BRC20_INSCRIBE_TRANSFER.borrow();
    let table = binding.deref();
    let ret = get_inscribe_transfer_inscription(table, inscription_id)?;
    if let Some(ret) = ret {
      return Ok(Some(ret));
    }
    self.use_internal_table(BRC20_INSCRIBE_TRANSFER, |table| {
      get_inscribe_transfer_inscription(&table, inscription_id)
    })
  }
}

impl<'a, 'db, 'txn> Brc20ReaderWriter for SimulateContext<'a, 'db, 'txn> {
  fn update_token_balance(
    &mut self,
    script_key: &ScriptKey,
    new_balance: Balance,
  ) -> crate::Result<(), Self::Error> {
    let mut traces = self.traces.borrow_mut();
    let binding = script_tick_key(script_key, &new_balance.tick);
    let key = binding.as_str();
    let key = key.as_bytes().to_vec();
    traces.push(TraceNode {
      trace_type: CacheTableIndex::BRC20_BALANCES,
      key,
    });
    let mut table = self.BRC20_BALANCES.borrow_mut();
    update_token_balance(&mut table, script_key, new_balance)
  }

  fn insert_token_info(
    &mut self,
    tick: &Tick,
    new_info: &TokenInfo,
  ) -> crate::Result<(), Self::Error> {
    let mut traces = self.traces.borrow_mut();
    let binding = tick.to_lowercase().hex();
    let key = binding.as_str();
    let key = key.as_bytes().to_vec();
    traces.push(TraceNode {
      trace_type: CacheTableIndex::BRC20_TOKEN,
      key,
    });

    let mut binding = self.BRC20_TOKEN.borrow_mut();
    let table = binding.deref_mut();
    insert_token_info(table, tick, new_info)
  }

  fn update_mint_token_info(
    &mut self,
    tick: &Tick,
    minted_amt: u128,
    minted_block_number: u32,
  ) -> crate::Result<(), Self::Error> {
    let info = self.get_token_info(tick)?;
    if info.is_none() {
      return Err(anyhow!(format!(
        "token {:?} not exist",
        tick.to_lowercase().to_string()
      )));
    }
    let mut info = info.unwrap();
    let mut binding = self.BRC20_TOKEN.borrow_mut();
    let table = binding.deref_mut();
    info.minted = minted_amt;
    info.latest_mint_number = minted_block_number;
    insert_token_info(table, tick, &info)
  }

  fn save_transaction_receipts(
    &mut self,
    txid: &Txid,
    receipt: &[Receipt],
  ) -> crate::Result<(), Self::Error> {
    let mut traces = self.traces.borrow_mut();
    let key = rmp_serde::to_vec(txid).unwrap();
    traces.push(TraceNode {
      trace_type: CacheTableIndex::BRC20_EVENTS,
      key,
    });
    let mut receipts = self.brc20_receipts.borrow_mut();
    receipts.extend_from_slice(receipt);
    let mut table = self.BRC20_EVENTS.borrow_mut();
    save_transaction_receipts(&mut table, txid, receipt)
  }

  fn insert_transferable(
    &mut self,
    script: &ScriptKey,
    tick: &Tick,
    inscription: &TransferableLog,
  ) -> crate::Result<(), Self::Error> {
    let mut traces = self.traces.borrow_mut();
    let binding = script_tick_id_key(script, tick, &inscription.inscription_id);
    let key = binding.as_str();
    let key = key.as_bytes().to_vec();
    traces.push(TraceNode {
      trace_type: CacheTableIndex::BRC20_TRANSFERABLELOG,
      key,
    });
    let mut table = self.BRC20_TRANSFERABLELOG.borrow_mut();
    insert_transferable(&mut table, script, tick, inscription)
  }

  fn remove_transferable(
    &mut self,
    script: &ScriptKey,
    tick: &Tick,
    inscription_id: &InscriptionId,
  ) -> crate::Result<(), Self::Error> {
    let mut table = self.BRC20_TRANSFERABLELOG.borrow_mut();
    remove_transferable(&mut table, script, tick, inscription_id)
  }

  fn insert_inscribe_transfer_inscription(
    &mut self,
    inscription_id: &InscriptionId,
    transfer_info: TransferInfo,
  ) -> crate::Result<(), Self::Error> {
    let mut traces = self.traces.borrow_mut();
    let key = &inscription_id.store();
    let key = InscriptionIdValue::as_bytes(key);
    traces.push(TraceNode {
      trace_type: CacheTableIndex::BRC20_INSCRIBE_TRANSFER,
      key,
    });
    let mut table = self.BRC20_INSCRIBE_TRANSFER.borrow_mut();
    insert_inscribe_transfer_inscription(&mut table, inscription_id, transfer_info)
  }

  fn remove_inscribe_transfer_inscription(
    &mut self,
    inscription_id: &InscriptionId,
  ) -> crate::Result<(), Self::Error> {
    let mut table = self.BRC20_INSCRIBE_TRANSFER.borrow_mut();
    remove_inscribe_transfer_inscription(&mut table, inscription_id)
  }
}

impl<'a, 'db, 'txn> OrdReaderWriter for SimulateContext<'a, 'db, 'txn> {
  fn save_transaction_operations(
    &mut self,
    txid: &Txid,
    operations: &[InscriptionOp],
  ) -> crate::Result<(), Self::Error> {
    let mut traces = self.traces.borrow_mut();
    let key = &txid.store();
    let key = TxidValue::as_slice(key).to_vec();
    traces.push(TraceNode {
      trace_type: CacheTableIndex::ORD_TX_TO_OPERATIONS,
      key,
    });

    let mut ord_traces = self.ord_operations.borrow_mut();
    ord_traces.extend_from_slice(operations);

    let mut table = self.ORD_TX_TO_OPERATIONS.borrow_mut();
    save_transaction_operations(&mut table, txid, operations)
  }

  fn set_inscription_by_collection_key(
    &mut self,
    key: &str,
    inscription_id: &InscriptionId,
  ) -> crate::Result<(), Self::Error> {
    let mut traces = self.traces.borrow_mut();
    let trace_key = key.as_bytes().to_vec();
    traces.push(TraceNode {
      trace_type: CacheTableIndex::COLLECTIONS_KEY_TO_INSCRIPTION_ID,
      key: trace_key,
    });
    let mut table = self.COLLECTIONS_KEY_TO_INSCRIPTION_ID.borrow_mut();
    set_inscription_by_collection_key(&mut table, key, inscription_id)
  }

  fn set_inscription_attributes(
    &mut self,
    inscription_id: &InscriptionId,
    kind: &[CollectionKind],
  ) -> crate::Result<(), Self::Error> {
    let mut traces = self.traces.borrow_mut();
    let key = inscription_id.store();
    let key = InscriptionIdValue::as_bytes(&key);
    traces.push(TraceNode {
      trace_type: CacheTableIndex::COLLECTIONS_INSCRIPTION_ID_TO_KINDS,
      key,
    });
    let mut table = self.COLLECTIONS_INSCRIPTION_ID_TO_KINDS.borrow_mut();
    set_inscription_attributes(&mut table, inscription_id, kind)
  }
}

impl<'a, 'db, 'txn> OrdReader for SimulateContext<'a, 'db, 'txn> {
  type Error = anyhow::Error;

  fn get_inscription_number_by_sequence_number(
    &self,
    sequence_number: u32,
  ) -> crate::Result<i32, Self::Error> {
    let binding = self.SEQUENCE_NUMBER_TO_INSCRIPTION_ENTRY.borrow();
    let table = binding.deref();
    let ret = get_inscription_number_by_sequence_number(table, sequence_number)
      .map_err(|e| anyhow!("failed to get inscription number from state! error: {e}"))?;
    if let Some(ret) = ret {
      return Ok(ret);
    }
    self
      .use_internal_table(SEQUENCE_NUMBER_TO_INSCRIPTION_ENTRY, |table| {
        get_inscription_number_by_sequence_number(&table, sequence_number)
      })
      .map_err(|e| anyhow!("failed to get inscription number from state! error: {e}"))?
      .ok_or(anyhow!(
        "failed to get inscription number! error: sequence number {} not found",
        sequence_number
      ))
  }

  fn get_script_key_on_satpoint(
    &mut self,
    satpoint: &SatPoint,
    network: Network,
  ) -> crate::Result<ScriptKey, Self::Error> {
    let binding = self.OUTPOINT_TO_ENTRY.borrow();
    let table = binding.deref();
    if let Some(tx_out) = get_txout_by_outpoint(table, &satpoint.outpoint)? {
      return Ok(ScriptKey::from_script(&tx_out.script_pubkey, network));
    } else {
      let ret = self.use_internal_table(OUTPOINT_TO_ENTRY, |table| {
        get_txout_by_outpoint(&table, &satpoint.outpoint)
      })?;
      if let Some(ret) = ret {
        return Ok(ScriptKey::from_script(&ret.script_pubkey, network));
      }
    }
    Err(anyhow!(
      "failed to get tx out! error: outpoint {} not found",
      &satpoint.outpoint
    ))
  }

  fn get_transaction_operations(
    &self,
    txid: &Txid,
  ) -> crate::Result<Vec<InscriptionOp>, Self::Error> {
    let binding = self.ORD_TX_TO_OPERATIONS.borrow();
    let table = binding.deref();
    let simulate = get_transaction_operations(table, txid)?;
    let mut simulate_operations: HashMap<InscriptionId, InscriptionOp> = simulate
      .into_iter()
      .map(|v| (v.inscription_id, v.clone()))
      .collect();
    let internal = self.use_internal_table(BRC20_EVENTS, |table| {
      get_transaction_operations(&table, txid)
    })?;
    for node in internal {
      if simulate_operations.contains_key(&node.inscription_id) {
        continue;
      }
      simulate_operations.insert(node.inscription_id, node.clone());
    }
    let ret = simulate_operations.into_values().collect();
    Ok(ret)
  }

  fn get_collections_of_inscription(
    &self,
    inscription_id: &InscriptionId,
  ) -> crate::Result<Option<Vec<CollectionKind>>, Self::Error> {
    let binding = self.COLLECTIONS_INSCRIPTION_ID_TO_KINDS.borrow();
    let table = binding.deref();
    let simulate = get_collections_of_inscription(table, inscription_id)?;
    let mut simulate = if let Some(ret) = simulate {
      ret
    } else {
      vec![]
    };

    let internal = self.use_internal_table(COLLECTIONS_INSCRIPTION_ID_TO_KINDS, |table| {
      get_collections_of_inscription(&table, inscription_id)
    })?;
    if let Some(internal) = internal {
      simulate.extend_from_slice(&internal);
    }
    if simulate.is_empty() {
      return Ok(None);
    }
    Ok(Some(simulate))
  }

  fn get_collection_inscription_id(
    &self,
    collection_key: &str,
  ) -> crate::Result<Option<InscriptionId>, Self::Error> {
    let binding = self.COLLECTIONS_KEY_TO_INSCRIPTION_ID.borrow();
    let table = binding.deref();
    let ret = get_collection_inscription_id(table, collection_key)?;
    if let Some(ret) = ret {
      return Ok(Some(ret));
    }
    self.use_internal_table(COLLECTIONS_KEY_TO_INSCRIPTION_ID, |table| {
      get_collection_inscription_id(&table, collection_key)
    })
  }
}

impl<'a, 'db, 'txn> ContextTrait for SimulateContext<'a, 'db, 'txn> {
  fn block_height(&self) -> u32 {
    self.current_height
  }

  fn network(&self) -> Network {
    self.network
  }

  fn block_time(&self) -> u32 {
    self.current_block_time
  }
}

impl<'a, 'db, 'txn> SimulateContext<'a, 'db, 'txn> {
  fn use_internal_table<K: RedbKey + 'static, V: RedbValue + 'static, T>(
    &self,
    table_def: TableDefinition<K, V>,
    f: impl FnOnce(ReadOnlyTable<K, V>) -> crate::Result<T>,
  ) -> crate::Result<T> {
    let rtx = self.internal_index.internal.begin_read()?;
    let table = rtx.0.open_table(table_def)?;
    f(table)
  }
}
