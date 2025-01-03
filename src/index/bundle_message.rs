use super::*;
use crate::index::event::{Action, OkxInscriptionEvent};
use crate::okx::bitmap::BitmapMessageExtractor;
use crate::okx::brc20::{BRC20Message, BRC20MessageExtractor};
use crate::okx::{SubMessage, UtxoAddress};

#[derive(Debug, Clone)]
pub enum InscriptionAction {
  Created { charms: u16 },
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
  pub sub_message: Option<SubMessage>,
}

impl BundleMessage {
  /// Determines whether this inscription needs to be tracked.
  /// Returns `false` when the message is a BRC20 Mint or Transfer, otherwise `true`.
  pub fn should_track(&self, index: &Index) -> bool {
    if index.disable_invalid_brc20_tracking {
      match &self.sub_message {
        Some(SubMessage::BRC20(BRC20Message::Transfer { .. }))
        | Some(SubMessage::BRC20(BRC20Message::Mint { .. })) => false,
        _ => true,
      }
    } else {
      true
    }
  }
}

impl BundleMessage {
  pub(in crate::index) fn from_okx_inscription_event<'tx>(
    event: OkxInscriptionEvent,
    height: u32,
    index: &Index,
    brc20_satpoint_to_transfer_assets: &mut Table<
      'tx,
      &'static SatPointValue,
      &'static BRC20TransferAssetValue,
    >,
    brc20_address_ticker_to_transfer_assets: &mut MultimapTable<
      'tx,
      &'static AddressTickerKeyValue,
      &'static SatPointValue,
    >,
  ) -> Result<Option<BundleMessage>> {
    let sub_message = if index.index_brc20 {
      event
        .extract_brc20_message(
          height,
          index.settings.chain(),
          brc20_satpoint_to_transfer_assets,
          brc20_address_ticker_to_transfer_assets,
        )?
        .map(SubMessage::BRC20)
    } else if index.index_bitmap {
      event.extract_bitmap_message()?.map(SubMessage::BITMAP)
    } else {
      None
    };

    if sub_message.is_some() || index.save_inscription_receipts {
      Ok(Some(BundleMessage {
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
          Action::Created { charms, .. } => InscriptionAction::Created { charms },
          Action::Transferred => InscriptionAction::Transferred,
        },
        sub_message,
      }))
    } else {
      Ok(None)
    }
  }
}
