use self::okx::datastore::ScriptKey;
use super::*;
use bitcoin::ScriptHash;

pub(crate) fn parse_and_validate_script_key_network(
  key: &str,
  network: Network,
) -> Result<ScriptKey> {
  if let Ok(address) = Address::from_str(key) {
    match address.clone().require_network(network) {
      Ok(_) => Ok(ScriptKey::Address(address)),
      Err(_) => Err(anyhow!("invalid network: {} for address: {}", network, key)),
    }
  } else if let Ok(script_hash) = ScriptHash::from_str(key) {
    Ok(ScriptKey::ScriptHash(script_hash))
  } else {
    Err(anyhow!("invalid script key: {}", key))
  }
}
