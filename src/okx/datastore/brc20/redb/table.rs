use crate::{
  index::{
    entry::{Entry, SatPointValue},
    TxidValue,
  },
  okx::datastore::{
    brc20::{
      redb::{
        max_script_tick_id_key, max_script_tick_key, min_script_tick_id_key, min_script_tick_key,
        script_tick_key,
      },
      Balance, Receipt, Tick, TokenInfo, TransferableLog,
    },
    ScriptKey,
  },
  Result, SatPoint,
};
use bitcoin::{OutPoint, Txid};
use redb::{MultimapTable, ReadableMultimapTable, ReadableTable, Table};

// BRC20_BALANCES
pub fn get_balances<T>(table: &T, script_key: &ScriptKey) -> Result<Vec<Balance>>
where
  T: ReadableTable<&'static str, &'static [u8]>,
{
  Ok(
    table
      .range(min_script_tick_key(script_key).as_str()..=max_script_tick_key(script_key).as_str())?
      .flat_map(|result| {
        result.map(|(_, data)| rmp_serde::from_slice::<Balance>(data.value()).unwrap())
      })
      .collect(),
  )
}

// BRC20_BALANCES
pub fn get_balance<T>(table: &T, script_key: &ScriptKey, tick: &Tick) -> Result<Option<Balance>>
where
  T: ReadableTable<&'static str, &'static [u8]>,
{
  Ok(
    table
      .get(script_tick_key(script_key, tick).as_str())?
      .map(|v| rmp_serde::from_slice::<Balance>(v.value()).unwrap()),
  )
}

// BRC20_TOKEN
pub fn get_token_info<T>(table: &T, tick: &Tick) -> Result<Option<TokenInfo>>
where
  T: ReadableTable<&'static str, &'static [u8]>,
{
  Ok(
    table
      .get(tick.to_lowercase().hex().as_str())?
      .map(|v| rmp_serde::from_slice::<TokenInfo>(v.value()).unwrap()),
  )
}

// BRC20_TOKEN
pub fn get_tokens_info<T>(table: &T) -> Result<Vec<TokenInfo>>
where
  T: ReadableTable<&'static str, &'static [u8]>,
{
  Ok(
    table
      .range::<&str>(..)?
      .flat_map(|result| {
        result.map(|(_, data)| rmp_serde::from_slice::<TokenInfo>(data.value()).unwrap())
      })
      .collect(),
  )
}

// BRC20_EVENTS
pub fn get_transaction_receipts<T>(table: &T, txid: &Txid) -> Result<Option<Vec<Receipt>>>
where
  T: ReadableTable<&'static TxidValue, &'static [u8]>,
{
  Ok(
    table
      .get(&txid.store())?
      .map(|x| rmp_serde::from_slice::<Vec<Receipt>>(x.value()).unwrap()),
  )
}

// BRC20_SATPOINT_TO_TRANSFERABLE_ASSETS
// BRC20_ADDRESS_TICKER_TO_TRANSFERABLE_ASSETS
pub fn get_transferable_assets_by_account<T, S>(
  address_table: &T,
  satpoint_table: &S,
  script: &ScriptKey,
) -> Result<Vec<(SatPoint, TransferableLog)>>
where
  T: ReadableMultimapTable<&'static str, &'static SatPointValue>,
  S: ReadableTable<&'static SatPointValue, &'static [u8]>,
{
  let mut transferable_assets = Vec::new();

  for range in address_table
    .range(min_script_tick_key(script).as_str()..max_script_tick_key(script).as_str())?
  {
    let (_, satpoints) = range?;
    for satpoint_guard in satpoints {
      let satpoint = SatPoint::load(*satpoint_guard?.value());
      let entry = satpoint_table.get(&satpoint.store())?.unwrap();
      transferable_assets.push((
        satpoint,
        rmp_serde::from_slice::<TransferableLog>(entry.value()).unwrap(),
      ));
    }
  }
  Ok(transferable_assets)
}

// BRC20_SATPOINT_TO_TRANSFERABLE_ASSETS
// BRC20_ADDRESS_TICKER_TO_TRANSFERABLE_ASSETS
pub fn get_transferable_assets_by_account_ticker<T, S>(
  address_table: &T,
  satpoint_table: &S,
  script: &ScriptKey,
  tick: &Tick,
) -> Result<Vec<(SatPoint, TransferableLog)>>
where
  T: ReadableMultimapTable<&'static str, &'static SatPointValue>,
  S: ReadableTable<&'static SatPointValue, &'static [u8]>,
{
  let mut transferable_assets = Vec::new();

  for range in address_table.range(
    min_script_tick_id_key(script, tick).as_str()..max_script_tick_id_key(script, tick).as_str(),
  )? {
    let (_, satpoints) = range?;
    for satpoint_guard in satpoints {
      let satpoint = SatPoint::load(*satpoint_guard?.value());
      let entry = satpoint_table.get(&satpoint.store())?.unwrap();
      transferable_assets.push((
        satpoint,
        rmp_serde::from_slice::<TransferableLog>(entry.value()).unwrap(),
      ));
    }
  }
  Ok(transferable_assets)
}

// BRC20_SATPOINT_TO_TRANSFERABLE_ASSETS
pub fn get_transferable_assets_by_satpoint<T>(
  table: &T,
  satpoint: &SatPoint,
) -> Result<Option<TransferableLog>>
where
  T: ReadableTable<&'static SatPointValue, &'static [u8]>,
{
  Ok(
    table
      .get(&satpoint.store())?
      .map(|entry| rmp_serde::from_slice::<TransferableLog>(entry.value()).unwrap()),
  )
}

// BRC20_SATPOINT_TO_TRANSFERABLE_ASSETS
pub fn get_transferable_assets_by_outpoint<T>(
  table: &T,
  outpoint: OutPoint,
) -> Result<Vec<(SatPoint, TransferableLog)>>
where
  T: ReadableTable<&'static SatPointValue, &'static [u8]>,
{
  let start = SatPoint {
    outpoint,
    offset: 0,
  }
  .store();

  let end = SatPoint {
    outpoint,
    offset: u64::MAX,
  }
  .store();

  let mut transferable_assets = Vec::new();
  for range in table.range::<&[u8; 44]>(&start..&end)? {
    let (satpoint_guard, asset) = range?;
    let satpoint = SatPoint::load(*satpoint_guard.value());
    transferable_assets.push((
      satpoint,
      rmp_serde::from_slice::<TransferableLog>(asset.value()).unwrap(),
    ));
  }
  Ok(transferable_assets)
}

// BRC20_BALANCES
pub fn update_token_balance(
  table: &mut Table<'_, '_, &'static str, &'static [u8]>,
  script_key: &ScriptKey,
  new_balance: Balance,
) -> Result<()> {
  table.insert(
    script_tick_key(script_key, &new_balance.tick).as_str(),
    rmp_serde::to_vec(&new_balance).unwrap().as_slice(),
  )?;
  Ok(())
}

// BRC20_TOKEN
pub fn insert_token_info(
  table: &mut Table<'_, '_, &'static str, &'static [u8]>,
  tick: &Tick,
  new_info: &TokenInfo,
) -> Result<()> {
  table.insert(
    tick.to_lowercase().hex().as_str(),
    rmp_serde::to_vec(new_info).unwrap().as_slice(),
  )?;
  Ok(())
}

// BRC20_TOKEN
pub fn update_mint_token_info(
  table: &mut Table<'_, '_, &'static str, &'static [u8]>,
  tick: &Tick,
  minted_amt: u128,
  minted_block_number: u32,
) -> Result<()> {
  let mut info =
    get_token_info(table, tick)?.unwrap_or_else(|| panic!("token {} not exist", tick.as_str()));

  info.minted = minted_amt;
  info.latest_mint_number = minted_block_number;

  table.insert(
    tick.to_lowercase().hex().as_str(),
    rmp_serde::to_vec(&info).unwrap().as_slice(),
  )?;
  Ok(())
}

// BRC20_EVENTS
pub fn save_transaction_receipts(
  table: &mut Table<'_, '_, &'static TxidValue, &'static [u8]>,
  txid: &Txid,
  receipts: &[Receipt],
) -> Result<()> {
  table.insert(
    &txid.store(),
    rmp_serde::to_vec(receipts).unwrap().as_slice(),
  )?;
  Ok(())
}

// BRC20_SATPOINT_TO_TRANSFERABLE_ASSETS
// BRC20_ADDRESS_TICKER_TO_TRANSFERABLE_ASSETS
pub fn insert_transferable_asset(
  satpoint_table: &mut Table<'_, '_, &'static SatPointValue, &'static [u8]>,
  address_table: &mut MultimapTable<'_, '_, &'static str, &'static SatPointValue>,
  satpoint: SatPoint,
  transferable_asset: &TransferableLog,
) -> Result<()> {
  satpoint_table.insert(
    &satpoint.store(),
    rmp_serde::to_vec(&transferable_asset).unwrap().as_slice(),
  )?;
  address_table.insert(
    script_tick_key(&transferable_asset.owner, &transferable_asset.tick).as_str(),
    &satpoint.store(),
  )?;
  Ok(())
}

// BRC20_SATPOINT_TO_TRANSFERABLE_ASSETS
// BRC20_ADDRESS_TICKER_TO_TRANSFERABLE_ASSETS
pub fn remove_transferable_asset(
  satpoint_table: &mut Table<'_, '_, &'static SatPointValue, &'static [u8]>,
  address_table: &mut MultimapTable<'_, '_, &'static str, &'static SatPointValue>,
  satpoint: SatPoint,
) -> Result<()> {
  if let Some(guard) = satpoint_table.remove(&satpoint.store())? {
    let transferable_asset = rmp_serde::from_slice::<TransferableLog>(guard.value()).unwrap();
    address_table.remove(
      script_tick_key(&transferable_asset.owner, &transferable_asset.tick).as_str(),
      &satpoint.store(),
    )?;
  }
  Ok(())
}
