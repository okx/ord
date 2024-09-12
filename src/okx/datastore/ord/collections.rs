use serde::{Deserialize, Serialize};
use std::fmt::Display;

// the act of marking an inscription.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum CollectionKind {
  BitMap,
  BRC20,
  BtcName,
}
impl Display for CollectionKind {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "{}",
      match self {
        CollectionKind::BitMap => String::from("bitmap"),
        CollectionKind::BtcName => String::from("btc_name"),
        CollectionKind::BRC20 => String::from("brc20"),
      }
    )
  }
}
