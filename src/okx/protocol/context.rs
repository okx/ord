use crate::{
  index::{
    entry::SatPointValue, InscriptionEntryValue, InscriptionIdValue, OutPointValue, TxidValue,
  },
  inscriptions::InscriptionId,
  okx::{
    datastore::{
      brc20::{
        redb::table::{
          get_balance, get_token_info, get_transferable_assets_by_satpoint, insert_token_info,
          insert_transferable_asset, remove_transferable_asset, save_transaction_receipts,
          update_burned_token_info, update_mint_token_info, update_token_balance,
        },
        Balance, Brc20Reader, Brc20ReaderWriter, Receipt, Tick, TokenInfo, TransferableLog,
      },
      ord::{
        collections::CollectionKind,
        redb::table::{
          add_inscription_attributes, get_collection_inscription_id,
          get_inscription_number_by_sequence_number, get_txout_by_outpoint,
          save_transaction_operations, set_inscription_by_collection_key,
        },
        InscriptionOp, OrdReader, OrdReaderWriter,
      },
      ScriptKey,
    },
    lru::SimpleLru,
    protocol::ChainContext,
  },
  Chain, SatPoint,
};
use anyhow::anyhow;
use bitcoin::{OutPoint, TxOut, Txid};
use redb::{MultimapTable, Table};

#[allow(non_snake_case)]
pub struct Context<'a, 'db, 'txn> {
  pub(crate) chain_conf: ChainContext,
  pub(crate) tx_out_cache: &'a mut SimpleLru<OutPoint, TxOut>,
  pub(crate) hit: u64,
  pub(crate) miss: u64,

  // ord tables
  pub(crate) ORD_TX_TO_OPERATIONS: &'a mut Table<'db, 'txn, &'static TxidValue, &'static [u8]>,
  pub(crate) COLLECTIONS_KEY_TO_INSCRIPTION_ID:
    &'a mut Table<'db, 'txn, &'static str, InscriptionIdValue>,
  pub(crate) COLLECTIONS_INSCRIPTION_ID_TO_KINDS:
    &'a mut MultimapTable<'db, 'txn, InscriptionIdValue, &'static [u8]>,
  pub(crate) SEQUENCE_NUMBER_TO_INSCRIPTION_ENTRY:
    &'a mut Table<'db, 'txn, u32, InscriptionEntryValue>,
  pub(crate) OUTPOINT_TO_ENTRY: &'a mut Table<'db, 'txn, &'static OutPointValue, &'static [u8]>,

  // BRC20 tables
  pub(crate) BRC20_BALANCES: &'a mut Table<'db, 'txn, &'static str, &'static [u8]>,
  pub(crate) BRC20_TOKEN: &'a mut Table<'db, 'txn, &'static str, &'static [u8]>,
  pub(crate) BRC20_EVENTS: &'a mut Table<'db, 'txn, &'static TxidValue, &'static [u8]>,
  pub(crate) BRC20_SATPOINT_TO_TRANSFERABLE_ASSETS:
    &'a mut Table<'db, 'txn, &'static SatPointValue, &'static [u8]>,
  pub(crate) BRC20_ADDRESS_TICKER_TO_TRANSFERABLE_ASSETS:
    &'a mut MultimapTable<'db, 'txn, &'static str, &'static SatPointValue>,
}

impl<'a, 'db, 'txn> OrdReader for Context<'a, 'db, 'txn> {
  type Error = anyhow::Error;

  fn get_inscription_number_by_sequence_number(
    &self,
    sequence_number: u32,
  ) -> crate::Result<i32, Self::Error> {
    get_inscription_number_by_sequence_number(
      self.SEQUENCE_NUMBER_TO_INSCRIPTION_ENTRY,
      sequence_number,
    )
    .map_err(|e| anyhow!("failed to get inscription number from state! error: {e}"))?
    .ok_or(anyhow!(
      "failed to get inscription number! error: sequence number {} not found",
      sequence_number
    ))
  }

  fn get_script_key_on_satpoint(
    &mut self,
    satpoint: &SatPoint,
    chain: Chain,
  ) -> crate::Result<ScriptKey, Self::Error> {
    if let Some(tx_out) = self.tx_out_cache.get(&satpoint.outpoint) {
      self.hit += 1;
      Ok(ScriptKey::from_script(&tx_out.script_pubkey, chain))
    } else if let Some(tx_out) = get_txout_by_outpoint(self.OUTPOINT_TO_ENTRY, &satpoint.outpoint)?
    {
      self.miss += 1;
      Ok(ScriptKey::from_script(&tx_out.script_pubkey, chain))
    } else {
      Err(anyhow!(
        "failed to get tx out! error: outpoint {} not found",
        &satpoint.outpoint
      ))
    }
  }

  // fn get_transaction_operations(
  //   &self,
  //   txid: &Txid,
  // ) -> crate::Result<Option<Vec<InscriptionOp>>, Self::Error> {
  //   get_transaction_operations(self.ORD_TX_TO_OPERATIONS, txid)
  // }

  // fn get_collections_of_inscription(
  //   &self,
  //   inscription_id: &InscriptionId,
  // ) -> crate::Result<Option<Vec<CollectionKind>>, Self::Error> {
  //   get_collections_of_inscription(self.COLLECTIONS_INSCRIPTION_ID_TO_KINDS, inscription_id)
  // }

  fn get_collection_inscription_id(
    &self,
    collection_key: &str,
  ) -> crate::Result<Option<InscriptionId>, Self::Error> {
    get_collection_inscription_id(self.COLLECTIONS_KEY_TO_INSCRIPTION_ID, collection_key)
  }
}

impl<'a, 'db, 'txn> OrdReaderWriter for Context<'a, 'db, 'txn> {
  fn save_transaction_operations(
    &mut self,
    txid: &Txid,
    operations: &[InscriptionOp],
  ) -> crate::Result<(), Self::Error> {
    save_transaction_operations(self.ORD_TX_TO_OPERATIONS, txid, operations)
  }

  fn set_inscription_by_collection_key(
    &mut self,
    key: &str,
    inscription_id: &InscriptionId,
  ) -> crate::Result<(), Self::Error> {
    set_inscription_by_collection_key(self.COLLECTIONS_KEY_TO_INSCRIPTION_ID, key, inscription_id)
  }

  fn add_inscription_attributes(
    &mut self,
    inscription_id: &InscriptionId,
    kind: CollectionKind,
  ) -> crate::Result<(), Self::Error> {
    add_inscription_attributes(
      self.COLLECTIONS_INSCRIPTION_ID_TO_KINDS,
      inscription_id,
      kind,
    )
  }
}

impl<'a, 'db, 'txn> Brc20Reader for Context<'a, 'db, 'txn> {
  type Error = anyhow::Error;

  // fn get_balances(&self, script_key: &ScriptKey) -> crate::Result<Vec<Balance>, Self::Error> {
  //   get_balances(self.BRC20_BALANCES, script_key)
  // }

  fn get_balance(
    &self,
    script_key: &ScriptKey,
    tick: &Tick,
  ) -> crate::Result<Option<Balance>, Self::Error> {
    get_balance(self.BRC20_BALANCES, script_key, tick)
  }

  fn get_token_info(&self, tick: &Tick) -> crate::Result<Option<TokenInfo>, Self::Error> {
    get_token_info(self.BRC20_TOKEN, tick)
  }

  // fn get_tokens_info(&self) -> crate::Result<Vec<TokenInfo>, Self::Error> {
  //   get_tokens_info(self.BRC20_TOKEN)
  // }

  // fn get_transaction_receipts(
  //   &self,
  //   txid: &Txid,
  // ) -> crate::Result<Option<Vec<Receipt>>, Self::Error> {
  //   get_transaction_receipts(self.BRC20_EVENTS, txid)
  // }

  // fn get_transferable_assets_by_account(
  //   &self,
  //   script: &ScriptKey,
  // ) -> crate::Result<Vec<(SatPoint, TransferableLog)>, Self::Error> {
  //   get_transferable_assets_by_account(
  //     self.BRC20_ADDRESS_TICKER_TO_TRANSFERABLE_ASSETS,
  //     self.BRC20_SATPOINT_TO_TRANSFERABLE_ASSETS,
  //     script,
  //   )
  // }

  // fn get_transferable_assets_by_account_ticker(
  //   &self,
  //   script: &ScriptKey,
  //   tick: &Tick,
  // ) -> crate::Result<Vec<(SatPoint, TransferableLog)>, Self::Error> {
  //   get_transferable_assets_by_account_ticker(
  //     self.BRC20_ADDRESS_TICKER_TO_TRANSFERABLE_ASSETS,
  //     self.BRC20_SATPOINT_TO_TRANSFERABLE_ASSETS,
  //     script,
  //     tick,
  //   )
  // }

  fn get_transferable_assets_by_satpoint(
    &self,
    satpoint: &SatPoint,
  ) -> crate::Result<Option<TransferableLog>, Self::Error> {
    get_transferable_assets_by_satpoint(self.BRC20_SATPOINT_TO_TRANSFERABLE_ASSETS, satpoint)
  }

  // fn get_transferable_assets_by_outpoint(
  //   &self,
  //   outpoint: OutPoint,
  // ) -> crate::Result<Vec<(SatPoint, TransferableLog)>, Self::Error> {
  //   get_transferable_assets_by_outpoint(self.BRC20_SATPOINT_TO_TRANSFERABLE_ASSETS, outpoint)
  // }
}

impl<'a, 'db, 'txn> Brc20ReaderWriter for Context<'a, 'db, 'txn> {
  fn update_token_balance(
    &mut self,
    script_key: &ScriptKey,
    new_balance: Balance,
  ) -> crate::Result<(), Self::Error> {
    update_token_balance(self.BRC20_BALANCES, script_key, new_balance)
  }

  fn insert_token_info(
    &mut self,
    tick: &Tick,
    new_info: &TokenInfo,
  ) -> crate::Result<(), Self::Error> {
    insert_token_info(self.BRC20_TOKEN, tick, new_info)
  }

  fn update_mint_token_info(
    &mut self,
    tick: &Tick,
    minted_amt: u128,
    minted_block_number: u32,
  ) -> crate::Result<(), Self::Error> {
    update_mint_token_info(self.BRC20_TOKEN, tick, minted_amt, minted_block_number)
  }

  fn update_burned_token_info(
    &mut self,
    tick: &Tick,
    burned_amt: u128,
  ) -> crate::Result<(), Self::Error> {
    update_burned_token_info(self.BRC20_TOKEN, tick, burned_amt)
  }

  fn save_transaction_receipts(
    &mut self,
    txid: &Txid,
    receipt: &[Receipt],
  ) -> crate::Result<(), Self::Error> {
    save_transaction_receipts(self.BRC20_EVENTS, txid, receipt)
  }

  fn insert_transferable_asset(
    &mut self,
    satpoint: SatPoint,
    transferable_asset: &TransferableLog,
  ) -> crate::Result<(), Self::Error> {
    insert_transferable_asset(
      self.BRC20_SATPOINT_TO_TRANSFERABLE_ASSETS,
      self.BRC20_ADDRESS_TICKER_TO_TRANSFERABLE_ASSETS,
      satpoint,
      transferable_asset,
    )
  }

  fn remove_transferable_asset(&mut self, satpoint: SatPoint) -> crate::Result<(), Self::Error> {
    remove_transferable_asset(
      self.BRC20_SATPOINT_TO_TRANSFERABLE_ASSETS,
      self.BRC20_ADDRESS_TICKER_TO_TRANSFERABLE_ASSETS,
      satpoint,
    )
  }
}
