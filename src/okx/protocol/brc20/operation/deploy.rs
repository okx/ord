use serde::{de, Deserialize, Deserializer, Serialize};
use std::str::FromStr;
#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct Deploy {
  #[serde(rename = "tick")]
  pub tick: String,
  #[serde(rename = "max")]
  pub max_supply: String,
  #[serde(rename = "lim", skip_serializing_if = "Option::is_none")]
  pub mint_limit: Option<String>,
  #[serde(rename = "dec", skip_serializing_if = "Option::is_none")]
  pub decimals: Option<String>,
  #[serde(
    default,
    rename = "self_mint",
    skip_serializing_if = "Option::is_none",
    deserialize_with = "de_from_str",
    serialize_with = "ser_to_str"
  )]
  pub self_mint: Option<bool>,
}

fn de_from_str<'de, D>(deserializer: D) -> Result<Option<bool>, D::Error>
where
  D: Deserializer<'de>,
{
  let s = Option::<String>::deserialize(deserializer)?;
  match s {
    Some(s) => bool::from_str(&s).map_err(de::Error::custom).map(Some),
    None => Ok(None),
  }
}

fn ser_to_str<S>(v: &Option<bool>, serializer: S) -> Result<S::Ok, S::Error>
where
  S: serde::Serializer,
{
  match v {
    Some(v) => serializer.serialize_str(&v.to_string()),
    None => serializer.serialize_none(),
  }
}

#[cfg(test)]
mod tests {
  use super::super::*;
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
      deserialize_brc20(
        r#"{"p":"brc-20","op":"deploy","tick":"abcd","max":"12000","lim":"12","dec":"11"}"#
      )
      .unwrap(),
      RawOperation::Deploy(Deploy {
        tick: "abcd".to_string(),
        max_supply: "12000".to_string(),
        mint_limit: Some("12".to_string()),
        decimals: Some("11".to_string()),
        self_mint: None,
      })
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
  fn test_self_mint_deserialize() {
    assert_eq!(
      deserialize_brc20(
        r#"{"p":"brc-20","op":"deploy","tick":"abcd","max":"12000","lim":"12","dec":"11"}"#
      )
      .unwrap(),
      RawOperation::Deploy(Deploy {
        tick: "abcd".to_string(),
        max_supply: "12000".to_string(),
        mint_limit: Some("12".to_string()),
        decimals: Some("11".to_string()),
        self_mint: None,
      })
    );

    assert_eq!(
      deserialize_brc20(
        r#"{"p":"brc-20","op":"deploy","tick":"abcd","max":"12000","lim":"12","dec":"11","self_mint":"true"}"#
      )
      .unwrap(),
      RawOperation::Deploy(Deploy {
        tick: "abcd".to_string(),
        max_supply: "12000".to_string(),
        mint_limit: Some("12".to_string()),
        decimals: Some("11".to_string()),
        self_mint: Some(true),
      })
    );
    assert_eq!(
      deserialize_brc20(
        r#"{"p":"brc-20","op":"deploy","tick":"abcd","max":"12000","lim":"12","dec":"11","self_mint":"false"}"#
      )
      .unwrap(),
      RawOperation::Deploy(Deploy {
        tick: "abcd".to_string(),
        max_supply: "12000".to_string(),
        mint_limit: Some("12".to_string()),
        decimals: Some("11".to_string()),
        self_mint: Some(false),
      })
    );
  }

  #[test]
  fn test_self_mint_deserialize_with_error_value() {
    assert_eq!(
      deserialize_brc20(
        r#"{"p":"brc-20","op":"deploy","tick":"abcd","max":"12000","lim":"12","dec":"11","self_mint":"True"}"#
      )
      .unwrap_err(),
      JSONError::ParseOperationJsonError("provided string was not `true` or `false`".to_string())
    );
    assert_eq!(
      deserialize_brc20(
        r#"{"p":"brc-20","op":"deploy","tick":"abcd","max":"12000","lim":"12","dec":"11","self_mint":"t"}"#
      )
      .unwrap_err(),
      JSONError::ParseOperationJsonError("provided string was not `true` or `false`".to_string())
    );
    assert_eq!(
      deserialize_brc20(
        r#"{"p":"brc-20","op":"deploy","tick":"abcd","max":"12000","lim":"12","dec":"11","self_mint":true}"#
      )
      .unwrap_err(),
      JSONError::ParseOperationJsonError("invalid type: boolean `true`, expected a string".to_string())
    );
  }

  #[test]
  fn test_loss_require_key() {
    assert_eq!(
      deserialize_brc20(r#"{"p":"brc-20","op":"deploy","tick":"11","lim":"22","dec":"11"}"#)
        .unwrap_err(),
      JSONError::ParseOperationJsonError("missing field `max`".to_string())
    );
  }

  #[test]
  fn test_loss_option_key() {
    // loss lim
    assert_eq!(
      deserialize_brc20(r#"{"p":"brc-20","op":"deploy","tick":"smol","max":"100","dec":"10"}"#)
        .unwrap(),
      RawOperation::Deploy(Deploy {
        tick: "smol".to_string(),
        max_supply: "100".to_string(),
        mint_limit: None,
        decimals: Some("10".to_string()),
        self_mint: None,
      })
    );

    // loss dec
    assert_eq!(
      deserialize_brc20(r#"{"p":"brc-20","op":"deploy","tick":"smol","max":"100","lim":"10"}"#)
        .unwrap(),
      RawOperation::Deploy(Deploy {
        tick: "smol".to_string(),
        max_supply: "100".to_string(),
        mint_limit: Some("10".to_string()),
        decimals: None,
        self_mint: None,
      })
    );

    // loss all option
    assert_eq!(
      deserialize_brc20(r#"{"p":"brc-20","op":"deploy","tick":"smol","max":"100"}"#).unwrap(),
      RawOperation::Deploy(Deploy {
        tick: "smol".to_string(),
        max_supply: "100".to_string(),
        mint_limit: None,
        decimals: None,
        self_mint: None,
      })
    );
  }

  #[test]
  fn test_duplicate_key() {
    let json_str = r#"{"p":"brc-20","op":"deploy","tick":"smol","max":"100","lim":"10","dec":"17","max":"200","lim":"20","max":"300"}"#;
    assert_eq!(
      deserialize_brc20(json_str).unwrap(),
      RawOperation::Deploy(Deploy {
        tick: "smol".to_string(),
        max_supply: "300".to_string(),
        mint_limit: Some("20".to_string()),
        decimals: Some("17".to_string()),
        self_mint: None,
      })
    );
  }
}
