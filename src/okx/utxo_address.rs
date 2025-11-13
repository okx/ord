use super::*;
use bitcoin::{
  address::{Address, NetworkUnchecked},
  Script, ScriptHash,
};

pub(crate) type UtxoAddressRef = UtxoAddressInner;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum UtxoAddressInner {
  Address(Address<NetworkUnchecked>),
  ScriptHash {
    op_return: bool,
    script: ScriptBuf,
    script_hash: ScriptHash,
  },
}

#[repr(transparent)]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UtxoAddress(UtxoAddressInner);

const BRC20_PROG_OP_RETURN: &[u8; 9] = &[66u8, 82u8, 67u8, 50u8, 48u8, 80u8, 82u8, 79u8, 71u8]; // "BRC20PROG"
pub static BRC20_PROG_OP_RETURN_SCRIPT_HASH: Lazy<ScriptHash> =
  Lazy::new(|| ScriptBuf::new_op_return(BRC20_PROG_OP_RETURN).script_hash());
pub static BRC20_PROG_OP_RETURN_UTXO_ADDRESS: Lazy<UtxoAddress> = Lazy::new(|| {
  UtxoAddress(UtxoAddressInner::ScriptHash {
    op_return: true,
    script: ScriptBuf::new_op_return(BRC20_PROG_OP_RETURN),
    script_hash: BRC20_PROG_OP_RETURN_SCRIPT_HASH.clone(),
  })
});

impl UtxoAddress {
  pub fn from_script(script: &Script, chain: &Chain) -> Self {
    Self(
      chain
        .address_from_script(script)
        .map(|address| UtxoAddressInner::Address(address.as_unchecked().clone()))
        .unwrap_or(UtxoAddressInner::ScriptHash {
          script_hash: script.script_hash(),
          script: script.to_owned(),
          op_return: script.is_op_return(),
        }),
    )
  }

  pub fn to_script_bytes(&self) -> Vec<u8> {
    match &self.0 {
      UtxoAddressInner::Address(address) => address
        .clone()
        .assume_checked()
        .script_pubkey()
        .as_bytes()
        .to_vec(),
      UtxoAddressInner::ScriptHash { script, .. } => script.as_bytes().to_vec(),
    }
  }

  pub fn from_str(address: &str, network: Network) -> Result<Self> {
    Ok(
      Address::from_str(address)?
        .require_network(network)
        .map(|address| Self(UtxoAddressInner::Address(address.as_unchecked().clone())))?,
    )
  }

  pub fn op_return(&self) -> bool {
    match &self.0 {
      UtxoAddressInner::Address(_) => false,
      UtxoAddressInner::ScriptHash { op_return, .. } => *op_return,
    }
  }

  pub fn op_return_prog(&self) -> bool {
    match &self.0 {
      UtxoAddressInner::ScriptHash {
        script_hash,
        script: _,
        op_return,
      } if *op_return && script_hash == &*BRC20_PROG_OP_RETURN_SCRIPT_HASH => true,
      _ => false,
    }
  }

  pub(crate) fn as_ref(&self) -> &UtxoAddressRef {
    &self.0
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use bitcoin::{Address, Script};
  use std::str::FromStr;

  #[test]
  fn test_from_script_with_valid_address() {
    let address = Address::from_str("bc1qhvd6suvqzjcu9pxjhrwhtrlj85ny3n2mqql5w4").unwrap();
    assert_eq!(
      UtxoAddress::from_script(
        address.clone().assume_checked().script_pubkey().as_script(),
        &Chain::Mainnet
      ),
      UtxoAddress(UtxoAddressInner::Address(address))
    );
  }

  #[test]
  fn test_from_script_with_non_op_return_script() {
    let hex_script = hex::decode(
      "0014017fed86bba5f31f955f8b316c7fb9bd45cb6cbc00000000000000000000000000000000000000",
    )
    .expect("Failed to decode hex script");
    let script = Script::from_bytes(hex_script.as_slice());

    assert_eq!(
      UtxoAddress::from_script(script, &Chain::Mainnet),
      UtxoAddress(UtxoAddressInner::ScriptHash {
        script: script.to_owned(),
        script_hash: ScriptHash::from_str("df65c8a338dce7900824e7bd18c336656ca19e57")
          .expect("Failed to parse script hash"),
        op_return: false,
      })
    );
  }

  #[test]
  fn test_from_script_with_op_return_script() {
    let hex_script =
      hex::decode("6a0b68656c6c6f20776f726c64").expect("Failed to decode hex script");
    let script = Script::from_bytes(hex_script.as_slice());

    assert_eq!(
      UtxoAddress::from_script(script, &Chain::Mainnet),
      UtxoAddress(UtxoAddressInner::ScriptHash {
        script: script.to_owned(),
        script_hash: ScriptHash::from_str("70c382a01444e96a1fd2eeb9041bdef603e0c410")
          .expect("Failed to parse script hash"),
        op_return: true,
      })
    );
  }

  #[test]
  fn test_serialize_deserialize_with_address() {
    let descriptor = UtxoAddress(UtxoAddressInner::Address(
      Address::from_str("bc1qhvd6suvqzjcu9pxjhrwhtrlj85ny3n2mqql5w4").unwrap(),
    ));

    let serialized = bincode::serialize(&descriptor).unwrap();
    assert_eq!(
      serialized,
      vec![
        0, 0, 0, 0, 42, 0, 0, 0, 0, 0, 0, 0, 98, 99, 49, 113, 104, 118, 100, 54, 115, 117, 118,
        113, 122, 106, 99, 117, 57, 112, 120, 106, 104, 114, 119, 104, 116, 114, 108, 106, 56, 53,
        110, 121, 51, 110, 50, 109, 113, 113, 108, 53, 119, 52
      ]
    );
    let deserialized: UtxoAddress = bincode::deserialize(&serialized).unwrap();

    assert_eq!(deserialized, descriptor);
  }

  #[test]
  fn test_serialize_deserialize_with_non_op_return_script() {
    let descriptor = UtxoAddress(UtxoAddressInner::ScriptHash {
      script: ScriptBuf::from_hex("0014df65c8a338dce7900824e7bd18c336656ca19e57").unwrap(),
      script_hash: ScriptHash::from_str("df65c8a338dce7900824e7bd18c336656ca19e57").unwrap(),
      op_return: false,
    });

    let serialized = bincode::serialize(&descriptor).unwrap();
    assert_eq!(
      serialized,
      vec![
        1, 0, 0, 0, 0, 20, 0, 0, 0, 0, 0, 0, 0, 223, 101, 200, 163, 56, 220, 231, 144, 8, 36, 231,
        189, 24, 195, 54, 101, 108, 161, 158, 87
      ]
    );
    let deserialized: UtxoAddress = bincode::deserialize(&serialized).unwrap();

    assert_eq!(deserialized, descriptor);
  }

  #[test]
  fn test_serialize_deserialize_with_op_return_script() {
    let descriptor = UtxoAddress(UtxoAddressInner::ScriptHash {
      script: ScriptBuf::from_hex("6a0b68656c6c6f20776f726c64").unwrap(),
      script_hash: ScriptHash::from_str("70c382a01444e96a1fd2eeb9041bdef603e0c410").unwrap(),
      op_return: true,
    });

    let serialized = bincode::serialize(&descriptor).unwrap();
    assert_eq!(
      serialized,
      vec![
        1, 0, 0, 0, 1, 20, 0, 0, 0, 0, 0, 0, 0, 112, 195, 130, 160, 20, 68, 233, 106, 31, 210, 238,
        185, 4, 27, 222, 246, 3, 224, 196, 16
      ]
    );
    let deserialized: UtxoAddress = bincode::deserialize(&serialized).unwrap();

    assert_eq!(deserialized, descriptor);
  }

  #[test]
  fn test_op_return_script_hash() {
    let script_hash_str = "ba82555f9c38d31ff5cd562b886c8f34b6559dbc";
    let script_hash = ScriptHash::from_str(script_hash_str).unwrap();
    let utxo_address = UtxoAddress(UtxoAddressInner::ScriptHash {
      script: ScriptBuf::new_op_return(&[1, 2, 3]),
      script_hash,
      op_return: true,
    });
    assert!(utxo_address.op_return());
    assert!(!utxo_address.op_return_prog());
  }
}
