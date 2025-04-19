use {
    bitcoin::{
        secp256k1::{XOnlyPublicKey},
        script::{ScriptBuf},
        key::{TweakedPublicKey},
    },
};

const BRC20_PUBKEY_ADDRESS_P2TR_SCRIPT: u8 = 0x51;
const BRC20_PUBKEY_ADDRESS_P2WPKH_EVEN: u8 = 0x52;
const BRC20_PUBKEY_ADDRESS_P2WPKH_ODD: u8 = 0x53;
const BRC20_PUBKEY_ADDRESS_P2PKH_EVEN: u8 = 0x54;
const BRC20_PUBKEY_ADDRESS_P2PKH_ODD: u8 = 0x55;
const BRC20_PUBKEY_ADDRESS_P2SH_P2WPKH_EVEN: u8 = 0x56;
const BRC20_PUBKEY_ADDRESS_P2SH_P2WPKH_ODD: u8 = 0x57;
const BRC20_PUBKEY_ADDRESS_P2TR_KEY: u8 = 0x58;

/// Get a script pubkey based on the provided pubkey and address type
pub fn get_pk_script_by_pubkey_and_type(x_only_pubkey_bytes: &[u8], address_type: u8) -> ScriptBuf {
  let x_only_pubkey = XOnlyPublicKey::from_slice(x_only_pubkey_bytes).unwrap();
  let parity = if address_type == BRC20_PUBKEY_ADDRESS_P2PKH_EVEN || address_type == BRC20_PUBKEY_ADDRESS_P2WPKH_EVEN || address_type == BRC20_PUBKEY_ADDRESS_P2SH_P2WPKH_EVEN {
    bitcoin::secp256k1::Parity::Even
  } else {
    bitcoin::secp256k1::Parity::Odd
  };
  let pubkey = bitcoin::PublicKey::new(x_only_pubkey.public_key(parity));
  match address_type {
    BRC20_PUBKEY_ADDRESS_P2TR_SCRIPT => {
      let secp = bitcoin::secp256k1::Secp256k1::verification_only();
      ScriptBuf::new_p2tr(&secp, x_only_pubkey, None)
    },
    BRC20_PUBKEY_ADDRESS_P2WPKH_EVEN | BRC20_PUBKEY_ADDRESS_P2WPKH_ODD => {
      ScriptBuf::new_p2wpkh(&pubkey.wpubkey_hash().unwrap())
    },
    BRC20_PUBKEY_ADDRESS_P2PKH_EVEN | BRC20_PUBKEY_ADDRESS_P2PKH_ODD => {
      ScriptBuf::new_p2pkh(&pubkey.pubkey_hash())
    },
    BRC20_PUBKEY_ADDRESS_P2SH_P2WPKH_EVEN | BRC20_PUBKEY_ADDRESS_P2SH_P2WPKH_ODD => {
      let wpkh_script = ScriptBuf::new_p2wpkh(&pubkey.wpubkey_hash().unwrap());
      ScriptBuf::new_p2sh(&wpkh_script.script_hash())
    },
    BRC20_PUBKEY_ADDRESS_P2TR_KEY => {
      ScriptBuf::new_p2tr_tweaked(TweakedPublicKey::dangerous_assume_tweaked(x_only_pubkey))
    },
    _ => ScriptBuf::new(),
  }
}

#[cfg(test)]
mod tests {
    use super::{
        get_pk_script_by_pubkey_and_type,
        BRC20_PUBKEY_ADDRESS_P2TR_SCRIPT,
        BRC20_PUBKEY_ADDRESS_P2WPKH_EVEN,
        BRC20_PUBKEY_ADDRESS_P2WPKH_ODD,
        BRC20_PUBKEY_ADDRESS_P2PKH_EVEN,
        BRC20_PUBKEY_ADDRESS_P2PKH_ODD,
        BRC20_PUBKEY_ADDRESS_P2SH_P2WPKH_EVEN,
        BRC20_PUBKEY_ADDRESS_P2SH_P2WPKH_ODD,
        BRC20_PUBKEY_ADDRESS_P2TR_KEY,
    };
    use hex::FromHex;
    use {
        bitcoin::{
            secp256k1::{XOnlyPublicKey},
        },
    };

    #[test]
    fn test_script() {
        let pubkey_hex = "50929b74c1a04954b78b4b6035e97a5e078a5b3f50a59849cd485efb9a3a8d9b";
        let pubkey_bytes = <Vec<u8>>::from_hex(pubkey_hex).unwrap();
        let x_only_pubkey_bytes = &XOnlyPublicKey::from_slice(&pubkey_bytes).unwrap().serialize();

        let script = get_pk_script_by_pubkey_and_type(x_only_pubkey_bytes, BRC20_PUBKEY_ADDRESS_P2TR_SCRIPT).as_script().to_string();
        debug_assert_eq!(script, "OP_PUSHNUM_1 OP_PUSHBYTES_32 3f5d684aefca2c2dfe2a15e73da9baee46abd0565db698d774351dddf869c20b");

        let script = get_pk_script_by_pubkey_and_type(x_only_pubkey_bytes, BRC20_PUBKEY_ADDRESS_P2WPKH_EVEN).as_script().to_string();
        debug_assert_eq!(script, "OP_0 OP_PUSHBYTES_20 f2cf7388606c1115b219e1de85f369aca3381ea2");

        let script = get_pk_script_by_pubkey_and_type(x_only_pubkey_bytes, BRC20_PUBKEY_ADDRESS_P2WPKH_ODD).as_script().to_string();
        debug_assert_eq!(script, "OP_0 OP_PUSHBYTES_20 89ea44e198e0bb923ad5ca2300eda835cf9cf4c2");

        let script = get_pk_script_by_pubkey_and_type(x_only_pubkey_bytes, BRC20_PUBKEY_ADDRESS_P2PKH_EVEN).as_script().to_string();
        debug_assert_eq!(script, "OP_DUP OP_HASH160 OP_PUSHBYTES_20 f2cf7388606c1115b219e1de85f369aca3381ea2 OP_EQUALVERIFY OP_CHECKSIG");

        let script = get_pk_script_by_pubkey_and_type(x_only_pubkey_bytes, BRC20_PUBKEY_ADDRESS_P2PKH_ODD).as_script().to_string();
        debug_assert_eq!(script, "OP_DUP OP_HASH160 OP_PUSHBYTES_20 89ea44e198e0bb923ad5ca2300eda835cf9cf4c2 OP_EQUALVERIFY OP_CHECKSIG");

        let script = get_pk_script_by_pubkey_and_type(x_only_pubkey_bytes, BRC20_PUBKEY_ADDRESS_P2SH_P2WPKH_EVEN).as_script().to_string();
        debug_assert_eq!(script, "OP_HASH160 OP_PUSHBYTES_20 9fe13f2414b5e0c7c6929f07d4eab1be3ebc452a OP_EQUAL");

        let script = get_pk_script_by_pubkey_and_type(x_only_pubkey_bytes, BRC20_PUBKEY_ADDRESS_P2SH_P2WPKH_ODD).as_script().to_string();
        debug_assert_eq!(script, "OP_HASH160 OP_PUSHBYTES_20 0eb6f1c05f8e8b7f7aa8952e2ce64aa7f050215b OP_EQUAL");

        let script = get_pk_script_by_pubkey_and_type(x_only_pubkey_bytes, BRC20_PUBKEY_ADDRESS_P2TR_KEY).as_script().to_string();
        debug_assert_eq!(script, "OP_PUSHNUM_1 OP_PUSHBYTES_32 50929b74c1a04954b78b4b6035e97a5e078a5b3f50a59849cd485efb9a3a8d9b");
    }
}
