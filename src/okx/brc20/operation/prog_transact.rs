use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct ProgTransact {
  #[serde(rename = "d")]
  pub data: Option<String>,
  #[serde(rename = "b")]
  pub base64_data: Option<String>,
}
