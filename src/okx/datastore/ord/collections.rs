use serde::{Deserialize, Serialize};
use std::fmt::Display;

// the act of marking an inscription.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum CollectionKind {
  BitMap,
  BRC20,
  BtcName,
  UnisatName,
  SatsName,
  XName,
}
impl Display for CollectionKind {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(
      f,
      "{}",
      match self {
        CollectionKind::BitMap => String::from("bitmap"),
        CollectionKind::BRC20 => String::from("brc20"),
        CollectionKind::BtcName => String::from("btc_name"),
        CollectionKind::UnisatName => String::from("unisat_name"),
        CollectionKind::SatsName => String::from("sats_name"),
        CollectionKind::XName => String::from("x_name"),
      }
    )
  }
}

impl TryFrom<&str> for CollectionKind {
  type Error = ();
  fn try_from(value: &str) -> Result<Self, Self::Error> {
    match value {
      "btc" => Ok(CollectionKind::BtcName),
      "unisat" => Ok(CollectionKind::UnisatName),
      "sats" => Ok(CollectionKind::SatsName),
      "x" => Ok(CollectionKind::XName),
      _ => Err(()),
    }
  }
}
