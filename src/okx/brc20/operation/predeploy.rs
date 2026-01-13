use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub struct Predeploy {
  #[serde(rename = "hash")]
  pub hash: String,
}
