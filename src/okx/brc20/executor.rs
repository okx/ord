use super::{
  entry::{BRC20Balance, BRC20Receipt, BRC20TickerInfo},
  error::BRC20Error,
  event::{BRC20Event, BRC20OpType, DeployEvent, InscribeTransferEvent, MintEvent, TransferEvent},
  *,
};

mod deploy;
mod inscribe_transfer;
mod mint;
mod transfer;

/// Represents a message used for executing BRC20 operations.
pub(crate) struct BRC20ExecutionMessage {
  txid: Txid,
  inscription_id: InscriptionId,
  sequence_number: u32,
  inscription_number: i32,
  old_satpoint: SatPoint,
  new_satpoint: SatPoint,
  sender: UtxoAddress,
  receiver: Option<UtxoAddress>, // no address, if unbound
  operation: BRC20Operation,
}

impl BRC20ExecutionMessage {
  pub(crate) fn new_from_bundle_message(
    value: &BundleMessage,
    context: &mut TableContext,
  ) -> Result<Option<Self>> {
    let build_message = |operation| {
      Ok(Some(Self {
        txid: value.txid,
        inscription_id: value.inscription_id,
        sequence_number: value.sequence_number,
        inscription_number: value.inscription_number,
        old_satpoint: value.old_satpoint,
        new_satpoint: value.new_satpoint,
        sender: value.sender.clone(),
        receiver: value.receiver.clone(),
        operation,
      }))
    };

    match &value.inscription_action {
      InscriptionAction::Created { sub_type, .. } => {
        if let Some(SubType::BRC20(brc20_operation)) = sub_type {
          build_message(brc20_operation.clone())
        } else {
          Ok(None)
        }
      }
      InscriptionAction::Transferred => match Option::<TransferredInscription>::from(value) {
        Some(transferred_inscription) => {
          match transferred_inscription.extract_and_validate_transfer(context) {
            Ok(Some(brc20_operation)) => build_message(brc20_operation),
            Ok(None) => Ok(None),
            Err(err) => Err(err),
          }
        }
        _ => unreachable!(),
      },
    }
  }
}

impl BRC20ExecutionMessage {
  pub fn execute(
    self,
    context: &mut TableContext,
    height: u32,
    blocktime: u32,
  ) -> Result<BRC20Receipt> {
    let result = match &self.operation {
      BRC20Operation::Deploy(..) => self.execute_deploy(context, height, blocktime),
      BRC20Operation::Mint { .. } => self.execute_mint(context, height),
      BRC20Operation::InscribeTransfer(_) => self.execute_inscribe_transfer(context),
      BRC20Operation::Transfer { .. } => self.execute_transfer(context),
    };

    match result {
      Ok(receipt) => Ok(receipt),
      Err(ExecutionError::ExecutionFailed(e)) => Ok(BRC20Receipt {
        // Handle specific execution failure
        inscription_id: self.inscription_id,
        sequence_number: self.sequence_number,
        inscription_number: self.inscription_number,
        old_satpoint: self.old_satpoint,
        new_satpoint: self.new_satpoint,
        op_type: BRC20OpType::from(&self.operation),
        sender: self.sender.clone(),
        receiver: self.receiver.unwrap_or(self.sender),
        result: Err(e),
      }),
      Err(e) => {
        log::error!(
          "BRC20 execution failed: txid = {}, inscription_id = {}, error = {:?}",
          self.txid,
          self.inscription_id,
          e
        );
        Err(e.into())
      }
    }
  }
}

#[derive(Debug, thiserror::Error)]
pub(super) enum ExecutionError {
  #[error("Storage error: {0}")]
  Storage(#[from] redb::StorageError),
  #[error("Execution failed: {0}")]
  ExecutionFailed(#[from] BRC20Error),
  #[error("Unexpected error: {0}")]
  Unexpected(#[from] Error),
}
