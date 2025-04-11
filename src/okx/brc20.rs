use super::{entry::DynamicEntry, *};
use crate::index::Curse;
use crate::Chain;
use fixed_point::FixedPoint;
use once_cell::sync::Lazy;
use operation::{BRC20OperationExtractor, Deploy, Mint, RawOperation, Transfer};
use policies::HardForks;

pub(crate) mod entry;
mod error;
pub(crate) mod event;
mod executor;
mod fixed_point;
mod operation;
mod policies;
mod ticker;
mod utils;

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
pub enum BRC20Operation {
  Deploy(Deploy),
  Mint {
    op: Mint,
    signer: Option<UtxoAddress>,
    parent: Option<InscriptionId>,
  },
  InscribeTransfer {
    signer: Option<UtxoAddress>,
    transfer: Transfer,
  },
  Transfer {
    ticker: BRC20Ticker,
    amount: u128,
  },
}

pub trait BRC20CreationOperationExtractor {
  fn extract_and_validate_creation(&self, height: u32, chain: Chain) -> Option<BRC20Operation>;
}

pub trait BRC20TransferOperationExtractor<'a, 'tx> {
  fn extract_and_validate_transfer(
    &self,
    context: &mut TableContext,
  ) -> Result<Option<BRC20Operation>>;
}

#[derive(Debug)]
pub struct CreatedInscription<'a> {
  pub txid: Txid,
  pub inscription: &'a Inscription,
  pub inscription_id: InscriptionId,
  pub sequence_number: u32,
  pub inscription_number: i32,
  pub parents: &'a Vec<InscriptionId>,
  pub new_satpoint: SatPoint,
  pub pre_jubilant_curse_reason: Option<&'a Curse>,
  pub charms: u16,
  pub tapscript_pk: [u8; 35],
}

impl CreatedInscription<'_> {
  pub fn is_change(&self) -> bool {
    self.new_satpoint.outpoint.txid != self.txid
  }
}

impl<'a> From<&'a OkxInscriptionEvent> for Option<CreatedInscription<'a>> {
  fn from(event: &'a OkxInscriptionEvent) -> Self {
    match &event.action {
      Action::Created {
        inscription,
        parents,
        pre_jubilant_curse_reason,
        charms,
        tapscript_pk,
        ..
      } => Some(CreatedInscription {
        txid: event.txid,
        inscription,
        inscription_id: event.inscription_id,
        sequence_number: event.sequence_number,
        inscription_number: event.inscription_number,
        parents: &parents,
        new_satpoint: event.new_satpoint,
        pre_jubilant_curse_reason: pre_jubilant_curse_reason.as_ref(),
        charms: *charms,
        tapscript_pk: *tapscript_pk,
      }),
      _ => None,
    }
  }
}

impl BRC20CreationOperationExtractor for CreatedInscription<'_> {
  fn extract_and_validate_creation(&self, height: u32, chain: Chain) -> Option<BRC20Operation> {
    // Creation inscription transferred to the coinbase as change does not qualify as a BRC20 operation.
    if self.is_change() {
      return None;
    }

    if HardForks::check_inscription_preconditions(
      height,
      &chain,
      self.charms,
      self.pre_jubilant_curse_reason,
    ) {
      let vindicated_set = HardForks::check_inscription_vindicated(self.charms);
      let mut address_type = if height < HardForks::self_single_step_transfer_activation_height(&chain) {
        0
      }else {
        self.tapscript_pk[34]
      };
      let mut signer = if address_type > 0 {
        let script = utils::get_pk_script_by_pubkey_and_type(&self.tapscript_pk[1..33], address_type);
        Some(UtxoAddress::from_script(script.as_script(), &chain))
      } else {
        None
      };

      match self.inscription.extract_brc20_operation() {
        Ok(RawOperation::Deploy(mut deploy)) => {
          if vindicated_set {
            return None;
          }
          // Filter out invalid deployments with a 5-byte ticker.
          // proposal for issuance self mint token.
          // https://l1f.discourse.group/t/brc-20-proposal-for-issuance-and-burn-enhancements-brc20-ip-1/621
          if deploy.tick.len() == SELF_ISSUANCE_TICKER_LENGTH {
            if !deploy.self_mint.unwrap_or_default() {
              log::debug!(
                "Self mint is not enabled for inscription: {} with ticker length: {}",
                self.inscription_id,
                SELF_ISSUANCE_TICKER_LENGTH
              );
              return None;
            }
            if height < HardForks::self_issuance_activation_height(&chain) {
              log::debug!(
                "Self mint is not activated at height: {} for inscription: {}",
                height,
                self.inscription_id
              );
              return None;
            }
          } else {
            deploy.self_mint = None;
          }
          Some(BRC20Operation::Deploy(deploy))
        }
        Ok(RawOperation::Mint(mint)) => {
          if vindicated_set {
            return None;
          }
          if mint.tick.len() != SELF_ISSUANCE_TICKER_LENGTH {
            signer = None;
          }
          Some(BRC20Operation::Mint {
            op: mint,
            signer,
            parent: self.parents.first().cloned(),
          })
        }
        Ok(RawOperation::Transfer(transfer)) => {
          if transfer.tick.len() != SELF_ISSUANCE_TICKER_LENGTH {
            signer = None;
            address_type = 0;
          }
          if vindicated_set && address_type == 0 {
            return None;
          }
          Some(BRC20Operation::InscribeTransfer {
            signer,
            transfer,
          })
        }
        _ => None,
      }
    } else {
      None
    }
  }
}

#[derive(Debug)]
pub struct TransferredInscription {
  pub inscription_id: InscriptionId,
  pub inscription_number: i32,
  pub old_satpoint: SatPoint,
  pub sender: UtxoAddress,
}
impl From<&BundleMessage> for Option<TransferredInscription> {
  fn from(message: &BundleMessage) -> Self {
    match message.inscription_action {
      InscriptionAction::Transferred { .. } => Some(TransferredInscription {
        inscription_id: message.inscription_id,
        inscription_number: message.inscription_number,
        old_satpoint: message.old_satpoint,
        sender: message.sender.clone(),
      }),
      _ => None,
    }
  }
}
impl BRC20TransferOperationExtractor<'_, '_> for TransferredInscription {
  fn extract_and_validate_transfer(
    &self,
    context: &mut TableContext,
  ) -> Result<Option<BRC20Operation>> {
    if self.inscription_number >= 0 && self.old_satpoint.outpoint.txid == self.inscription_id.txid {
      let Some(asset) = context.load_brc20_transferring_asset(self.old_satpoint)? else {
        return Ok(None);
      };

      // Since a single old_satpoint may correspond to multiple inscriptions,
      // we need to verify whether the current inscription_id matches the asset's inscription_id.
      // Only if they match can it be considered a valid BRC20 transfer message.
      if self.inscription_id != asset.inscription_id {
        return Ok(None);
      }

      // Remove the asset from tables.
      context.remove_brc20_transferring_asset(self.old_satpoint)?;
      return Ok(Some(BRC20Operation::Transfer {
        ticker: asset.ticker,
        amount: asset.amount,
      }));
    }
    Ok(None)
  }
}
