use super::*;
use crate::okx::datastore::ord::btc_name::BtcDomain;

pub(crate) struct Rtx<'a>(pub(crate) redb::ReadTransaction<'a>);

impl Rtx<'_> {
  pub(crate) fn block_height(&self) -> Result<Option<Height>> {
    Ok(
      self
        .0
        .open_table(HEIGHT_TO_BLOCK_HEADER)?
        .range(0..)?
        .next_back()
        .and_then(|result| result.ok())
        .map(|(height, _header)| Height(height.value())),
    )
  }

  pub(crate) fn block_count(&self) -> Result<u32> {
    Ok(
      self
        .0
        .open_table(HEIGHT_TO_BLOCK_HEADER)?
        .range(0..)?
        .next_back()
        .and_then(|result| result.ok())
        .map(|(height, _header)| height.value() + 1)
        .unwrap_or(0),
    )
  }

  pub(crate) fn block_hash(&self, height: Option<u32>) -> Result<Option<BlockHash>> {
    let height_to_block_header = self.0.open_table(HEIGHT_TO_BLOCK_HEADER)?;

    Ok(
      match height {
        Some(height) => height_to_block_header.get(height)?,
        None => height_to_block_header
          .range(0..)?
          .next_back()
          .transpose()?
          .map(|(_height, header)| header),
      }
      .map(|header| Header::load(*header.value()).block_hash()),
    )
  }

  pub(crate) fn latest_block(&self) -> Result<Option<(Height, BlockHash)>> {
    Ok(
      self
        .0
        .open_table(HEIGHT_TO_BLOCK_HEADER)?
        .range(0..)?
        .next_back()
        .and_then(|result| result.ok())
        .map(|(height, hash)| {
          (
            Height(height.value()),
            Header::load(*hash.value()).block_hash(),
          )
        }),
    )
  }

  pub(crate) fn inscription_id_to_sequence_number(
    &self,
    inscription_id: InscriptionId,
  ) -> Result<Option<u32>> {
    Ok(
      self
        .0
        .open_table(INSCRIPTION_ID_TO_SEQUENCE_NUMBER)?
        .get(&inscription_id.store())?
        .map(|guard| guard.value()),
    )
  }

  pub(crate) fn inscription_number_to_sequence_number(
    &self,
    inscription_number: i32,
  ) -> Result<Option<u32>> {
    Ok(
      self
        .0
        .open_table(INSCRIPTION_NUMBER_TO_SEQUENCE_NUMBER)?
        .get(inscription_number)?
        .map(|guard| guard.value()),
    )
  }

  pub(crate) fn sequence_number_to_satpoint(
    &self,
    sequence_number: u32,
  ) -> Result<Option<SatPoint>> {
    Ok(
      self
        .0
        .open_table(SEQUENCE_NUMBER_TO_SATPOINT)?
        .get(sequence_number)?
        .map(|satpoint| Entry::load(*satpoint.value())),
    )
  }

  pub(crate) fn inscriptions_on_output_with_satpoints(
    &self,
    outpoint: OutPoint,
  ) -> Result<Vec<(SatPoint, InscriptionId)>> {
    let satpoint_to_sequence_number = self.0.open_multimap_table(SATPOINT_TO_SEQUENCE_NUMBER)?;
    let sequence_number_to_inscription_entry =
      self.0.open_table(SEQUENCE_NUMBER_TO_INSCRIPTION_ENTRY)?;

    Index::inscriptions_on_output(
      &satpoint_to_sequence_number,
      &sequence_number_to_inscription_entry,
      outpoint,
    )
  }

  pub(crate) fn sequence_number_to_inscription_entry(
    &self,
    sequence_number: u32,
  ) -> Result<Option<InscriptionEntry>> {
    Ok(
      self
        .0
        .open_table(SEQUENCE_NUMBER_TO_INSCRIPTION_ENTRY)?
        .get(sequence_number)?
        .map(|value| InscriptionEntry::load(value.value())),
    )
  }

  pub(crate) fn transaction_id_to_transaction(&self, txid: Txid) -> Result<Option<Transaction>> {
    Ok(
      self
        .0
        .open_table(TRANSACTION_ID_TO_TRANSACTION)?
        .get(&txid.store())?
        .map(|transaction| consensus::encode::deserialize(transaction.value()))
        .transpose()?,
    )
  }

  pub(crate) fn outpoint_to_entry(&self, outpoint: OutPoint) -> Result<Option<TxOut>> {
    let table = self.0.open_table(OUTPOINT_TO_ENTRY)?;
    get_txout_by_outpoint(&table, &outpoint)
  }

  pub(crate) fn get_inscription_entry(
    &self,
    inscription_id: InscriptionId,
  ) -> Result<Option<InscriptionEntry>> {
    if let Some(sequence_number) = self.inscription_id_to_sequence_number(inscription_id)? {
      self.sequence_number_to_inscription_entry(sequence_number)
    } else {
      Ok(None)
    }
  }

  pub(crate) fn ord_inscription_id_to_collections(
    &self,
    inscription_id: InscriptionId,
  ) -> Result<Option<Vec<CollectionKind>>> {
    let table = self
      .0
      .open_multimap_table(COLLECTIONS_INSCRIPTION_ID_TO_KINDS)?;
    get_collections_of_inscription(&table, &inscription_id)
  }

  pub(crate) fn ord_district_to_inscription_id(
    &self,
    number: u32,
  ) -> Result<Option<InscriptionId>> {
    let district = District { number };
    let table = self.0.open_table(COLLECTIONS_KEY_TO_INSCRIPTION_ID)?;
    get_collection_inscription_id(&table, &district.to_collection_key())
  }

  pub(crate) fn btc_name_to_inscription_id(&self, btc_name: &str) -> Result<Option<InscriptionId>> {
    let btc_name_raw = btc_name.as_bytes().to_vec();
    let domain = BtcDomain::parse(&btc_name_raw)?;
    let table = self.0.open_table(COLLECTIONS_KEY_TO_INSCRIPTION_ID)?;
    get_collection_inscription_id(&table, &domain.to_collection_key())
  }

  pub(crate) fn ord_transaction_id_to_inscription_operations(
    &self,
    txid: Txid,
  ) -> Result<Option<Vec<ord::InscriptionOp>>> {
    let table = self.0.open_table(ORD_TX_TO_OPERATIONS)?;
    get_transaction_operations(&table, &txid)
  }

  pub(crate) fn brc20_get_tick_info(&self, name: &brc20::Tick) -> Result<Option<brc20::TokenInfo>> {
    let table = self.0.open_table(BRC20_TOKEN)?;
    get_token_info(&table, name)
  }

  pub(crate) fn brc20_get_all_tick_info(&self) -> Result<Vec<brc20::TokenInfo>> {
    let table = self.0.open_table(BRC20_TOKEN)?;
    get_tokens_info(&table)
  }

  pub(crate) fn brc20_get_balance_by_address(
    &self,
    tick: &brc20::Tick,
    script_key: ScriptKey,
  ) -> Result<Option<brc20::Balance>> {
    let table = self.0.open_table(BRC20_BALANCES)?;
    get_balance(&table, &script_key, tick)
  }

  pub(crate) fn brc20_get_all_balance_by_address(
    &self,
    script_key: ScriptKey,
  ) -> Result<Vec<brc20::Balance>> {
    let table = self.0.open_table(BRC20_BALANCES)?;
    get_balances(&table, &script_key)
  }

  pub(crate) fn brc20_transaction_id_to_transaction_receipt(
    &self,
    txid: Txid,
  ) -> Result<Option<Vec<brc20::Receipt>>> {
    let table = self.0.open_table(BRC20_EVENTS)?;
    get_transaction_receipts(&table, &txid)
  }

  pub(crate) fn brc20_get_tick_transferable_by_address(
    &self,
    tick: &brc20::Tick,
    script_key: ScriptKey,
  ) -> Result<Vec<(SatPoint, brc20::TransferableLog)>> {
    let address_table = self
      .0
      .open_multimap_table(BRC20_ADDRESS_TICKER_TO_TRANSFERABLE_ASSETS)?;
    let satpoint_table = self.0.open_table(BRC20_SATPOINT_TO_TRANSFERABLE_ASSETS)?;
    get_transferable_assets_by_account_ticker(&address_table, &satpoint_table, &script_key, tick)
  }

  pub(crate) fn brc20_get_all_transferable_by_address(
    &self,
    script_key: ScriptKey,
  ) -> Result<Vec<(SatPoint, brc20::TransferableLog)>> {
    let address_table = self
      .0
      .open_multimap_table(BRC20_ADDRESS_TICKER_TO_TRANSFERABLE_ASSETS)?;
    let satpoint_table = self.0.open_table(BRC20_SATPOINT_TO_TRANSFERABLE_ASSETS)?;
    get_transferable_assets_by_account(&address_table, &satpoint_table, &script_key)
  }

  pub(crate) fn brc20_transferable_assets_on_output_with_satpoints(
    &self,
    outpoint: OutPoint,
  ) -> Result<Vec<(SatPoint, brc20::TransferableLog)>> {
    let satpoint_to_sequence_number = self.0.open_table(BRC20_SATPOINT_TO_TRANSFERABLE_ASSETS)?;
    get_transferable_assets_by_outpoint(&satpoint_to_sequence_number, outpoint)
  }

  pub(super) fn list_sat_range(&self, outpoint: OutPointValue) -> Result<Option<Vec<u8>>> {
    Ok(
      self
        .0
        .open_table(OUTPOINT_TO_SAT_RANGES)?
        .get(&outpoint)?
        .map(|outpoint| outpoint.value().to_vec()),
    )
  }
}
