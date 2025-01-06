use super::{
  composite_key::AddressTickerKey,
  entry::{AddressTickerKeyValue, DynamicEntry},
  *,
};
use crate::index::entry::{Entry, SatPointValue};
use crate::Chain;
use entry::BRC20TransferAssetValue;
use fixed_point::FixedPoint;
use once_cell::sync::Lazy;
use operation::{BRC20OperationExtractor, Deploy, Mint, RawOperation, Transfer};
use policies::HardForks;
use redb::{MultimapTable, Table};

pub(crate) mod entry;
mod error;
pub(crate) mod event;
mod executor;
mod fixed_point;
mod operation;
mod policies;
mod ticker;

pub static MAXIMUM_SUPPLY: Lazy<FixedPoint> =
  Lazy::new(|| FixedPoint::new_unchecked(u128::from(u64::MAX), 0));

pub(crate) use self::{
  entry::{BRC20Balance, BRC20Receipt, BRC20TickerInfo, BRC20TransferAsset},
  error::BRC20Error,
  executor::BRC20ExecutionMessage,
  ticker::{BRC20LowerCaseTicker, BRC20Ticker},
};
const SELF_ISSUANCE_TICKER_LENGTH: usize = 5;
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

impl<'a, 'tx> BRC20MessageExtractor<'a, 'tx> for OkxInscriptionEvent {
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
          Ok(RawOperation::Deploy(mut deploy)) => {
            // Filter out invalid deployments with a 5-byte ticker.
            // proposal for issuance self mint token.
            // https://l1f.discourse.group/t/brc-20-proposal-for-issuance-and-burn-enhancements-brc20-ip-1/621
            if deploy.tick.len() == SELF_ISSUANCE_TICKER_LENGTH {
              if !deploy.self_mint.unwrap_or_default() {
                return Ok(None);
              }
              if height < HardForks::self_issuance_activation_height(&chain) {
                return Ok(None);
              }
            } else {
              deploy.self_mint = None;
            }
            Ok(Some(BRC20Message::Deploy(deploy)))
          }
          Ok(RawOperation::Mint(mint)) => Ok(Some(BRC20Message::Mint {
            op: mint,
            parent: parents.first().cloned(),
          })),
          Ok(RawOperation::Transfer(transfer)) => {
            Ok(Some(BRC20Message::InscribeTransfer(transfer)))
          }
          _ => Ok(None),
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
      _ => Ok(None),
    }
  }
}
