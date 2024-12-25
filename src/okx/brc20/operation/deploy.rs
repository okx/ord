use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct Deploy {
  #[serde(rename = "tick")]
  pub tick: String,
  #[serde(rename = "max")]
  pub max_supply: String,
  #[serde(rename = "lim", skip_serializing_if = "Option::is_none")]
  pub mint_limit: Option<String>,
  #[serde(rename = "dec", skip_serializing_if = "Option::is_none")]
  pub decimals: Option<String>,
  #[serde(default, with = "parse_bool", skip_serializing_if = "Option::is_none")]
  pub self_mint: Option<bool>,
}

mod parse_bool {
  use serde::{de, Deserialize, Deserializer};
  pub fn serialize<S>(v: &Option<bool>, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    match v {
      Some(v) => serializer.serialize_str(&v.to_string()),
      None => serializer.serialize_none(),
    }
  }

  pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
  where
    D: Deserializer<'de>,
  {
    let s: Option<String> = Deserialize::deserialize(deserializer)?;

    match s.as_deref() {
      Some("true") => Ok(Some(true)),
      Some("false") => Ok(Some(false)),
      Some(v) => Err(de::Error::unknown_variant(v, &["true", "false"])),
      None => Ok(None),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_serialize() {
    let obj = Deploy {
      tick: "abcd".to_string(),
      max_supply: "12000".to_string(),
      mint_limit: Some("12".to_string()),
      decimals: Some("11".to_string()),
      self_mint: None,
    };

    assert_eq!(
      serde_json::to_string(&obj).unwrap(),
      format!(
        r##"{{"tick":"{}","max":"{}","lim":"{}","dec":"{}"}}"##,
        obj.tick,
        obj.max_supply,
        obj.mint_limit.unwrap(),
        obj.decimals.unwrap()
      )
    )
  }

  #[test]
  fn test_deserialize() {
    assert_eq!(
      serde_json::from_str::<Deploy>(r#"{"tick":"abcd","max":"12000","lim":"12","dec":"11"}"#)
        .unwrap(),
      Deploy {
        tick: "abcd".to_string(),
        max_supply: "12000".to_string(),
        mint_limit: Some("12".to_string()),
        decimals: Some("11".to_string()),
        self_mint: None,
      }
    );
  }

  #[test]
  fn test_self_mint_serialize() {
    let obj = Deploy {
      tick: "abcd".to_string(),
      max_supply: "12000".to_string(),
      mint_limit: Some("12".to_string()),
      decimals: Some("11".to_string()),
      self_mint: None,
    };

    assert_eq!(
      serde_json::to_string(&obj).unwrap(),
      format!(
        r##"{{"tick":"{}","max":"{}","lim":"{}","dec":"{}"}}"##,
        obj.tick,
        obj.max_supply,
        obj.mint_limit.as_ref().unwrap(),
        obj.decimals.as_ref().unwrap(),
      )
    );

    let obj = Deploy {
      self_mint: Some(true),
      ..obj
    };

    assert_eq!(
      serde_json::to_string(&obj).unwrap(),
      format!(
        r##"{{"tick":"{}","max":"{}","lim":"{}","dec":"{}","self_mint":"{}"}}"##,
        obj.tick,
        obj.max_supply,
        obj.mint_limit.as_ref().unwrap(),
        obj.decimals.as_ref().unwrap(),
        obj.self_mint.as_ref().unwrap()
      )
    );

    let obj = Deploy {
      self_mint: Some(false),
      ..obj
    };
    assert_eq!(
      serde_json::to_string(&obj).unwrap(),
      format!(
        r##"{{"tick":"{}","max":"{}","lim":"{}","dec":"{}","self_mint":"{}"}}"##,
        obj.tick,
        obj.max_supply,
        obj.mint_limit.as_ref().unwrap(),
        obj.decimals.as_ref().unwrap(),
        obj.self_mint.as_ref().unwrap()
      )
    )
  }

  #[test]
  fn test_five_bytes_ticker_self_mint_deserialize() {
    let json_str = r#"{"tick":"abcde","max":"100","lim":"10","dec":"10","self_mint":"true"}"#;
    assert_eq!(
      serde_json::from_str::<Deploy>(json_str).unwrap(),
      Deploy {
        tick: "abcde".to_string(),
        max_supply: "100".to_string(),
        mint_limit: Some("10".to_string()),
        decimals: Some("10".to_string()),
        self_mint: Some(true),
      }
    );

    let json_str = r#"{"self_mint":"true","tick":"abcde","max":"100","lim":"10","dec":"10"}"#;
    assert_eq!(
      serde_json::from_str::<Deploy>(json_str).unwrap(),
      Deploy {
        tick: "abcde".to_string(),
        max_supply: "100".to_string(),
        mint_limit: Some("10".to_string()),
        decimals: Some("10".to_string()),
        self_mint: Some(true),
      }
    );

    let json_str = r#"{"self_mint":"false","tick":"abcde","max":"100","lim":"10","dec":"10"}"#;
    assert_eq!(
      serde_json::from_str::<Deploy>(json_str).unwrap(),
      Deploy {
        tick: "abcde".to_string(),
        max_supply: "100".to_string(),
        mint_limit: Some("10".to_string()),
        decimals: Some("10".to_string()),
        self_mint: Some(false),
      }
    );
  }

  #[test]
  fn test_self_mint_deserialize_with_error_value() {
    assert_eq!(
      serde_json::from_str::<Deploy>(
        r#"{"tick":"abcde","max":"12000","lim":"12","dec":"11","self_mint":"True"}"#
      )
      .unwrap_err()
      .to_string(),
      "unknown variant `True`, expected `true` or `false` at line 1 column 71"
    );

    assert_eq!(
      serde_json::from_str::<Deploy>(
        r#"{"tick":"abcde","max":"12000","lim":"12","dec":"11","self_mint":"t"}"#
      )
      .unwrap_err()
      .to_string(),
      "unknown variant `t`, expected `true` or `false` at line 1 column 68"
    );

    assert_eq!(
      serde_json::from_str::<Deploy>(
        r#"{"tick":"abcde","max":"12000","lim":"12","dec":"11","self_mint":true}"#
      )
      .unwrap_err()
      .to_string(),
      "invalid type: boolean `true`, expected a string at line 1 column 68"
    );
  }

  #[test]
  fn test_missing_required_key() {
    assert_eq!(
      serde_json::from_str::<Deploy>(r#"{"tick":"11","lim":"22","dec":"11"}"#)
        .unwrap_err()
        .to_string(),
      "missing field `max` at line 1 column 35"
    );
  }

  #[test]
  fn test_missing_option_key() {
    // loss lim
    assert_eq!(
      serde_json::from_str::<Deploy>(r#"{"tick":"smol","max":"100","dec":"10"}"#).unwrap(),
      Deploy {
        tick: "smol".to_string(),
        max_supply: "100".to_string(),
        mint_limit: None,
        decimals: Some("10".to_string()),
        self_mint: None,
      }
    );

    // loss dec
    assert_eq!(
      serde_json::from_str::<Deploy>(r#"{"tick":"smol","max":"100","lim":"10"}"#).unwrap(),
      Deploy {
        tick: "smol".to_string(),
        max_supply: "100".to_string(),
        mint_limit: Some("10".to_string()),
        decimals: None,
        self_mint: None,
      }
    );

    // loss all option
    assert_eq!(
      serde_json::from_str::<Deploy>(r#"{"tick":"smol","max":"100"}"#).unwrap(),
      Deploy {
        tick: "smol".to_string(),
        max_supply: "100".to_string(),
        mint_limit: None,
        decimals: None,
        self_mint: None,
      }
    );
  }
}
