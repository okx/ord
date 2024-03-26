use self::okx::datastore::ScriptKey;
use super::*;
use bitcoin::ScriptHash;

pub(crate) fn parse_and_validate_script_key_with_chain(
  key: &str,
  chain: Chain,
) -> Result<ScriptKey> {
  if let Ok(address) = Address::from_str(key) {
    match address.clone().require_network(chain.network()) {
      Ok(_) => Ok(ScriptKey::Address(address)),
      Err(_) => Err(anyhow!("invalid chain: {} for address: {}", chain, key)),
    }
  } else if let Ok(script_hash) = ScriptHash::from_str(key) {
    Ok(ScriptKey::ScriptHash {
      script_hash,
      is_op_return: false,
    })
  } else {
    Err(anyhow!("invalid script key: {}", key))
  }
}
