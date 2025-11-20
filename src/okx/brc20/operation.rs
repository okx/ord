use super::*;
use serde_json::{json, Value};

mod deploy;
mod mint;
mod predeploy;
mod prog_call;
mod prog_deploy;
mod prog_transact;
mod transfer;
mod withdraw;

pub use self::{
  deploy::Deploy, mint::Mint, predeploy::Predeploy, prog_call::ProgCall, prog_deploy::ProgDeploy,
  prog_transact::ProgTransact, transfer::Transfer, withdraw::Withdraw,
};

pub const PROTOCOL_LITERAL: &str = "brc-20";

pub const PROG_PROTOCOL_LITERAL: &str = "brc20-prog";
pub const MODULE_PROTOCOL_LITERAL: &str = "brc20-module";
pub const PROG_MODULE_LITERAL: &str = "BRC20PROG";

pub trait BRC20OperationExtractor {
  fn extract_brc20_operation(&self) -> Result<RawOperation, Error>;
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
#[serde(tag = "op")]
pub enum RawOperation {
  #[serde(rename = "predeploy")]
  Predeploy(Predeploy),
  #[serde(rename = "deploy")]
  Deploy(Deploy),
  #[serde(rename = "mint")]
  Mint(Mint),
  #[serde(rename = "transfer")]
  Transfer(Transfer),
  #[serde(rename = "prog-deploy")]
  ProgDeploy {
    deploy: ProgDeploy,
    inscription_byte_length: u64,
  },
  #[serde(rename = "prog-call")]
  ProgCall {
    call: ProgCall,
    inscription_byte_length: u64,
  },
  #[serde(rename = "prog-transact")]
  ProgTransact {
    transact: ProgTransact,
    inscription_byte_length: u64,
  },
  #[serde(rename = "withdraw")]
  Withdraw(Withdraw),
}

impl BRC20OperationExtractor for Inscription {
  fn extract_brc20_operation(&self) -> Result<RawOperation, Error> {
    let content_body = self
      .body()
      .and_then(|body| std::str::from_utf8(body).ok())
      .ok_or(Error::InvalidJson)?;

    if content_body.len() < 40 {
      return Err(Error::NotBRC20Json);
    }

    let content_type = self.content_type().ok_or(Error::InvalidContentType)?;

    if content_type != "text/plain"
      && content_type != "text/plain;charset=utf-8"
      && content_type != "text/plain;charset=UTF-8"
      && content_type != "application/json"
      && content_type != "application/json;charset=utf-8"
      && content_type != "application/json;charset=UTF-8"
      && !content_type.starts_with("text/plain;")
    {
      return Err(Error::UnSupportContentType);
    }

    let value: Value = serde_json::from_str(content_body).map_err(|_| Error::InvalidJson)?;
    let content_length = content_body.len();

    if value.get("p") != Some(&json!(PROTOCOL_LITERAL))
      && value.get("p") != Some(&json!(PROG_PROTOCOL_LITERAL))
      && (value.get("p") != Some(&json!(MODULE_PROTOCOL_LITERAL))
        || value.get("op") != Some(&json!("withdraw"))
        || value.get("module") != Some(&json!(PROG_MODULE_LITERAL)))
    {
      return Err(Error::NotBRC20Json);
    }

    deserialize_brc20_operation(&value)
      .or_else(|_| deserialize_brc20_prog_operation(&value, content_length))
      .or_else(|_| deserialize_withdraw_operation(&value))
  }
}

fn deserialize_brc20_operation(value: &Value) -> Result<RawOperation, Error> {
  if value.get("p") != Some(&json!(PROTOCOL_LITERAL)) {
    // Allow "p": "brc-20"
    return Err(Error::NotBRC20Json);
  }

  if !matches!(
    value.get("op").and_then(|v| v.as_str()),
    Some("predeploy") | Some("deploy") | Some("mint") | Some("transfer") // Supported ops
  ) {
    return Err(Error::NotBRC20Json);
  }

  serde_json::from_value(value.clone()).map_err(|e| Error::ParseOperationJsonError(e.to_string()))
}

fn deserialize_brc20_prog_operation(
  value: &Value,
  content_length: usize,
) -> Result<RawOperation, Error> {
  if value.get("p") == Some(&json!(PROG_PROTOCOL_LITERAL)) {
    // Allow "p": "brc20-prog"
    match value.get("op").and_then(|v| v.as_str()) {
      Some("deploy") | Some("d") => {
        return Ok(RawOperation::ProgDeploy {
          deploy: serde_json::from_value::<ProgDeploy>(value.clone())
            .map_err(|e| Error::ParseOperationJsonError(e.to_string()))?,
          inscription_byte_length: content_length as u64,
        });
      }
      Some("call") | Some("c") => {
        return Ok(RawOperation::ProgCall {
          call: serde_json::from_value::<ProgCall>(value.clone())
            .map_err(|e| Error::ParseOperationJsonError(e.to_string()))?,
          inscription_byte_length: content_length as u64,
        });
      }
      Some("transact") | Some("t") => {
        return Ok(RawOperation::ProgTransact {
          transact: serde_json::from_value::<ProgTransact>(value.clone())
            .map_err(|e| Error::ParseOperationJsonError(e.to_string()))?,
          inscription_byte_length: content_length as u64,
        });
      }
      _ => return Err(Error::NotBRC20Json),
    }
  }

  return Err(Error::NotBRC20Json);
}

fn deserialize_withdraw_operation(value: &Value) -> Result<RawOperation, Error> {
  if value.get("p") != Some(&json!(MODULE_PROTOCOL_LITERAL)) // Allow "p": "brc20-module"
      || value.get("op") != Some(&json!("withdraw"))
      || value.get("module") != Some(&json!(PROG_MODULE_LITERAL))
  {
    return Err(Error::NotBRC20Json);
  }

  Ok(RawOperation::Withdraw(
    serde_json::from_value::<Withdraw>(value.clone())
      .map_err(|e| Error::ParseOperationJsonError(e.to_string()))?,
  ))
}

#[derive(PartialEq, Debug)]
pub enum Error {
  InvalidContentType,
  UnSupportContentType,
  InvalidJson,
  NotBRC20Json,
  ParseOperationJsonError(String),
}

impl Display for Error {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    match self {
      Self::InvalidContentType => write!(f, "Invalid content type"),
      Self::UnSupportContentType => write!(f, "Unsupported content type"),
      Self::InvalidJson => write!(f, "Invalid JSON string"),
      Self::NotBRC20Json => write!(f, "Not a valid BRC20 JSON"),
      Self::ParseOperationJsonError(err) => write!(f, "Failed to parse operation JSON: {}", err),
    }
  }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_predeploy_deserialize() {
    let hash = "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890";
    let decoded_hash = hex::decode(hash).unwrap().as_slice().try_into().unwrap();
    let json_str = format!(
      r##"{{
      "p": "brc-20",
      "op": "predeploy",
      "hash": "{hash}"
    }}"##
    );
    assert_eq!(
      deserialize_brc20_operation(&serde_json::from_str(&json_str).unwrap()).unwrap(),
      RawOperation::Predeploy(Predeploy { hash: decoded_hash })
    );
  }

  #[test]
  fn test_predeploy_deserialize_invalid_hash() {
    let hash = "invalidhash";
    let json_str = format!(
      r##"{{
      "p": "brc-20",
      "op": "predeploy",
      "hash": "{hash}"
    }}"##
    );
    assert!(deserialize_brc20_operation(&serde_json::from_str(&json_str).unwrap()).is_err());
  }

  #[test]
  fn test_deploy_deserialize() {
    let max_supply = "21000000".to_string();
    let mint_limit = "1000".to_string();

    let json_str = format!(
      r##"{{
  "p": "brc-20",
  "op": "deploy",
  "tick": "ordi",
  "max": "{max_supply}",
  "lim": "{mint_limit}"
}}"##
    );

    assert_eq!(
      deserialize_brc20_operation(&serde_json::from_str(&json_str).unwrap()).unwrap(),
      RawOperation::Deploy(Deploy {
        tick: "ordi".to_string(),
        max_supply,
        mint_limit: Some(mint_limit),
        decimals: None,
        self_mint: None,
        salt: None,
      })
    );
  }

  #[test]
  fn test_mint_deserialize() {
    let amount = "1000".to_string();

    let json_str = format!(
      r##"{{
  "p": "brc-20",
  "op": "mint",
  "tick": "ordi",
  "amt": "{amount}"
}}"##
    );

    assert_eq!(
      deserialize_brc20_operation(&serde_json::from_str(&json_str).unwrap()).unwrap(),
      RawOperation::Mint(Mint {
        tick: "ordi".to_string(),
        amount,
      })
    );
  }

  #[test]
  fn test_transfer_deserialize() {
    let amount = "100".to_string();

    let json_str = format!(
      r##"{{
  "p": "brc-20",
  "op": "transfer",
  "tick": "ordi",
  "amt": "{amount}"
}}"##
    );

    assert_eq!(
      deserialize_brc20_operation(&serde_json::from_str(&json_str).unwrap()).unwrap(),
      RawOperation::Transfer(Transfer {
        tick: "ordi".to_string(),
        amount,
      })
    );
  }

  #[test]
  fn test_json_duplicate_field() {
    let json_str = r#"{"p":"brc-20","op":"mint","tick":"smol","amt":"333","amt":"33"}"#;
    assert_eq!(
      deserialize_brc20_operation(&serde_json::from_str(&json_str).unwrap()).unwrap(),
      RawOperation::Mint(Mint {
        tick: String::from("smol"),
        amount: String::from("33"),
      })
    )
  }

  #[test]
  fn test_missing_required_key() {
    assert_eq!(
      deserialize_brc20_operation(
        &serde_json::from_str(r#"{"p":"brc-20","op":"transfer","tick":"abcd"}"#).unwrap()
      )
      .unwrap_err(),
      Error::ParseOperationJsonError("missing field `amt`".to_string())
    );
  }

  #[test]
  fn test_json_non_string() {
    let json_str = r#"{"p":"brc-20","op":"mint","tick":"smol","amt":33}"#;
    assert!(deserialize_brc20_operation(&serde_json::from_str(&json_str).unwrap()).is_err())
  }

  #[test]
  fn test_deserialize_case_insensitive() {
    let max_supply = "21000000".to_string();
    let mint_limit = "1000".to_string();

    let json_str = format!(
      r##"{{
  "P": "brc-20",
  "Op": "deploy",
  "Tick": "ordi",
  "mAx": "{max_supply}",
  "Lim": "{mint_limit}"
}}"##
    );

    assert_eq!(
      deserialize_brc20_operation(&serde_json::from_str(&json_str).unwrap()),
      Err(Error::NotBRC20Json)
    );
  }

  #[test]
  fn test_duplicate_key() {
    let json_str = r#"{"p":"brc-20","op":"deploy","tick":"smol","max":"100","lim":"10","dec":"17","max":"200","lim":"20","max":"300"}"#;
    assert_eq!(
      deserialize_brc20_operation(&serde_json::from_str(&json_str).unwrap()).unwrap(),
      RawOperation::Deploy(Deploy {
        tick: "smol".to_string(),
        max_supply: "300".to_string(),
        mint_limit: Some("20".to_string()),
        decimals: Some("17".to_string()),
        self_mint: None,
        salt: None,
      })
    );

    let json_str = r#"{"p":"brc-20","op":"mint","tick":"smol","amt":"100","tick":"hhaa","amt":"200","tick":"actt"}"#;
    assert_eq!(
      deserialize_brc20_operation(&serde_json::from_str(&json_str).unwrap()).unwrap(),
      RawOperation::Mint(Mint {
        tick: "actt".to_string(),
        amount: "200".to_string(),
      })
    );

    let json_str = r#"{"p":"brc-20","op":"transfer","tick":"smol","amt":"100","tick":"hhaa","amt":"200","tick":"actt"}"#;
    assert_eq!(
      deserialize_brc20_operation(&serde_json::from_str(&json_str).unwrap()).unwrap(),
      RawOperation::Transfer(Transfer {
        tick: "actt".to_string(),
        amount: "200".to_string(),
      })
    );
  }

  #[test]
  fn test_prog_deploy_deserialize() {
    let json_str = r##"{"p": "brc20-prog","op": "deploy","d": "0x0000123456789abcdef"}"##;
    assert_eq!(
      deserialize_brc20_prog_operation(&serde_json::from_str(&json_str).unwrap(), json_str.len())
        .unwrap(),
      RawOperation::ProgDeploy {
        deploy: ProgDeploy {
          data: Some("0x0000123456789abcdef".to_string()),
          base64_data: None
        },
        inscription_byte_length: json_str.len() as u64,
      }
    );
  }
}
