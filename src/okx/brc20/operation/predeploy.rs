use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct Predeploy {
  #[serde(rename = "hash", with = "parse_sha256")]
  pub hash: [u8; 32],
}

mod parse_sha256 {
  use serde::{de, Deserialize, Deserializer, Serializer};

  pub fn serialize<S>(v: &[u8; 32], serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    serializer.serialize_str(&hex::encode(v))
  }

  pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 32], D::Error>
  where
    D: Deserializer<'de>,
  {
    let s: String = Deserialize::deserialize(deserializer)?;
    let bytes = hex::decode(s).map_err(|_| de::Error::custom("invalid hex string"))?;
    if bytes.len() != 32 {
      return Err(de::Error::custom("invalid length for hash"));
    }
    let mut array = [0u8; 32];
    array.copy_from_slice(&bytes);
    Ok(array)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_serialize() {
    let obj = Predeploy {
      hash: [
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        26, 27, 28, 29, 30, 31, 32,
      ],
    };
    assert_eq!(
      serde_json::to_string(&obj).unwrap(),
      r#"{"hash":"0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20"}"#
    );
  }

  #[test]
  fn test_deserialize() {
    assert_eq!(
      serde_json::from_str::<Predeploy>(
        r#"{"hash":"0102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f20"}"#
      )
      .unwrap(),
      Predeploy {
        hash: [
          1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
          25, 26, 27, 28, 29, 30, 31, 32
        ],
      }
    );
  }

  #[test]
  fn test_deserialize_invalid_hex_string() {
    assert!(serde_json::from_str::<Predeploy>(r#"{"hash":"abcdeflol"}"#).is_err());
  }

  #[test]
  fn test_deserialize_invalid_length() {
    assert!(serde_json::from_str::<Predeploy>(r#"{"hash":"abcdef"}"#).is_err());
  }
}
