use super::*;
use bitcoin::ScriptHash;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ApiUtxoAddress {
  Address(Address<NetworkUnchecked>),
  NonStandard(ScriptHash),
}

impl From<UtxoAddress> for ApiUtxoAddress {
  fn from(utxo_address: UtxoAddress) -> Self {
    match utxo_address.as_ref() {
      UtxoAddressRef::Address(address) => ApiUtxoAddress::Address(address.clone()),
      UtxoAddressRef::ScriptHash { script_hash, .. } => {
        ApiUtxoAddress::NonStandard(script_hash.clone())
      }
    }
  }
}
#[cfg(test)]
mod tests {
  use super::*;

  fn test_serialize_utxo_address(script: impl Into<ScriptBuf>, expected_json: &str) {
    let script_pubkey: ApiUtxoAddress =
      UtxoAddress::from_script(&script.into(), &Chain::Mainnet).into();
    let serialized = serde_json::to_string(&script_pubkey).unwrap();
    assert_eq!(serialized, expected_json);
  }
  #[test]
  fn serialize_api_utxo_address() {
    let address = "bc1qhvd6suvqzjcu9pxjhrwhtrlj85ny3n2mqql5w4";
    test_serialize_utxo_address(
      Address::from_str(address)
        .unwrap()
        .assume_checked()
        .script_pubkey(),
      &format!(r#"{{"address":"{}"}}"#, address),
    );

    test_serialize_utxo_address(
      Script::from_bytes(
        hex::decode(
          "0014017fed86bba5f31f955f8b316c7fb9bd45cb6cbc00000000000000000000000000000000000000",
        )
        .unwrap()
        .as_slice(),
      ),
      r#"{"nonStandard":"df65c8a338dce7900824e7bd18c336656ca19e57"}"#,
    );
  }
}
