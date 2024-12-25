use super::*;
use crate::index::entry::{Entry, SatPointValue};
use crate::okx::brc20::operation::{BRC20OperationExtractor, Deploy, Mint, RawOperation, Transfer};
use crate::okx::brc20::policies::HardForks;
use crate::okx::composite_key::AddressTickerKey;
use crate::okx::entry::{AddressTickerKeyValue, DynamicEntry};
use crate::okx::InscriptionMessage;
use crate::Chain;
use once_cell::sync::Lazy;
use redb::{MultimapTable, Table};

pub(crate) mod entry;
mod error;
pub(crate) mod event;
mod executor;
mod num;
mod operation;
mod policies;
mod ticker;

pub const MAX_DECIMAL_WIDTH: u8 = 18;

pub static MAXIMUM_SUPPLY: Lazy<Num> = Lazy::new(|| Num::from(u64::MAX));

pub static BIGDECIMAL_TEN: Lazy<Num> = Lazy::new(|| Num::from(10u64));

pub(crate) use self::{
  executor::BRC20ExecutionMessage,
  ticker::{BRC20LowerCaseTicker, BRC20Ticker},
};
use crate::okx::brc20::entry::BRC20TransferAssetValue;
use crate::okx::brc20::num::Num;
use entry::BRC20TransferAsset;

#[derive(Debug, Clone)]
pub enum BRC20Message {
  Deploy(Deploy),
  Mint {
    op: Mint,
    parent: Option<InscriptionId>,
  },
  InscribeTransfer(Transfer),
  Transfer {
    ticker: BRC20Ticker,
    amount: u128,
  },
}

pub trait BRC20MessageExtractor<'a, 'tx> {
  fn extract_brc20_message(
    &self,
    height: u32,
    chain: Chain,
    satpoint_to_assets_table: &'a mut Table<
      'tx,
      &'static SatPointValue,
      &'static BRC20TransferAssetValue,
    >,
    address_to_assets_table: &'a mut MultimapTable<
      'tx,
      &'static AddressTickerKeyValue,
      &'static SatPointValue,
    >,
  ) -> Result<Option<BRC20Message>>;
}

impl<'a, 'tx> BRC20MessageExtractor<'a, 'tx> for InscriptionMessage {
  fn extract_brc20_message(
    &self,
    height: u32,
    chain: Chain,
    satpoint_to_assets_table: &'a mut Table<
      'tx,
      &'static SatPointValue,
      &'static BRC20TransferAssetValue,
    >,
    address_to_assets_table: &'a mut MultimapTable<
      'tx,
      &'static AddressTickerKeyValue,
      &'static SatPointValue,
    >,
  ) -> Result<Option<BRC20Message>> {
    match &self.action {
      Action::Created {
        inscription,
        parents,
        pre_jubilant_curse_reason,
        charms,
      } if HardForks::check_inscription_preconditions(
        height,
        &chain,
        *charms,
        pre_jubilant_curse_reason.as_ref(),
      ) =>
      {
        match inscription.extract_brc20_operation() {
          Ok(RawOperation::Deploy(deploy)) => Ok(Some(BRC20Message::Deploy(deploy))),
          Ok(RawOperation::Mint(mint)) => Ok(Some(BRC20Message::Mint {
            op: mint,
            parent: parents.first().cloned(),
          })),
          Ok(RawOperation::Transfer(transfer)) => {
            Ok(Some(BRC20Message::InscribeTransfer(transfer)))
          }
          _ => {
            return Ok(None);
          }
        }
      }
      Action::Transferred => {
        let Some(asset) = satpoint_to_assets_table
          .remove(&self.old_satpoint.store())?
          .map(|asset| BRC20TransferAsset::load(asset.value()))
        else {
          return Ok(None);
        };
        assert_eq!(asset.inscription_id, self.inscription_id);

        // Remove the asset from the address-ticker to satpoint mapping.
        address_to_assets_table.remove(
          AddressTickerKey {
            primary: self.sender.clone(),
            secondary: asset.ticker.to_lowercase(),
          }
          .store()
          .as_ref(),
          &self.old_satpoint.store(),
        )?;
        Ok(Some(BRC20Message::Transfer {
          ticker: asset.ticker,
          amount: asset.amount,
        }))
      }
      _ => {
        return Ok(None);
      }
    }
  }
}
