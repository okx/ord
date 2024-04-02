use {
  crate::{Inscription, InscriptionId, SatPoint},
  bitcoin::Txid,
  serde::{Deserialize, Serialize},
};

// collect the inscription operation.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct InscriptionOp {
  pub txid: Txid,
  pub action: Action,
  pub sequence_number: u32,
  pub inscription_number: Option<i32>,
  pub inscription_id: InscriptionId,
  pub old_satpoint: SatPoint,
  pub new_satpoint: Option<SatPoint>,
}

// the act of marking an inscription.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum Action {
  New {
    cursed: bool,
    unbound: bool,
    #[serde(skip)]
    inscription: Inscription,
    #[serde(skip)]
    vindicated: bool,
    #[serde(skip)]
    parent: Option<InscriptionId>,
  },
  Transfer,
}
