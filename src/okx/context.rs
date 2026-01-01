use {
  super::{
    brc20::{
      entry::{
        BRC20Balance, BRC20BalanceValue, BRC20Predeploy, BRC20PredeployValue, BRC20ProgCall,
        BRC20ProgCallValue, BRC20ProgDeploy, BRC20ProgDeployValue, BRC20ProgTransact,
        BRC20ProgTransactValue, BRC20Receipt, BRC20ReceiptsValue, BRC20TickerInfo,
        BRC20TickerInfoValue, BRC20TransferAsset, BRC20TransferAssetValue, BRC20Withdraw,
        BRC20WithdrawValue, OpiBlockValidation, OpiBlockValidationValue,
      },
      BRC20Ticker,
    },
    composite_key::{AddressEndpoint, AddressTickerKey},
    entry::{
      AddressTickerKeyValue, CollectionType, DynamicEntry, InscriptionReceipt,
      InscriptionReceiptsValue,
    },
    utxo_address::UtxoAddress,
  },
  crate::{
    index::entry::{Entry, SatPointValue, TxidValue},
    InscriptionId,
  },
  bitcoin::Txid,
  ordinals::SatPoint,
  redb::{MultimapTable, ReadableTable, Table},
};

pub(crate) struct TableContext<'a, 'txn> {
  inscription_receipts: &'a mut Table<'txn, &'static TxidValue, &'static InscriptionReceiptsValue>,
  // BRC20 tables
  brc20_balances: &'a mut Table<'txn, &'static AddressTickerKeyValue, &'static BRC20BalanceValue>,
  // Inscription ID as str -> BRC20 Predeploy Hash, Deployer Address and Block Height
  brc20_predeploys: &'a mut Table<'txn, &'static str, &'static BRC20PredeployValue>,
  brc20_ticker_info:
    &'a mut Table<'txn, &'static AddressTickerKeyValue, &'static BRC20TickerInfoValue>,
  brc20_receipts: &'a mut Table<'txn, &'static TxidValue, &'static BRC20ReceiptsValue>,
  brc20_satpoint_to_transfer_assets:
    &'a mut Table<'txn, &'static SatPointValue, &'static BRC20TransferAssetValue>,
  brc20_address_ticker_to_transfer_assets:
    &'a mut MultimapTable<'txn, &'static AddressTickerKeyValue, &'static SatPointValue>,
  sequence_number_to_collection_type: &'a mut Table<'txn, u32, u16>,
  bitmap_block_height_to_sequence_number: &'a mut Table<'txn, u32, u32>,
  btc_domain_to_sequence_number: &'a mut Table<'txn, &'static str, u32>,

  brc20_satpoint_to_prog_deploy_assets:
    &'a mut Table<'txn, &'static SatPointValue, &'static BRC20ProgDeployValue>,
  brc20_satpoint_to_prog_call_assets:
    &'a mut Table<'txn, &'static SatPointValue, &'static BRC20ProgCallValue>,
  brc20_satpoint_to_prog_transact_assets:
    &'a mut Table<'txn, &'static SatPointValue, &'static BRC20ProgTransactValue>,

  brc20_satpoint_to_withdraw_assets:
    &'a mut Table<'txn, &'static SatPointValue, &'static BRC20WithdrawValue>,
  opi_block_validations: &'a mut Table<'txn, u32, &'static OpiBlockValidationValue>,
}

impl<'a, 'txn> TableContext<'a, 'txn> {
  pub fn new(
    inscription_receipts: &'a mut Table<
      'txn,
      &'static TxidValue,
      &'static InscriptionReceiptsValue,
    >,
    brc20_balances: &'a mut Table<'txn, &'static AddressTickerKeyValue, &'static BRC20BalanceValue>,
    brc20_predeploys: &'a mut Table<'txn, &'static str, &'static BRC20PredeployValue>,
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
    sequence_number_to_collection_type: &'a mut Table<'txn, u32, u16>,
    bitmap_block_height_to_sequence_number: &'a mut Table<'txn, u32, u32>,
    btc_domain_to_sequence_number: &'a mut Table<'txn, &'static str, u32>,
    brc20_satpoint_to_prog_deploy_assets: &'a mut Table<
      'txn,
      &'static SatPointValue,
      &'static BRC20ProgDeployValue,
    >,
    brc20_satpoint_to_prog_call_assets: &'a mut Table<
      'txn,
      &'static SatPointValue,
      &'static BRC20ProgCallValue,
    >,
    brc20_satpoint_to_prog_transact_assets: &'a mut Table<
      'txn,
      &'static SatPointValue,
      &'static BRC20ProgTransactValue,
    >,
    brc20_satpoint_to_withdraw_assets: &'a mut Table<
      'txn,
      &'static SatPointValue,
      &'static BRC20WithdrawValue,
    >,
    opi_block_validations: &'a mut Table<'txn, u32, &'static OpiBlockValidationValue>,
  ) -> Self {
    Self {
      inscription_receipts,
      brc20_balances,
      brc20_predeploys,
      brc20_ticker_info,
      brc20_receipts,
      brc20_satpoint_to_transfer_assets,
      brc20_address_ticker_to_transfer_assets,
      sequence_number_to_collection_type,
      bitmap_block_height_to_sequence_number,
      btc_domain_to_sequence_number,
      brc20_satpoint_to_prog_deploy_assets,
      brc20_satpoint_to_prog_call_assets,
      brc20_satpoint_to_prog_transact_assets,
      brc20_satpoint_to_withdraw_assets,
      opi_block_validations,
    }
  }

  pub fn insert_brc20_predeploy(
    &mut self,
    inscription_id: &InscriptionId,
    predeploy: BRC20Predeploy,
  ) -> Result<(), redb::StorageError> {
    self.brc20_predeploys.insert(
      inscription_id.to_string().as_str(),
      predeploy.store().as_ref(),
    )?;
    Ok(())
  }

  pub fn load_brc20_predeploy(
    &mut self,
    inscription_id: &InscriptionId,
  ) -> Result<Option<BRC20Predeploy>, redb::StorageError> {
    Ok(
      self
        .brc20_predeploys
        .get(inscription_id.to_string().as_str())?
        .map(|v| DynamicEntry::load(v.value())),
    )
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

  pub fn load_brc20_balances_by_address(
    &mut self,
    address: &UtxoAddress,
  ) -> Result<Vec<BRC20Balance>, redb::StorageError> {
    Ok(
      self
        .brc20_balances
        .range(
          AddressEndpoint::Left(address.clone()).store().as_ref()
            ..=AddressEndpoint::Right(address.clone()).store().as_ref(),
        )?
        .flat_map(|result| result.map(|(_, v)| BRC20Balance::load(v.value())))
        .collect(),
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

  pub fn pop_brc20_prog_call_asset(
    &mut self,
    satpoint: SatPoint,
  ) -> Result<Option<BRC20ProgCall>, redb::StorageError> {
    let value = self
      .brc20_satpoint_to_prog_call_assets
      .get(&satpoint.store())?
      .map(|v| DynamicEntry::load(v.value()));
    if value.is_some() {
      self
        .brc20_satpoint_to_prog_call_assets
        .remove(&satpoint.store())?;
    }
    Ok(value)
  }

  pub fn pop_brc20_prog_transact_asset(
    &mut self,
    satpoint: SatPoint,
  ) -> Result<Option<BRC20ProgTransact>, redb::StorageError> {
    let value = self
      .brc20_satpoint_to_prog_transact_assets
      .get(&satpoint.store())?
      .map(|v| DynamicEntry::load(v.value()));
    if value.is_some() {
      self
        .brc20_satpoint_to_prog_transact_assets
        .remove(&satpoint.store())?;
    }
    Ok(value)
  }

  pub fn pop_brc20_prog_deploy_asset(
    &mut self,
    satpoint: SatPoint,
  ) -> Result<Option<BRC20ProgDeploy>, redb::StorageError> {
    let value = self
      .brc20_satpoint_to_prog_deploy_assets
      .get(&satpoint.store())?
      .map(|v| DynamicEntry::load(v.value()));
    if value.is_some() {
      self
        .brc20_satpoint_to_prog_deploy_assets
        .remove(&satpoint.store())?;
    }
    Ok(value)
  }

  pub fn pop_brc20_prog_withdraw_asset(
    &mut self,
    satpoint: SatPoint,
  ) -> Result<Option<BRC20Withdraw>, redb::StorageError> {
    let value = self
      .brc20_satpoint_to_withdraw_assets
      .get(&satpoint.store())?
      .map(|v| DynamicEntry::load(v.value()));
    if value.is_some() {
      self
        .brc20_satpoint_to_withdraw_assets
        .remove(&satpoint.store())?;
    }
    Ok(value)
  }

  pub fn insert_brc20_prog_deploy_asset(
    &mut self,
    satpoint: SatPoint,
    asset: BRC20ProgDeploy,
  ) -> std::result::Result<(), redb::StorageError> {
    self
      .brc20_satpoint_to_prog_deploy_assets
      .insert(&satpoint.store(), asset.store().as_ref())?;
    Ok(())
  }

  pub fn insert_brc20_prog_call_asset(
    &mut self,
    satpoint: SatPoint,
    asset: BRC20ProgCall,
  ) -> std::result::Result<(), redb::StorageError> {
    self
      .brc20_satpoint_to_prog_call_assets
      .insert(&satpoint.store(), asset.store().as_ref())?;
    Ok(())
  }

  pub fn insert_brc20_prog_transact_asset(
    &mut self,
    satpoint: SatPoint,
    asset: BRC20ProgTransact,
  ) -> std::result::Result<(), redb::StorageError> {
    self
      .brc20_satpoint_to_prog_transact_assets
      .insert(&satpoint.store(), asset.store().as_ref())?;
    Ok(())
  }

  pub fn insert_brc20_withdraw_asset(
    &mut self,
    satpoint: SatPoint,
    asset: BRC20Withdraw,
  ) -> std::result::Result<(), redb::StorageError> {
    self
      .brc20_satpoint_to_withdraw_assets
      .insert(&satpoint.store(), asset.store().as_ref())?;
    Ok(())
  }

  pub fn load_brc20_transferring_asset(
    &mut self,
    satpoint: SatPoint,
  ) -> std::result::Result<Option<BRC20TransferAsset>, redb::StorageError> {
    Ok(
      self
        .brc20_satpoint_to_transfer_assets
        .get(&satpoint.store())?
        .map(|v| DynamicEntry::load(v.value())),
    )
  }

  pub fn remove_brc20_transferring_asset(
    &mut self,
    satpoint: SatPoint,
  ) -> std::result::Result<(), redb::StorageError> {
    if let Some(asset) = self
      .brc20_satpoint_to_transfer_assets
      .remove(&satpoint.store())?
      .map(|v| BRC20TransferAsset::load(v.value()))
    {
      self.brc20_address_ticker_to_transfer_assets.remove(
        AddressTickerKey {
          primary: asset.owner,
          secondary: asset.original_ticker.to_lowercase(),
        }
        .store()
        .as_ref(),
        &satpoint.store(),
      )?;
    }
    Ok(())
  }

  pub fn insert_brc20_transferring_asset(
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

  pub fn load_bitmap_block_height_to_sequence_number(
    &mut self,
    height: u32,
  ) -> Result<Option<u32>, redb::StorageError> {
    Ok(
      self
        .bitmap_block_height_to_sequence_number
        .get(&height)?
        .map(|v| v.value()),
    )
  }

  pub fn load_btc_domain_to_sequence_number(
    &mut self,
    domain: &str,
  ) -> Result<Option<u32>, redb::StorageError> {
    Ok(
      self
        .btc_domain_to_sequence_number
        .get(domain)?
        .map(|v| v.value()),
    )
  }

  pub fn insert_bitmap_block_height_to_sequence_number(
    &mut self,
    height: u32,
    sequence_number: u32,
  ) -> Result<(), redb::StorageError> {
    self
      .bitmap_block_height_to_sequence_number
      .insert(height, sequence_number)?;
    Ok(())
  }

  pub fn insert_btc_domain_to_sequence_number(
    &mut self,
    domain: &str,
    sequence_number: u32,
  ) -> Result<(), redb::StorageError> {
    self
      .btc_domain_to_sequence_number
      .insert(domain, sequence_number)?;
    Ok(())
  }

  pub fn insert_sequence_number_to_collection_type(
    &mut self,
    sequence_number: u32,
    collection_type: CollectionType,
  ) -> Result<(), redb::StorageError> {
    self
      .sequence_number_to_collection_type
      .insert(sequence_number, u16::from(collection_type))?;
    Ok(())
  }

  pub fn insert_opi_block_validation(
    &mut self,
    height: u32,
    validation: OpiBlockValidation,
  ) -> Result<(), redb::StorageError> {
    self
      .opi_block_validations
      .insert(height, validation.store().as_ref())?;
    Ok(())
  }

  pub fn get_opi_block_validations(
    &mut self,
    height: u32,
  ) -> Result<Option<OpiBlockValidation>, redb::StorageError> {
    Ok(
      self
        .opi_block_validations
        .get(height)?
        .map(|v| DynamicEntry::load(v.value())),
    )
  }
}
