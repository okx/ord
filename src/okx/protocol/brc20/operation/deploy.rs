use crate::okx::datastore::brc20::SELF_ISSUANCE_TICK_LENGTH;
use serde::{de, Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::str::FromStr;

#[derive(Debug, PartialEq, Clone, Serialize)]
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
    serialize_with = "ser_to_str"
  )]
  pub self_mint: Option<bool>,
}

impl<'de> Deserialize<'de> for Deploy {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    #[derive(Deserialize)]
    struct DeployFields {
      tick: String,
      max: String,
      lim: Option<String>,
      dec: Option<String>,
      self_mint: Option<Value>,
    }
    let deploy = DeployFields::deserialize(deserializer)?;
    let self_mint = if deploy.tick.len() == SELF_ISSUANCE_TICK_LENGTH {
      match deploy.self_mint {
        Some(v) => Some(
          bool::from_str(
            serde_json::from_value::<String>(v)
              .map_err(de::Error::custom)?
              .as_str(),
          )
          .map_err(de::Error::custom)?,
        ),
        None => return Err(de::Error::missing_field("self_mint")),
      }
    } else {
      None
    };
    Ok(Deploy {
      tick: deploy.tick.clone(),
      max_supply: deploy.max,
      mint_limit: deploy.lim,
      decimals: deploy.dec,
      self_mint,
    })
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
  fn test_five_bytes_ticker_self_mint_deserialize() {
    let json_str = r#"{"p":"brc-20","op":"deploy","tick":"abcde","max":"100","lim":"10","dec":"10","self_mint":"true"}"#;
    assert_eq!(
      deserialize_brc20(json_str).unwrap(),
      RawOperation::Deploy(Deploy {
        tick: "abcde".to_string(),
        max_supply: "100".to_string(),
        mint_limit: Some("10".to_string()),
        decimals: Some("10".to_string()),
        self_mint: Some(true),
      })
    );

    let json_str = r#"{"self_mint":"true","p":"brc-20","op":"deploy","tick":"abcde","max":"100","lim":"10","dec":"10"}"#;
    assert_eq!(
      deserialize_brc20(json_str).unwrap(),
      RawOperation::Deploy(Deploy {
        tick: "abcde".to_string(),
        max_supply: "100".to_string(),
        mint_limit: Some("10".to_string()),
        decimals: Some("10".to_string()),
        self_mint: Some(true),
      })
    );
  }

  #[test]
  fn test_self_mint_deserialize_with_error_value() {
    assert_eq!(
      deserialize_brc20(
        r#"{"p":"brc-20","op":"deploy","tick":"abcde","max":"12000","lim":"12","dec":"11","self_mint":"True"}"#
      )
      .unwrap_err(),
      JSONError::ParseOperationJsonError("provided string was not `true` or `false`".to_string())
    );
    assert_eq!(
      deserialize_brc20(
        r#"{"p":"brc-20","op":"deploy","tick":"abcde","max":"12000","lim":"12","dec":"11","self_mint":"t"}"#
      )
      .unwrap_err(),
      JSONError::ParseOperationJsonError("provided string was not `true` or `false`".to_string())
    );
    assert_eq!(
      deserialize_brc20(
        r#"{"p":"brc-20","op":"deploy","tick":"abcde","max":"12000","lim":"12","dec":"11","self_mint":true}"#
      )
      .unwrap_err(),
      JSONError::ParseOperationJsonError("invalid type: boolean `true`, expected a string".to_string())
    );
  }

  #[test]
  fn test_loss_self_mint() {
    assert_eq!(
      deserialize_brc20(
        r#"{"p":"brc-20","op":"deploy","tick":"1234","max":"100","lim":"22","dec":"11"}"#
      )
      .unwrap(),
      RawOperation::Deploy(Deploy {
        tick: "1234".to_string(),
        max_supply: "100".to_string(),
        mint_limit: Some("22".to_string()),
        decimals: Some("11".to_string()),
        self_mint: None,
      })
    );
    assert_eq!(
      deserialize_brc20(
        r#"{"p":"brc-20","op":"deploy","tick":"12345","max":"100","lim":"22","dec":"11"}"#
      )
      .unwrap_err(),
      JSONError::ParseOperationJsonError("missing field `self_mint`".to_string())
    );
  }

  #[test]
  fn test_ignore_self_mint() {
    assert_eq!(
      deserialize_brc20(
        r#"{"p":"brc-20","op":"deploy","tick":"1234","max":"100","lim":"22","dec":"11","self_mint":"true"}"#
      )
      .unwrap(),
      RawOperation::Deploy(Deploy {
        tick: "1234".to_string(),
        max_supply: "100".to_string(),
        mint_limit: Some("22".to_string()),
        decimals: Some("11".to_string()),
        self_mint: None,
      })
    );
    assert_eq!(
      deserialize_brc20(
        r#"{"p":"brc-20","op":"deploy","tick":"1234","max":"100","lim":"22","dec":"11","self_mint":true}"#
      )
      .unwrap(),
      RawOperation::Deploy(Deploy {
        tick: "1234".to_string(),
        max_supply: "100".to_string(),
        mint_limit: Some("22".to_string()),
        decimals: Some("11".to_string()),
        self_mint: None,
      })
    );
    assert_eq!(
      deserialize_brc20(
        r#"{"p":"brc-20","op":"deploy","tick":"1234","max":"100","lim":"22","dec":"11","self_mint":"True"}"#
      )
      .unwrap(),
      RawOperation::Deploy(Deploy {
        tick: "1234".to_string(),
        max_supply: "100".to_string(),
        mint_limit: Some("22".to_string()),
        decimals: Some("11".to_string()),
        self_mint: None,
      })
    );
  }

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
