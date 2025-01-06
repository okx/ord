use super::*;

#[derive(Debug, PartialEq, Clone, PartialOrd, Ord, Eq, Serialize, DeserializeFromStr)]
pub struct BRC20Ticker(Box<[u8]>);

impl BRC20Ticker {
  pub const MIN_SIZE: usize = 4;
  pub const MAX_SIZE: usize = 5;

  pub fn len(&self) -> usize {
    self.0.len()
  }

  pub fn to_lowercase(&self) -> BRC20LowerCaseTicker {
    let str = self.to_string().to_lowercase();
    BRC20LowerCaseTicker(str.as_bytes().to_vec().into_boxed_slice())
  }
}

impl Display for BRC20Ticker {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(f, "{}", std::str::from_utf8(&self.0).unwrap())
  }
}

impl FromStr for BRC20Ticker {
  type Err = Error;

  fn from_str(s: &str) -> Result<Self, Self::Err> {
    let bytes = s.as_bytes();
    let length = bytes.len();

    // BRC20Ticker names on the Bitcoin mainnet will be limited to 4 - 5 bytes.
    if !(Self::MIN_SIZE..=Self::MAX_SIZE).contains(&length) {
      return Err(Error::Range);
    }

    Ok(Self(bytes.into()))
  }
}

#[derive(Debug, PartialEq, Clone, PartialOrd, Ord, Eq, Serialize, Deserialize)]
pub struct BRC20LowerCaseTicker(Box<[u8]>);

impl BRC20LowerCaseTicker {
  pub fn len(&self) -> usize {
    self.0.len()
  }
}

impl Display for BRC20LowerCaseTicker {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(f, "{}", std::str::from_utf8(&self.0).unwrap())
  }
}

#[derive(Debug, PartialEq, Clone, Deserialize, Serialize)]
pub enum Error {
  Range,
}

impl Display for Error {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    match self {
      Self::Range => write!(f, "ticker name out of range"),
    }
  }
}

impl std::error::Error for Error {}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_ticker_from_str_valid_bytes() {
    assert!(BRC20Ticker::from_str("BTC").is_err()); // length is less than MIN_SIZE
    assert!(BRC20Ticker::from_str("BITOIN").is_err()); // length is greater than MAX_SIZE

    assert_eq!(BRC20Ticker::from_str("ORDI").unwrap().to_string(), "ORDI"); // length is 4 bytes
    assert_eq!(BRC20Ticker::from_str("USDTS").unwrap().to_string(), "USDTS"); // length is 5 bytes
  }

  #[test]
  fn test_ticker_from_str_invalid_bytes() {
    assert_eq!(BRC20Ticker::from_str(""), Err(Error::Range));
    assert_eq!(BRC20Ticker::from_str("XAİİ"), Err(Error::Range));
  }

  #[test]
  fn test_ticker_from_str_valid() {
    // 4 bytes
    assert!(BRC20Ticker::from_str("XAİ").is_ok());
    assert!(BRC20Ticker::from_str("X。").is_ok());
    assert!(BRC20Ticker::from_str("aBc1").is_ok());
    assert!(BRC20Ticker::from_str("ατ").is_ok());
    assert!(BRC20Ticker::from_str("∑i").is_ok());
    assert!(BRC20Ticker::from_str("⊢i").is_ok());
    assert!(BRC20Ticker::from_str("≯a").is_ok());

    // 5 bytes
    assert!(BRC20Ticker::from_str("∑ii").is_ok());
    assert!(BRC20Ticker::from_str("⊢ii").is_ok());
    assert!(BRC20Ticker::from_str("a≯a").is_ok());
  }

  #[test]
  fn test_ticker_from_str_invalid() {
    assert_eq!(BRC20Ticker::from_str(""), Err(Error::Range));
    assert_eq!(BRC20Ticker::from_str("BTC"), Err(Error::Range));
    assert_eq!(BRC20Ticker::from_str("BITCOI"), Err(Error::Range));
    assert_eq!(BRC20Ticker::from_str("XAİİ"), Err(Error::Range));
  }

  #[test]
  fn test_ticker_to_lowercase() {
    assert_eq!(
      BRC20Ticker::from_str("aBc1")
        .unwrap()
        .to_lowercase()
        .to_string(),
      "abc1"
    );
    assert_eq!(
      BRC20Ticker::from_str("XAİ")
        .unwrap()
        .to_lowercase()
        .to_string(),
      "xai\u{307}"
    );
    assert_eq!(
      BRC20Ticker::from_str("ατ")
        .unwrap()
        .to_lowercase()
        .to_string(),
      "ατ"
    );
    assert_eq!(
      BRC20Ticker::from_str("∑H")
        .unwrap()
        .to_lowercase()
        .to_string(),
      "∑h"
    );
    assert_eq!(
      BRC20Ticker::from_str("⊢I")
        .unwrap()
        .to_lowercase()
        .to_string(),
      "⊢i"
    );
    assert_eq!(
      BRC20Ticker::from_str("≯A")
        .unwrap()
        .to_lowercase()
        .to_string(),
      "≯a"
    );
  }

  #[test]
  fn test_lowercase_ticker_display() {
    let lower = BRC20LowerCaseTicker(b"ordi".to_vec().into_boxed_slice());
    assert_eq!(format!("{}", lower), "ordi");

    let lower = BRC20LowerCaseTicker(b"sats".to_vec().into_boxed_slice());
    assert_eq!(format!("{}", lower), "sats");
  }

  #[test]
  fn test_ticker_display() {
    let ticker = BRC20Ticker::from_str("ORDI").unwrap();
    assert_eq!(format!("{}", ticker), "ORDI");

    let ticker = BRC20Ticker::from_str("SATS").unwrap();
    assert_eq!(format!("{}", ticker), "SATS");
  }

  #[test]
  fn test_ticker_len() {
    let ticker = BRC20Ticker::from_str("BTCD").unwrap();
    assert_eq!(ticker.len(), 4);

    let ticker = BRC20Ticker::from_str("USDTU").unwrap();
    assert_eq!(ticker.len(), 5);
  }

  #[test]
  fn test_ticker_equality() {
    let ticker1 = BRC20Ticker::from_str("BTCD").unwrap();
    let ticker2 = BRC20Ticker::from_str("BTCD").unwrap();
    let ticker3 = BRC20Ticker::from_str("USDT").unwrap();

    assert_eq!(ticker1, ticker2);
    assert_ne!(ticker1, ticker3);
  }

  #[test]
  fn test_ticker_ordering() {
    let ticker1 = BRC20Ticker::from_str("BTCD").unwrap();
    let ticker2 = BRC20Ticker::from_str("ETHI").unwrap();
    let ticker3 = BRC20Ticker::from_str("USDTT").unwrap();

    assert!(ticker1 < ticker2);
    assert!(ticker2 < ticker3);
  }

  #[test]
  fn test_error_display() {
    let ticker1 = BRC20Ticker::from_str("BTCDDD");
    assert_eq!(
      format!("{}", ticker1.err().unwrap()),
      "ticker name out of range"
    );
  }

  #[test]
  fn test_lowercase_ticker_from_bytes() {
    let lower = BRC20LowerCaseTicker(b"ordi".to_vec().into_boxed_slice());
    assert_eq!(lower.to_string(), "ordi");
    let lower = BRC20LowerCaseTicker("xai\u{307}".as_bytes().to_vec().into_boxed_slice());
    assert_eq!(lower.to_string(), "xai\u{307}");
  }

  #[test]
  fn test_tick_serialize() {
    let obj = BRC20Ticker::from_str("Ab1;").unwrap();
    let serialized = bincode::serialize(&obj).unwrap();
    assert_eq!(serialized, vec![4, 0, 0, 0, 0, 0, 0, 0, 65, 98, 49, 59]);

    let lower = obj.to_lowercase();
    let serialized = bincode::serialize(&lower).unwrap();
    assert_eq!(serialized, vec![4, 0, 0, 0, 0, 0, 0, 0, 97, 98, 49, 59]);
  }

  #[test]
  fn test_tick_deserialize() {
    let obj = BRC20Ticker::from_str("Ab1;").unwrap();
    let deserialized =
      bincode::deserialize::<BRC20Ticker>(&[4, 0, 0, 0, 0, 0, 0, 0, 65, 98, 49, 59]).unwrap();

    assert_eq!(deserialized, obj);

    // deserialize with error
    assert_eq!(
      bincode::deserialize::<BRC20Ticker>(&[6, 0, 0, 0, 0, 0, 0, 0, 65, 98, 49, 59, 47, 49])
        .unwrap_err()
        .to_string(),
      Error::Range.to_string()
    );
  }
}
