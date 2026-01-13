use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct ProgCall {
  #[serde(rename = "c")]
  pub contract_address: Option<String>,
  #[serde(rename = "i")]
  pub inscription_id: Option<String>,
  #[serde(rename = "d")]
  pub data: Option<String>,
  #[serde(rename = "b")]
  pub base64_data: Option<String>,
}
