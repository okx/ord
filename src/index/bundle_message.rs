use super::*;
use crate::{
  index::event::{Action, OkxInscriptionEvent},
  okx::{
    bitmap::{BitmapDistrict, BitmapMessageExtractor},
    brc20::{BRC20CreationOperationExtractor, BRC20Operation, CreatedInscription},
    btc_domain::{BTCDomainExtractor, BtcDomain},
    UtxoAddress,
  },
};
use {
  bitcoin::{
    secp256k1::{XOnlyPublicKey},
    script::{ScriptBuf},
    key::{TweakedPublicKey},
  },
};

#[derive(Debug, Clone)]
pub enum SubType {
  BRC20(BRC20Operation),
  Bitmap(BitmapDistrict),
  BtcDomain(BtcDomain),
}

#[derive(Debug, Clone)]
pub enum InscriptionAction {
  Created {
    charms: u16,
    sub_type: Option<SubType>,
    signer: Option<UtxoAddress>,
  },
  Transferred,
}

#[derive(Debug, Clone)]
pub struct BundleMessage {
  pub txid: Txid,
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
        inscription_id: event.inscription_id,
        sequence_number: event.sequence_number,
        inscription_number: event.inscription_number,
        old_satpoint: event.old_satpoint,
        new_satpoint: event.new_satpoint,
        sender: event.sender,
        receiver: event.receiver,
        inscription_action: match event.action {
          Action::Created { charms, tapscript_pk, .. } => {
            let address_type = tapscript_pk[34];
            let signer = if address_type > 0 {
              let script = get_pk_script_by_pubkey_and_type(&tapscript_pk[1..33], address_type);
              Some(UtxoAddress::from_script(script.as_script(), &index.settings.chain()))
            } else {
              None
            };
            InscriptionAction::Created { charms, sub_type, signer, }
          },
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

  if index.index_bitmap && !inscription_event.inscription_number.is_negative() {
    if let Action::Created { inscription, .. } = &inscription_event.action {
      if let Some(bitmap_msg) = inscription.extract_bitmap_message() {
        return Ok(Some(SubType::Bitmap(bitmap_msg)));
      }
    }
  }

  if index.index_btc_domain && !inscription_event.inscription_number.is_negative() {
    if let Action::Created { inscription, .. } = &inscription_event.action {
      if let Some(btc_domain) = inscription.extract_btc_domain() {
        return Ok(Some(SubType::BtcDomain(btc_domain)));
      }
    }
  }
  Ok(None)
}

/// Get a script pubkey based on the provided pubkey and address type
pub fn get_pk_script_by_pubkey_and_type(x_only_pubkey_bytes: &[u8], address_type: u8) -> ScriptBuf {
  const BRC20_PUBKEY_ADDRESS_P2TR_SCRIPT: u8 = 0x51;
  const BRC20_PUBKEY_ADDRESS_P2PKH_EVEN: u8 = 0x52;
  const BRC20_PUBKEY_ADDRESS_P2PKH_ODD: u8 = 0x53;
  const BRC20_PUBKEY_ADDRESS_P2WPKH: u8 = 0x54;
  const BRC20_PUBKEY_ADDRESS_P2TR_KEY: u8 = 0x55;
  const BRC20_PUBKEY_ADDRESS_P2SH_P2WPKH: u8 = 0x56;

  let x_only_pubkey = XOnlyPublicKey::from_slice(x_only_pubkey_bytes).unwrap();
  match address_type {
    BRC20_PUBKEY_ADDRESS_P2TR_SCRIPT => {
      let secp = bitcoin::secp256k1::Secp256k1::verification_only();
      ScriptBuf::new_p2tr(&secp, x_only_pubkey, None)
    },
    BRC20_PUBKEY_ADDRESS_P2PKH_EVEN | BRC20_PUBKEY_ADDRESS_P2PKH_ODD => {
      let parity = if address_type == BRC20_PUBKEY_ADDRESS_P2PKH_EVEN {
        bitcoin::secp256k1::Parity::Even
      } else {
        bitcoin::secp256k1::Parity::Odd
      };
      let pubkey = bitcoin::PublicKey::new(x_only_pubkey.public_key(parity));
      ScriptBuf::new_p2pkh(&pubkey.pubkey_hash())
    },
    BRC20_PUBKEY_ADDRESS_P2WPKH | BRC20_PUBKEY_ADDRESS_P2SH_P2WPKH => {
      let pubkey = bitcoin::PublicKey::new(x_only_pubkey.public_key(bitcoin::secp256k1::Parity::Even));
      let wpkh_script = ScriptBuf::new_p2wpkh(&pubkey.wpubkey_hash().unwrap());
      if address_type == BRC20_PUBKEY_ADDRESS_P2WPKH {
        wpkh_script
      } else {
        ScriptBuf::new_p2sh(&wpkh_script.script_hash())
      }
    },
    BRC20_PUBKEY_ADDRESS_P2TR_KEY => {
      ScriptBuf::new_p2tr_tweaked(TweakedPublicKey::dangerous_assume_tweaked(x_only_pubkey))
    },
    _ => ScriptBuf::new(),
  }
}
