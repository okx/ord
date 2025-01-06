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
  offset: u64,
  inscription_id: InscriptionId,
  sequence_number: u32,
  inscription_number: i32,
  old_satpoint: SatPoint,
  new_satpoint: SatPoint,
  sender: UtxoAddress,
  receiver: Option<UtxoAddress>, // no address, if unbound
  message: BRC20Message,
}

impl From<&BundleMessage> for Option<BRC20ExecutionMessage> {
  fn from(value: &BundleMessage) -> Self {
    match &value.sub_message {
      Some(SubMessage::BRC20(message)) => Some(BRC20ExecutionMessage {
        txid: value.txid,
        offset: value.offset,
        inscription_id: value.inscription_id,
        sequence_number: value.sequence_number,
        inscription_number: value.inscription_number,
        old_satpoint: value.old_satpoint,
        new_satpoint: value.new_satpoint,
        sender: value.sender.clone(),
        receiver: value.receiver.clone(),
        message: message.clone(),
      }),
      _ => None,
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
    let result = match &self.message {
      BRC20Message::Deploy(..) => self.execute_deploy(context, height, blocktime),
      BRC20Message::Mint { .. } => self.execute_mint(context, height),
      BRC20Message::InscribeTransfer(_) => self.execute_inscribe_transfer(context),
      BRC20Message::Transfer { .. } => self.execute_transfer(context),
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
        op_type: BRC20OpType::from(&self.message),
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
