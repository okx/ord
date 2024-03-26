use super::*;
use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt::Formatter, str::FromStr};

pub const ORIGINAL_TICK_LENGTH: usize = 4;
pub const SELF_ISSUANCE_TICK_LENGTH: usize = 5;
pub const MAX_TICK_BYTE_COUNT: usize = SELF_ISSUANCE_TICK_LENGTH;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Tick(Box<[u8]>);

impl FromStr for Tick {
  type Err = BRC20Error;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let bytes = s.as_bytes();

    if bytes.len() < ORIGINAL_TICK_LENGTH || bytes.len() > SELF_ISSUANCE_TICK_LENGTH {
      return Err(BRC20Error::InvalidTickLen(s.to_string()));
    }

    Ok(Self(bytes.into()))
  }
}

impl Tick {
  pub fn as_str(&self) -> String {
    // NOTE: Tick comes from &str by from_str,
    // so it could be calling unwrap when convert to str
    String::from_utf8(self.0.to_vec()).unwrap()
  }

  pub fn to_lowercase(&self) -> LowerTick {
    LowerTick::new(&self.as_str().to_lowercase())
  }

  pub fn self_issuance_tick(&self) -> bool {
    self.0.len() == SELF_ISSUANCE_TICK_LENGTH
  }
}

impl Serialize for Tick {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: Serializer,
  {
    self.as_str().serialize(serializer)
  }
}

impl<'de> Deserialize<'de> for Tick {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    Self::from_str(&String::deserialize(deserializer)?)
      .map_err(|e| de::Error::custom(format!("deserialize tick error: {}", e)))
  }
}

impl Display for Tick {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.as_str())
  }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LowerTick(Box<[u8]>);

impl LowerTick {
  fn new(str: &str) -> Self {
    LowerTick(str.as_bytes().to_vec().into_boxed_slice())
  }

  pub fn as_str(&self) -> &str {
    std::str::from_utf8(&self.0).unwrap()
  }

  pub fn hex(&self) -> String {
    let mut data = [0u8; MAX_TICK_BYTE_COUNT * 4];
    data[..self.0.len()].copy_from_slice(&self.0);
    hex::encode(data)
  }

  pub fn min_hex() -> String {
    hex::encode([0u8; MAX_TICK_BYTE_COUNT * 4])
  }

  pub fn max_hex() -> String {
    hex::encode([0xffu8; MAX_TICK_BYTE_COUNT * 4])
  }
}

impl Display for LowerTick {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.as_str())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_tick_length_case() {
    assert!(Tick::from_str("XAİ").is_ok());
    assert!(Tick::from_str("XAİİ").is_err());
    assert!("XAİ".parse::<Tick>().is_ok());
    assert!("XAİİ".parse::<Tick>().is_err());
    assert!(Tick::from_str("X。").is_ok());
    assert!("X。".parse::<Tick>().is_ok());
    assert!(Tick::from_str("aBc1").is_ok());
    assert!("aBc1".parse::<Tick>().is_ok());
    assert!("ατ".parse::<Tick>().is_ok());
    assert!("∑ii".parse::<Tick>().is_ok()); // when self issuance is enabled
    assert!("∑i".parse::<Tick>().is_ok());
    assert!("⊢i".parse::<Tick>().is_ok());
    assert!("⊢ii".parse::<Tick>().is_ok()); // when self issuance is enabled
    assert!("≯a".parse::<Tick>().is_ok());
    assert!("a≯a".parse::<Tick>().is_ok()); // when self issuance is enabled
  }
  #[test]
  fn test_tick_hex() {
    assert_eq!(
      Tick::from_str("XAİ").unwrap().to_lowercase().hex(),
      "786169cc87000000000000000000000000000000"
    );
    assert_eq!(
      Tick::from_str("aBc1").unwrap().to_lowercase().hex(),
      "6162633100000000000000000000000000000000"
    );
  }

  #[test]
  fn test_tick_unicode_lowercase() {
    assert_eq!(
      Tick::from_str("XAİ").unwrap().to_lowercase().as_str(),
      "xai\u{307}"
    );
    assert_eq!(
      Tick::from_str("aBc1").unwrap().to_lowercase().as_str(),
      "abc1",
    );
    assert_eq!("ατ".parse::<Tick>().unwrap().to_lowercase().as_str(), "ατ");
    assert_eq!("∑H".parse::<Tick>().unwrap().to_lowercase().as_str(), "∑h");
    assert_eq!("⊢I".parse::<Tick>().unwrap().to_lowercase().as_str(), "⊢i");
    assert_eq!("≯A".parse::<Tick>().unwrap().to_lowercase().as_str(), "≯a");
  }

  #[test]
  fn test_tick_compare_ignore_case() {
    assert_ne!(Tick::from_str("aBc1"), Tick::from_str("AbC1"));

    assert_ne!(Tick::from_str("aBc1"), Tick::from_str("aBc2"));

    assert_eq!(
      Tick::from_str("aBc1").unwrap().to_lowercase(),
      Tick::from_str("AbC1").unwrap().to_lowercase(),
    );
    assert_ne!(
      Tick::from_str("aBc1").unwrap().to_lowercase(),
      Tick::from_str("AbC2").unwrap().to_lowercase(),
    );
  }

  #[test]
  fn test_tick_serialize() {
    let obj = Tick::from_str("Ab1;").unwrap();
    assert_eq!(serde_json::to_string(&obj).unwrap(), r#""Ab1;""#);
  }

  #[test]
  fn test_tick_deserialize() {
    assert_eq!(
      serde_json::from_str::<Tick>(r#""Ab1;""#).unwrap(),
      Tick::from_str("Ab1;").unwrap()
    );
  }
}
