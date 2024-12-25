use super::*;
use crate::index::entry::{Entry, SatPointValue, TxidValue};
use crate::okx::brc20::entry::{
  BRC20Balance, BRC20BalanceValue, BRC20Receipt, BRC20ReceiptsValue, BRC20TickerInfo,
  BRC20TickerInfoValue, BRC20TransferAsset, BRC20TransferAssetValue,
};
use crate::okx::brc20::BRC20Ticker;
use crate::okx::composite_key::AddressTickerKey;
use crate::okx::entry::{
  AddressTickerKeyValue, DynamicEntry, InscriptionReceipt, InscriptionReceiptsValue,
};
use redb::{MultimapTable, ReadableTable, Table};

pub(crate) struct TableContext<'a, 'txn> {
  inscription_receipts: &'a mut Table<'txn, &'static TxidValue, &'static InscriptionReceiptsValue>,
  // BRC20 tables
  brc20_balances: &'a mut Table<'txn, &'static AddressTickerKeyValue, &'static BRC20BalanceValue>,
  brc20_ticker_info:
    &'a mut Table<'txn, &'static AddressTickerKeyValue, &'static BRC20TickerInfoValue>,
  brc20_receipts: &'a mut Table<'txn, &'static TxidValue, &'static BRC20ReceiptsValue>,
  brc20_satpoint_to_transfer_assets:
    &'a mut Table<'txn, &'static SatPointValue, &'static BRC20TransferAssetValue>,
  brc20_address_ticker_to_transfer_assets:
    &'a mut MultimapTable<'txn, &'static AddressTickerKeyValue, &'static SatPointValue>,
}

impl<'a, 'txn> TableContext<'a, 'txn> {
  pub fn new(
    inscription_receipts: &'a mut Table<
      'txn,
      &'static TxidValue,
      &'static InscriptionReceiptsValue,
    >,
    brc20_balances: &'a mut Table<'txn, &'static AddressTickerKeyValue, &'static BRC20BalanceValue>,
    brc20_ticker_info: &'a mut Table<
      'txn,
      &'static AddressTickerKeyValue,
      &'static BRC20TickerInfoValue,
    >,
    brc20_receipts: &'a mut Table<'txn, &'static TxidValue, &'static BRC20ReceiptsValue>,
    brc20_satpoint_to_transfer_assets: &'a mut Table<
      'txn,
      &'static SatPointValue,
      &'static BRC20TransferAssetValue,
    >,
    brc20_address_ticker_to_transfer_assets: &'a mut MultimapTable<
      'txn,
      &'static AddressTickerKeyValue,
      &'static SatPointValue,
    >,
  ) -> Self {
    Self {
      inscription_receipts,
      brc20_balances,
      brc20_ticker_info,
      brc20_receipts,
      brc20_satpoint_to_transfer_assets,
      brc20_address_ticker_to_transfer_assets,
    }
  }

  pub fn load_brc20_ticker_info(
    &mut self,
    ticker: &BRC20Ticker,
  ) -> Result<Option<BRC20TickerInfo>, redb::StorageError> {
    Ok(
      self
        .brc20_ticker_info
        .get(ticker.to_lowercase().store().as_ref())?
        .map(|v| DynamicEntry::load(v.value())),
    )
  }

  pub fn update_brc20_ticker_info(
    &mut self,
    ticker: &BRC20Ticker,
    info: BRC20TickerInfo,
  ) -> Result<(), redb::StorageError> {
    self.brc20_ticker_info.insert(
      ticker.to_lowercase().store().as_ref(),
      info.store().as_ref(),
    )?;
    Ok(())
  }

  pub fn load_brc20_balance(
    &mut self,
    address: &UtxoAddress,
    ticker: &BRC20Ticker,
  ) -> Result<Option<BRC20Balance>, redb::StorageError> {
    Ok(
      self
        .brc20_balances
        .get(
          AddressTickerKey {
            primary: address.clone(),
            secondary: ticker.to_lowercase().clone(),
          }
          .store()
          .as_ref(),
        )?
        .map(|v| DynamicEntry::load(v.value())),
    )
  }

  pub fn update_brc20_balance(
    &mut self,
    address: &UtxoAddress,
    ticker: &BRC20Ticker,
    balance: BRC20Balance,
  ) -> Result<(), redb::StorageError> {
    self.brc20_balances.insert(
      AddressTickerKey {
        primary: address.clone(),
        secondary: ticker.to_lowercase().clone(),
      }
      .store()
      .as_ref(),
      balance.store().as_ref(),
    )?;
    Ok(())
  }

  pub fn update_brc20_transferring_asset(
    &mut self,
    address: &UtxoAddress,
    ticker: &BRC20Ticker,
    satpoint: SatPoint,
    asset: BRC20TransferAsset,
  ) -> std::result::Result<(), redb::StorageError> {
    self
      .brc20_satpoint_to_transfer_assets
      .insert(&satpoint.store(), asset.store().as_ref())?;
    self.brc20_address_ticker_to_transfer_assets.insert(
      AddressTickerKey {
        primary: address.clone(),
        secondary: ticker.to_lowercase().clone(),
      }
      .store()
      .as_ref(),
      &satpoint.store(),
    )?;
    Ok(())
  }

  pub fn insert_brc20_tx_receipts(
    &mut self,
    txid: &Txid,
    receipts: Vec<BRC20Receipt>,
  ) -> std::result::Result<(), redb::StorageError> {
    self
      .brc20_receipts
      .insert(&txid.store(), receipts.store().as_ref())?;
    Ok(())
  }

  pub fn insert_inscription_tx_receipts(
    &mut self,
    txid: &Txid,
    receipts: Vec<InscriptionReceipt>,
  ) -> std::result::Result<(), redb::StorageError> {
    self
      .inscription_receipts
      .insert(&txid.store(), receipts.store().as_ref())?;
    Ok(())
  }
}
