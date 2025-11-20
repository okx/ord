use super::{entry::DynamicEntry, *};
use crate::Chain;
use crate::{index::Curse, okx::brc20::operation::Predeploy};
use fixed_point::FixedPoint;
use once_cell::sync::Lazy;
use operation::{
  BRC20OperationExtractor, Deploy, Mint, ProgCall, ProgDeploy, ProgTransact, RawOperation,
  Transfer, Withdraw,
};
use policies::HardForks;

pub(crate) mod entry;
mod error;
pub(crate) mod event;
mod executor;
pub mod event_hash;
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
const PREDEPLOYED_TICKER_LENGTH: usize = 6;

#[derive(Debug, Clone)]
pub enum BRC20Operation {
  Predeploy(Predeploy),
  Deploy {
    deploy: Deploy,
    parent: Option<InscriptionId>,
  },
  Mint {
    op: Mint,
    parent: Option<InscriptionId>,
  },
  InscribeTransfer(Transfer),
  Transfer {
    ticker: BRC20Ticker,
    amount: u128,
  },
  InscribeProgDeploy {
    deploy: ProgDeploy,
    inscription_byte_length: u64,
  },
  ProgDeploy {
    data: Option<String>,
    base64_data: Option<String>,
    inscription_byte_length: u64,
  },
  InscribeProgCall {
    call: ProgCall,
    inscription_byte_length: u64,
  },
  ProgCall {
    contract_address: Option<String>,
    contract_inscription_id: Option<String>,
    data: Option<String>,
    base64_data: Option<String>,
    inscription_byte_length: u64,
  },
  InscribeProgTransact {
    transact: ProgTransact,
    inscription_byte_length: u64,
  },
  ProgTransact {
    data: Option<String>,
    base64_data: Option<String>,
    inscription_byte_length: u64,
  },
  InscribeWithdraw(Withdraw),
  Withdraw {
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
      match self.inscription.extract_brc20_operation() {
        Ok(RawOperation::Predeploy(predeploy)) => {
          if height < HardForks::predeploy_activation_height(&chain) {
            log::debug!(
              "Pre-deploy feature is not activated at height: {} for inscription: {}",
              height,
              self.inscription_id
            );
            return None;
          }
          Some(BRC20Operation::Predeploy(predeploy))
        }
        Ok(RawOperation::Deploy(mut deploy)) => {
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
          } else if deploy.tick.len() == PREDEPLOYED_TICKER_LENGTH {
            if height < HardForks::six_byte_deploy_activation_height(&chain) {
              log::debug!(
                "Pre-deployed 6-byte tickers are not activated at height: {} for inscription: {} with ticker length: {}",
                height,
                self.inscription_id,
                PREDEPLOYED_TICKER_LENGTH
              );
              return None;
            }
          } else {
            deploy.self_mint = None;
          }
          Some(BRC20Operation::Deploy {
            deploy,
            parent: self.parents.first().cloned(),
          })
        }
        Ok(RawOperation::Mint(mint)) => Some(BRC20Operation::Mint {
          op: mint,
          parent: self.parents.first().cloned(),
        }),
        Ok(RawOperation::Transfer(transfer)) => Some(BRC20Operation::InscribeTransfer(transfer)),
        Ok(RawOperation::ProgDeploy {
          deploy,
          inscription_byte_length,
        }) => {
          if height < HardForks::brc20_prog_activation_height(&chain) {
            log::debug!(
              "BRC20 Prog feature is not activated at height: {} for inscription: {}",
              height,
              self.inscription_id
            );
            return None;
          }
          Some(BRC20Operation::InscribeProgDeploy {
            deploy,
            inscription_byte_length,
          })
        }
        Ok(RawOperation::ProgCall {
          call,
          inscription_byte_length,
        }) => {
          if height < HardForks::brc20_prog_activation_height(&chain) {
            log::debug!(
              "BRC20 Prog feature is not activated at height: {} for inscription: {}",
              height,
              self.inscription_id
            );
            return None;
          }
          Some(BRC20Operation::InscribeProgCall {
            call,
            inscription_byte_length,
          })
        }
        Ok(RawOperation::ProgTransact {
          transact,
          inscription_byte_length,
        }) => {
          if height < HardForks::brc20_prog_activation_height(&chain) {
            log::debug!(
              "BRC20 Prog feature is not activated at height: {} for inscription: {}",
              height,
              self.inscription_id
            );
            return None;
          }
          Some(BRC20Operation::InscribeProgTransact {
            transact,
            inscription_byte_length,
          })
        }
        Ok(RawOperation::Withdraw(withdraw)) => {
          if height < HardForks::brc20_prog_activation_height(&chain) {
            log::debug!(
              "BRC20 Prog feature is not activated at height: {} for inscription: {}",
              height,
              self.inscription_id
            );
            return None;
          }
          Some(BRC20Operation::InscribeWithdraw(withdraw))
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
      if let Some(transfer_asset) = context.load_brc20_transferring_asset(self.old_satpoint)? {
        // Since a single old_satpoint may correspond to multiple inscriptions,
        // we need to verify whether the current inscription_id matches the asset's inscription_id.
        // Only if they match can it be considered a valid BRC20 transfer message.
        if self.inscription_id != transfer_asset.inscription_id {
          return Ok(None);
        }
        context.remove_brc20_transferring_asset(self.old_satpoint)?;
        return Ok(Some(BRC20Operation::Transfer {
          ticker: transfer_asset.ticker,
          amount: transfer_asset.amount,
        }));
      } else if let Some(prog_deploy_asset) =
        context.pop_brc20_prog_deploy_asset(self.old_satpoint)?
      {
        // Asset found, proceed with extraction.
        if self.inscription_id != prog_deploy_asset.inscription_id {
          return Ok(None);
        }

        return Ok(Some(BRC20Operation::ProgDeploy {
          data: prog_deploy_asset.data,
          base64_data: prog_deploy_asset.base64_data,
          inscription_byte_length: prog_deploy_asset.inscription_byte_length,
        }));
      } else if let Some(prog_call_asset) = context.pop_brc20_prog_call_asset(self.old_satpoint)? {
        if self.inscription_id != prog_call_asset.inscription_id {
          return Ok(None);
        }

        return Ok(Some(BRC20Operation::ProgCall {
          contract_address: prog_call_asset.contract_address,
          contract_inscription_id: prog_call_asset.contract_inscription_id,
          data: prog_call_asset.data,
          base64_data: prog_call_asset.base64_data,
          inscription_byte_length: prog_call_asset.inscription_byte_length,
        }));
      } else if let Some(prog_transact_asset) =
        context.pop_brc20_prog_transact_asset(self.old_satpoint)?
      {
        if self.inscription_id != prog_transact_asset.inscription_id {
          return Ok(None);
        }

        return Ok(Some(BRC20Operation::ProgTransact {
          data: prog_transact_asset.data,
          base64_data: prog_transact_asset.base64_data,
          inscription_byte_length: prog_transact_asset.inscription_byte_length,
        }));
      } else if let Some(withdraw_asset) =
        context.pop_brc20_prog_withdraw_asset(self.old_satpoint)?
      {
        if self.inscription_id != withdraw_asset.inscription_id {
          return Ok(None);
        }
        return Ok(Some(BRC20Operation::Withdraw {
          ticker: withdraw_asset.ticker,
          amount: withdraw_asset.amount,
        }));
      }
    }
    Ok(None)
  }
}
