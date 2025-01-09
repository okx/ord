use super::*;
use crate::{
  index::event::{Action, OkxInscriptionEvent},
  okx::{
    bitmap::{BitmapMessageExtractor, BitmapOperation},
    brc20::{BRC20CreationOperationExtractor, BRC20Operation, CreatedInscription},
    UtxoAddress,
  },
};

#[derive(Debug, Clone)]
pub enum SubType {
  BRC20(BRC20Operation),
  BITMAP(BitmapOperation),
}

#[derive(Debug, Clone)]
pub enum InscriptionAction {
  Created {
    charms: u16,
    sub_type: Option<SubType>,
  },
  Transferred,
}

#[derive(Debug, Clone)]
pub struct BundleMessage {
  pub txid: Txid,
  pub offset: u64, // Offset of the inscription in the transaction.
  pub inscription_id: InscriptionId,
  pub sequence_number: u32,
  pub inscription_number: i32,
  pub old_satpoint: SatPoint,
  pub new_satpoint: SatPoint,
  pub sender: UtxoAddress,
  pub receiver: Option<UtxoAddress>, // If unbound, no receiver address.
  pub inscription_action: InscriptionAction,
}

impl BundleMessage {
  /// Determines whether this inscription needs to be tracked.
  /// Returns `false` when the message is a BRC20 Mint or Transfer, otherwise `true`.
  pub fn should_track(&self, index: &Index) -> bool {
    if !index.disable_invalid_brc20_tracking {
      return true;
    }

    if let InscriptionAction::Created { sub_type, .. } = &self.inscription_action {
      if let Some(SubType::BRC20(operation)) = sub_type {
        return !matches!(
          operation,
          BRC20Operation::Mint { .. } | BRC20Operation::InscribeTransfer(_)
        );
      }
    }
    true
  }
}

impl BundleMessage {
  pub(in crate::index) fn from_okx_inscription_event(
    event: OkxInscriptionEvent,
    height: u32,
    index: &Index,
  ) -> Result<Option<Self>> {
    let sub_type = extract_sub_type(&event, height, index)?;

    if sub_type.is_some() || index.index_brc20 || index.save_inscription_receipts {
      Ok(Some(Self {
        txid: event.txid,
        offset: event.offset,
        inscription_id: event.inscription_id,
        sequence_number: event.sequence_number,
        inscription_number: event.inscription_number,
        old_satpoint: event.old_satpoint,
        new_satpoint: event.new_satpoint,
        sender: event.sender,
        receiver: event.receiver,
        inscription_action: match event.action {
          Action::Created { charms, .. } => InscriptionAction::Created { charms, sub_type },
          Action::Transferred => InscriptionAction::Transferred,
        },
      }))
    } else {
      Ok(None)
    }
  }
}

fn extract_sub_type(
  inscription_event: &OkxInscriptionEvent,
  block_height: u32,
  index: &Index,
) -> Result<Option<SubType>> {
  if index.index_brc20 {
    if let Some(inscription) = Option::<CreatedInscription>::from(inscription_event) {
      if let Some(brc20_op) =
        inscription.extract_and_validate_creation(block_height, index.settings.chain())
      {
        return Ok(Some(SubType::BRC20(brc20_op)));
      }
    }
  }

  if index.index_bitmap {
    if let Some(bitmap_msg) = inscription_event.extract_bitmap_message()? {
      return Ok(Some(SubType::BITMAP(bitmap_msg)));
    }
  }
  Ok(None)
}
