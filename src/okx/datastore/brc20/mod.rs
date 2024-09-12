pub(super) mod balance;
pub(super) mod errors;
pub(super) mod events;
pub mod redb;
pub(super) mod tick;
pub(super) mod token_info;
pub(super) mod transferable_log;

pub use self::{
  balance::Balance, errors::BRC20Error, events::Receipt, events::*, tick::*, token_info::TokenInfo,
  transferable_log::TransferableLog,
};
use super::ScriptKey;
use crate::{Result, SatPoint};
use bitcoin::Txid;
use std::fmt::{Debug, Display};

pub trait Brc20Reader {
  type Error: Debug + Display;

  // fn get_balances(&self, script_key: &ScriptKey) -> Result<Vec<Balance>, Self::Error>;
  fn get_balance(
    &self,
    script_key: &ScriptKey,
    tick: &Tick,
  ) -> Result<Option<Balance>, Self::Error>;

  fn get_token_info(&self, tick: &Tick) -> Result<Option<TokenInfo>, Self::Error>;
  // fn get_tokens_info(&self) -> Result<Vec<TokenInfo>, Self::Error>;

  // fn get_transaction_receipts(&self, txid: &Txid) -> Result<Option<Vec<Receipt>>, Self::Error>;

  fn get_transferable_assets_by_satpoint(
    &self,
    satpoint: &SatPoint,
  ) -> Result<Option<TransferableLog>, Self::Error>;
  // fn get_transferable_assets_by_account(
  //   &self,
  //   script: &ScriptKey,
  // ) -> Result<Vec<(SatPoint, TransferableLog)>, Self::Error>;
  // fn get_transferable_assets_by_account_ticker(
  //   &self,
  //   script: &ScriptKey,
  //   tick: &Tick,
  // ) -> Result<Vec<(SatPoint, TransferableLog)>, Self::Error>;
  // fn get_transferable_assets_by_outpoint(
  //   &self,
  //   outpoint: OutPoint,
  // ) -> Result<Vec<(SatPoint, TransferableLog)>, Self::Error>;
}

pub trait Brc20ReaderWriter: Brc20Reader {
  fn update_token_balance(
    &mut self,
    script_key: &ScriptKey,
    new_balance: Balance,
  ) -> Result<(), Self::Error>;

  fn insert_token_info(&mut self, tick: &Tick, new_info: &TokenInfo) -> Result<(), Self::Error>;

  fn update_mint_token_info(
    &mut self,
    tick: &Tick,
    minted_amt: u128,
    minted_block_number: u32,
  ) -> Result<(), Self::Error>;

  fn update_burned_token_info(&mut self, tick: &Tick, burned_amt: u128) -> Result<(), Self::Error>;

  fn save_transaction_receipts(
    &mut self,
    txid: &Txid,
    receipt: &[Receipt],
  ) -> Result<(), Self::Error>;

  fn insert_transferable_asset(
    &mut self,
    satpoint: SatPoint,
    inscription: &TransferableLog,
  ) -> Result<(), Self::Error>;

  fn remove_transferable_asset(&mut self, satpoint: SatPoint) -> Result<(), Self::Error>;
}
