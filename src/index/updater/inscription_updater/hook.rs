use super::*;
use crate::okx::bitmap::BitmapMessageExtractor;
use crate::okx::brc20::{BRC20Message, BRC20MessageExtractor};
use crate::okx::{Action, InscriptionMessage, Message, UtxoAddress};

#[derive(Debug, Clone)]
pub enum InscriptionAction {
  Created { charms: u16 },
  Transferred,
}

#[derive(Debug, Clone)]
pub struct BundleMessage {
  pub txid: Txid,
  pub offset: u64,
  pub inscription_id: InscriptionId,
  pub sequence_number: u32,
  pub inscription_number: i32,
  pub old_satpoint: SatPoint,
  pub new_satpoint: SatPoint,
  pub sender: UtxoAddress,
  pub receiver: Option<UtxoAddress>, // If unbound, no receiver address.
  pub inscription_action: InscriptionAction,
  pub internal_message: Option<Message>,
}

impl BundleMessage {
  /// Determines whether this inscription needs to be tracked.
  /// Returns `false` when the message is a BRC20 Mint or Transfer, otherwise `true`.
  pub fn should_track(&self) -> bool {
    match &self.internal_message {
      Some(Message::BRC20(BRC20Message::Transfer { .. }))
      | Some(Message::BRC20(BRC20Message::Mint { .. })) => false,
      _ => true,
    }
  }
}

pub(super) struct InscriptionHook<'a, 'b, 'tx> {
  pub(super) flotsam: &'a Flotsam,
  pub(super) new_satpoint: SatPoint,
  pub(super) new_script_buf: Option<&'a ScriptBuf>,
  pub(super) sequence_number: u32,
  pub(super) inscription_number: i32,
  pub(super) charms: Option<u16>, // Charms are only present during creation.
  pub(super) updater: &'a mut InscriptionUpdater<'b, 'tx>,
  pub(super) index: &'a Index,
}

impl InscriptionHook<'_, '_, '_> {
  pub(super) fn handle_inscription(self) -> Result<bool> {
    let chain = self.index.settings.chain();
    let sender = UtxoAddress::from_script(
      Script::from_bytes(self.flotsam.input_script_buf.as_slice()),
      &chain,
    );
    let receiver = self
      .new_script_buf
      .map(|script| UtxoAddress::from_script(script, &chain));

    let inscription_message = InscriptionMessage {
      inscription_id: self.flotsam.inscription_id,
      sequence_number: self.sequence_number,
      inscription_number: self.inscription_number,
      old_satpoint: self.flotsam.old_satpoint,
      new_satpoint: self.new_satpoint,
      sender: sender.clone(),
      receiver: receiver.clone(),
      action: match &self.flotsam.origin {
        Origin::New {
          inscription,
          parents,
          pre_jubilant_curse_reason,
          ..
        } => Action::Created {
          inscription: inscription.clone(),
          parents: parents.clone(),
          pre_jubilant_curse_reason: pre_jubilant_curse_reason.clone(),
          charms: self.charms.unwrap(),
        },
        Origin::Old { .. } => Action::Transferred,
      },
    };

    let internal_message = if self.index.index_brc20 {
      inscription_message
        .extract_brc20_message(
          self.updater.height,
          chain,
          self.updater.brc20_satpoint_to_transfer_assets,
          self.updater.brc20_address_ticker_to_transfer_assets,
        )?
        .map(Message::BRC20)
    } else if self.index.index_bitmap {
      inscription_message
        .extract_bitmap_message()?
        .map(Message::BITMAP)
    } else {
      None
    };

    if internal_message.is_none() && !self.index.save_inscription_receipts {
      return Ok(true);
    }

    let bundle_message = BundleMessage {
      txid: self.flotsam.txid,
      offset: self.flotsam.offset,
      inscription_id: self.flotsam.inscription_id,
      sequence_number: self.sequence_number,
      inscription_number: self.inscription_number,
      old_satpoint: self.flotsam.old_satpoint,
      new_satpoint: self.new_satpoint,
      sender,
      receiver,
      inscription_action: match self.flotsam.origin {
        Origin::New { .. } => InscriptionAction::Created {
          charms: self.charms.unwrap(),
        },
        Origin::Old { .. } => InscriptionAction::Transferred,
      },
      internal_message,
    };

    let should_track = bundle_message.should_track();
    self
      .updater
      .block_bundle_messages
      .entry(bundle_message.txid)
      .or_insert_with(Vec::new)
      .push(bundle_message);

    Ok(should_track)
  }
}
