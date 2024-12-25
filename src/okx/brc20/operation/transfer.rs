use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct Transfer {
  #[serde(rename = "tick")]
  pub tick: String,
  #[serde(rename = "amt")]
  pub amount: String,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_serialize() {
    let obj = Transfer {
      tick: "abcd".to_string(),
      amount: "333".to_string(),
    };
    assert_eq!(
      serde_json::to_string(&obj).unwrap(),
      r#"{"tick":"abcd","amt":"333"}"#
    );
  }

  #[test]
  fn test_deserialize() {
    assert_eq!(
      serde_json::from_str::<Transfer>(r#"{"tick":"abcd","amt":"12000"}"#).unwrap(),
      Transfer {
        tick: "abcd".to_string(),
        amount: "12000".to_string()
      }
    );
  }

  #[test]
  fn test_missing_required_key() {
    assert_eq!(
      serde_json::from_str::<Transfer>(r#"{"tick":"abcd"}"#)
        .unwrap_err()
        .to_string(),
      "missing field `amt` at line 1 column 15"
    );
  }
}
