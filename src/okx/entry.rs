use crate::index::InscriptionAction;
use super::*;
use crate::okx::brc20::entry::BRC20Balance;
use crate::okx::composite_key::{AddressEndpoint, AddressTickerKey};

pub(crate) trait DynamicEntry {
  type Value: AsRef<[u8]> + ?Sized;

  fn load(value: &Self::Value) -> Self;
  fn store(&self) -> Box<Self::Value>;
}

#[macro_export]
macro_rules! impl_bincode_dynamic_entry {
  ($type:ty, $value_type:ty) => {
    impl DynamicEntry for $type {
      type Value = $value_type;

      fn load(value: &Self::Value) -> Self {
        bincode::deserialize(value).unwrap()
      }

      fn store(&self) -> Box<Self::Value> {
        bincode::serialize(self).unwrap().into()
      }
    }
  };
}

pub(crate) type UtxoAddressValue = [u8];
impl_bincode_dynamic_entry!(UtxoAddress, UtxoAddressValue);

pub type AddressTickerKeyValue = [u8];
impl_bincode_dynamic_entry!(AddressTickerKey, AddressTickerKeyValue);
impl_bincode_dynamic_entry!(AddressEndpoint, AddressTickerKeyValue);

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum Action {
  Created { charms: u16 },
  Transferred,
}

pub(crate) type InscriptionReceiptsValue = [u8];
impl_bincode_dynamic_entry!(Vec<InscriptionReceipt>, InscriptionReceiptsValue);

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct InscriptionReceipt {
  pub sequence_number: u32,
  pub inscription_id: InscriptionId,
  pub old_satpoint: SatPoint,
  pub new_satpoint: Option<SatPoint>,
  pub sender: UtxoAddress,
  pub receiver: Option<UtxoAddress>,
  pub action: Action,
}

impl From<BundleMessage> for InscriptionReceipt {
  fn from(value: BundleMessage) -> Self {
    Self {
      sequence_number: value.sequence_number,
      inscription_id: value.inscription_id,
      old_satpoint: value.old_satpoint,
      new_satpoint: Some(value.new_satpoint),
      sender: value.sender,
      receiver: value.receiver,
      action: match value.inscription_action {
        InscriptionAction::Created { charms, .. } => Action::Created { charms },
        InscriptionAction::Transferred => Action::Transferred,
      },
    }
  }
}
