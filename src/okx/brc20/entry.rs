use super::*;
use event::{BRC20Event, BRC20OpType};

pub type BRC20BalanceValue = [u8];
impl_bincode_dynamic_entry!(BRC20Balance, BRC20BalanceValue);
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BRC20Balance {
  pub ticker: BRC20Ticker,
  pub total: u128,
  pub available: u128,
}

impl BRC20Balance {
  pub fn new_with_ticker(ticker: &BRC20Ticker) -> Self {
    Self {
      ticker: ticker.clone(),
      total: 0,
      available: 0,
    }
  }
}

pub type BRC20TickerValue = [u8];
impl_bincode_dynamic_entry!(BRC20Ticker, BRC20TickerValue);

pub type BRC20LowerCaseTickerValue = [u8];
impl_bincode_dynamic_entry!(BRC20LowerCaseTicker, BRC20LowerCaseTickerValue);

pub(crate) type BRC20TickerInfoValue = [u8];
impl_bincode_dynamic_entry!(BRC20TickerInfo, BRC20TickerInfoValue);
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BRC20TickerInfo {
  pub ticker: BRC20Ticker,
  pub sequence_number: u32,
  pub inscription_number: i32,
  pub inscription_id: InscriptionId,
  pub total_supply: u128,
  pub burned: u128,
  pub minted: u128,
  pub max_mint_limit: u128,
  pub decimals: u8,
  pub deployer: UtxoAddress,
  pub self_minted: bool,
  pub deployed_block_height: u32,
  pub deployed_timestamp: u32,
  pub latest_minted_block_height: u32,
}

pub(crate) type BRC20TransferAssetValue = [u8];
impl_bincode_dynamic_entry!(BRC20TransferAsset, BRC20TransferAssetValue);
#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct BRC20TransferAsset {
  pub ticker: BRC20Ticker,
  pub amount: u128,
  pub owner: UtxoAddress,
  pub sequence_number: u32,
  pub inscription_number: i32,
  pub inscription_id: InscriptionId,
}

pub(crate) type BRC20ReceiptsValue = [u8];
impl_bincode_dynamic_entry!(Vec<BRC20Receipt>, BRC20ReceiptsValue);
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct BRC20Receipt {
  pub inscription_id: InscriptionId,
  pub sequence_number: u32,
  pub inscription_number: i32,
  pub old_satpoint: SatPoint,
  pub new_satpoint: SatPoint,
  pub op_type: BRC20OpType,
  pub sender: UtxoAddress,
  pub receiver: UtxoAddress,
  pub result: Result<BRC20Event, error::BRC20Error>,
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::define_table;
  use redb::{ReadableTable, TableDefinition};
  use tempfile::NamedTempFile;

  #[test]
  fn test_store() {
    let ticker = BRC20Ticker::from_str("aBcD").unwrap();
    let value = ticker.clone().store();
    assert_eq!(value.to_vec(), vec![4, 0, 0, 0, 0, 0, 0, 0, 97, 66, 99, 68]);
    let lower_ticker = ticker.to_lowercase();
    let value = lower_ticker.store();
    assert_eq!(
      value.to_vec(),
      vec![4, 0, 0, 0, 0, 0, 0, 0, 97, 98, 99, 100]
    );
  }

  #[test]
  fn test_load() {
    let value = vec![4, 0, 0, 0, 0, 0, 0, 0, 97, 66, 99, 68];
    let ticker = BRC20Ticker::load(&value);
    assert_eq!(ticker, BRC20Ticker::from_str("aBcD").unwrap());
    let value = vec![4, 0, 0, 0, 0, 0, 0, 0, 97, 98, 99, 100];
    let lower_ticker = BRC20LowerCaseTicker::load(&value);
    assert_eq!(
      lower_ticker,
      BRC20Ticker::from_str("abcd").unwrap().to_lowercase()
    );
  }

  #[test]
  fn test_lower_case_ticker_as_key() {
    define_table!(LOWER_CASE_TICKER, &BRC20LowerCaseTickerValue, u32);

    let db_file = NamedTempFile::new().unwrap();
    let database = redb::Database::builder().create(db_file.path()).unwrap();
    let wtx = database.begin_write().unwrap();
    let mut table = wtx.open_table(LOWER_CASE_TICKER).unwrap();

    let lower_case_ticker = BRC20Ticker::from_str("keys").unwrap().to_lowercase();

    table
      .insert(lower_case_ticker.clone().store().as_ref(), 1)
      .unwrap();

    let retrieved_value = table
      .get(lower_case_ticker.store().as_ref())
      .unwrap()
      .unwrap()
      .value();
    assert_eq!(retrieved_value, 1);
  }

  #[test]
  fn test_lower_case_ticker_as_value() {
    define_table!(LOWER_CASE_TICKER, u32, &BRC20LowerCaseTickerValue);

    let db_file = NamedTempFile::new().unwrap();
    let database = redb::Database::builder().create(db_file.path()).unwrap();
    let wtx = database.begin_write().unwrap();
    let mut table = wtx.open_table(LOWER_CASE_TICKER).unwrap();

    let lower_case_ticker = BRC20Ticker::from_str("value").unwrap().to_lowercase();

    table
      .insert(1, lower_case_ticker.clone().store().as_ref())
      .unwrap();

    let retrieved_value = table
      .get(1)
      .unwrap()
      .map(|v| BRC20LowerCaseTicker::load(v.value()))
      .unwrap();
    assert_eq!(retrieved_value, lower_case_ticker);
  }

  #[test]
  fn test_ticker() {
    define_table!(TICKER, u32, &BRC20TickerValue);

    let db_file = NamedTempFile::new().unwrap();
    let database = redb::Database::builder().create(db_file.path()).unwrap();
    let wtx = database.begin_write().unwrap();
    let mut table = wtx.open_table(TICKER).unwrap();

    let ticker = BRC20Ticker::from_str("value").unwrap();

    table.insert(1, ticker.clone().store().as_ref()).unwrap();

    let retrieved_value = table
      .get(1)
      .unwrap()
      .map(|v| BRC20Ticker::load(v.value()))
      .unwrap();

    assert_eq!(retrieved_value, ticker);
  }
}
