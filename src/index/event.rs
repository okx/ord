use super::*;
use crate::okx::UtxoAddress;

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
  InscriptionCreated {
    block_height: u32,
    charms: u16,
    inscription_id: InscriptionId,
    location: Option<SatPoint>,
    parent_inscription_ids: Vec<InscriptionId>,
    sequence_number: u32,
  },
  InscriptionTransferred {
    block_height: u32,
    inscription_id: InscriptionId,
    new_location: SatPoint,
    old_location: SatPoint,
    sequence_number: u32,
  },
  RuneBurned {
    amount: u128,
    block_height: u32,
    rune_id: RuneId,
    txid: Txid,
  },
  RuneEtched {
    block_height: u32,
    rune_id: RuneId,
    txid: Txid,
  },
  RuneMinted {
    amount: u128,
    block_height: u32,
    rune_id: RuneId,
    txid: Txid,
  },
  RuneTransferred {
    amount: u128,
    block_height: u32,
    outpoint: OutPoint,
    rune_id: RuneId,
    txid: Txid,
  },
}

#[derive(Debug)]
pub enum Action {
  Created {
    inscription: Inscription,
    parents: Vec<InscriptionId>,
    pre_jubilant_curse_reason: Option<Curse>,
    charms: u16,
  },
  Transferred,
}

#[derive(Debug)]
pub struct OkxInscriptionEvent {
  pub txid: Txid,
  pub offset: u64,
  pub inscription_id: InscriptionId,
  pub sequence_number: u32,
  pub inscription_number: i32,
  pub old_satpoint: SatPoint,
  pub new_satpoint: SatPoint,
  pub sender: UtxoAddress,
  pub receiver: Option<UtxoAddress>,
  pub action: Action,
}
