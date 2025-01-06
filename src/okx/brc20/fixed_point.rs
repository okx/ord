use bigdecimal::{num_bigint::BigInt, BigDecimal, Pow, ToPrimitive};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{
  fmt,
  fmt::{Display, Formatter},
  ops::{Add, AddAssign, Sub, SubAssign},
  str::FromStr,
};

#[derive(Debug, Clone, Default, Eq, PartialEq, Copy)]
pub struct FixedPoint {
  value: u128,
  scale: u8,
}

impl FixedPoint {
  pub const MAX_SCALE: u8 = 18;

  #[cfg(test)]
  const MAX: Self = Self {
    value: u128::MAX,
    scale: 0,
  };

  #[cfg(test)]
  const MIN: Self = Self { value: 0, scale: 0 };

  pub fn new(value: u128, scale: u8) -> Result<Self, NumParseError> {
    if scale > Self::MAX_SCALE {
      return Err(NumParseError::OutOfMaxScale(i64::from(scale)));
    }
    Ok(Self { value, scale })
  }

  pub fn new_unchecked(value: u128, scale: u8) -> Self {
    debug_assert!(scale <= Self::MAX_SCALE, "Scale exceeds MAX_SCALE");
    Self { value, scale }
  }

  pub fn new_from_str(s: &str, scale: u8) -> Result<Self, NumParseError> {
    static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^\d+(\.\d+)?$").unwrap());
    if !RE.is_match(s) {
      return Err(NumParseError::InvalidFormat(s.to_string()));
    }

    let num = BigDecimal::from_str(s).map_err(|_| NumParseError::InvalidNumber(s.to_string()))?;
    let (mut bigint, b_scale) = num.into_bigint_and_scale();
    let target_scale = i64::from(scale);
    if b_scale > target_scale {
      return Err(NumParseError::OutOfMaxScale(target_scale));
    }

    if target_scale - b_scale > 0 {
      bigint *= BigInt::from(10).pow((target_scale - b_scale) as u64);
    }

    let value = bigint
      .to_u128()
      .ok_or(NumParseError::OutOfRange(s.to_string()))?;

    Ok(Self { value, scale })
  }

  pub fn is_zero(&self) -> bool {
    self.value == 0
  }

  pub fn checked_add(self, rhs: Self) -> Option<Self> {
    assert_eq!(
      self.scale, rhs.scale,
      "Scales must be the same for addition"
    );
    Some(Self {
      value: self.value.checked_add(rhs.value)?,
      scale: self.scale,
    })
  }

  pub fn checked_sub(self, rhs: Self) -> Option<Self> {
    assert_eq!(
      self.scale, rhs.scale,
      "Scales must be the same for subtraction"
    );
    Some(Self {
      value: self.value.checked_sub(rhs.value)?,
      scale: self.scale,
    })
  }

  pub fn to_u128_and_scale(self) -> (u128, u8) {
    (self.value, self.scale)
  }

  pub fn to_big_decimal(self) -> BigDecimal {
    BigDecimal::new(BigInt::from(self.value), i64::from(self.scale))
  }
}

impl PartialOrd for FixedPoint {
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    self.to_big_decimal().partial_cmp(&other.to_big_decimal())
  }
}

impl Ord for FixedPoint {
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    self.to_big_decimal().cmp(&other.to_big_decimal())
  }
}

impl Add for FixedPoint {
  type Output = Self;
  fn add(self, other: Self) -> Self::Output {
    self.checked_add(other).expect("num overflow")
  }
}

impl AddAssign for FixedPoint {
  fn add_assign(&mut self, other: Self) {
    *self = *self + other;
  }
}

impl Sub for FixedPoint {
  type Output = Self;
  fn sub(self, other: Self) -> Self::Output {
    self.checked_sub(other).expect("num underflow")
  }
}

impl SubAssign for FixedPoint {
  fn sub_assign(&mut self, other: Self) {
    *self = *self - other;
  }
}

impl Display for FixedPoint {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    self.to_big_decimal().fmt(f)
  }
}

#[derive(Debug, Clone, PartialEq, thiserror::Error, Deserialize, Serialize)]
pub enum NumParseError {
  #[error("Invalid format: '{0}'.")]
  InvalidFormat(String),

  #[error("Cannot parse: '{0}'.")]
  InvalidNumber(String),

  #[error("Precision exceeds max: {0}.")]
  OutOfMaxScale(i64),

  #[error("Out of range: '{0}'.")]
  OutOfRange(String),
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::assert_matches;
  #[test]
  fn test_valid_fixed_point_from_str() {
    assert_eq!(
      FixedPoint::new_from_str("0", 0).unwrap(),
      FixedPoint { value: 0, scale: 0 }
    );
    assert_eq!(
      FixedPoint::new_from_str("0.0", 1).unwrap(),
      FixedPoint { value: 0, scale: 1 }
    );
    assert_eq!(
      FixedPoint::new_from_str("0.0", 18).unwrap(),
      FixedPoint {
        value: 0,
        scale: 18
      }
    );
    assert_eq!(
      FixedPoint::new_from_str("0.000000000000000000", 18).unwrap(),
      FixedPoint {
        value: 0,
        scale: 18
      }
    );
    assert_eq!(
      FixedPoint::new_from_str("0.000000000000000001", 18).unwrap(),
      FixedPoint {
        value: 1,
        scale: 18
      }
    );
    assert_eq!(
      FixedPoint::new_from_str("0.00000000000000001", 17).unwrap(),
      FixedPoint {
        value: 1,
        scale: 17
      }
    );
    assert_eq!(
      FixedPoint::new_from_str("0.00000000000000001", 18).unwrap(),
      FixedPoint {
        value: 10,
        scale: 18
      }
    );
    assert_eq!(
      FixedPoint::new_from_str("340282366920938463463.374607431768211455", 18).unwrap(),
      FixedPoint {
        value: u128::MAX,
        scale: 18
      }
    );

    assert_eq!(
      FixedPoint::new_from_str("001", 0).unwrap(),
      FixedPoint { value: 1, scale: 0 }
    );
    assert_eq!(
      FixedPoint::new_from_str("00.1", 1).unwrap(),
      FixedPoint { value: 1, scale: 1 }
    );
    assert_eq!(
      FixedPoint::new_from_str("0.100", 3).unwrap(),
      FixedPoint {
        value: 100,
        scale: 3
      }
    );
    assert_eq!(
      FixedPoint::new_from_str("00.00100", 5).unwrap(),
      FixedPoint {
        value: 100,
        scale: 5
      }
    );
    assert_eq!(
      FixedPoint::new_from_str("1.1", 1).unwrap(),
      FixedPoint {
        value: 11,
        scale: 1
      }
    );

    assert_eq!(
      FixedPoint::new_from_str("1.1000", 4).unwrap(),
      FixedPoint {
        value: 11000,
        scale: 4
      }
    );
    assert_eq!(
      FixedPoint::new_from_str("1.01", 2).unwrap(),
      FixedPoint {
        value: 101,
        scale: 2
      }
    );
    assert_eq!(
      FixedPoint::new_from_str("1.000000000000000001", 18).unwrap(),
      FixedPoint {
        value: 1_000_000_000_000_000_001,
        scale: 18
      }
    );
    assert_eq!(
      FixedPoint::new_from_str("340282366920938463463.000000000000000001", 18).unwrap(),
      FixedPoint {
        value: 340_282_366_920_938_463_463_000_000_000_000_000_001,
        scale: 18
      }
    );
  }

  #[test]
  fn test_fixed_point_invalid_format() {
    assert_matches!(
      FixedPoint::new_from_str(".", 0),
      Err(NumParseError::InvalidFormat(_))
    );
    assert_matches!(
      FixedPoint::new_from_str(".456", 0),
      Err(NumParseError::InvalidFormat(_))
    );
    assert_matches!(
      FixedPoint::new_from_str(".456 ", 0),
      Err(NumParseError::InvalidFormat(_))
    );

    assert_matches!(
      FixedPoint::new_from_str("+123.456", 0),
      Err(NumParseError::InvalidFormat(_))
    );
    // can not be negative
    assert_matches!(
      FixedPoint::new_from_str("-1.1", 0),
      Err(NumParseError::InvalidFormat(_))
    );
    assert_matches!(
      FixedPoint::new_from_str("1e10", 0),
      Err(NumParseError::InvalidFormat(_))
    );
    assert_matches!(
      FixedPoint::new_from_str("1E10", 0),
      Err(NumParseError::InvalidFormat(_))
    );
    assert_matches!(
      FixedPoint::new_from_str("123.-456", 0),
      Err(NumParseError::InvalidFormat(_))
    );
    assert_matches!(
      FixedPoint::new_from_str("123.+456", 0),
      Err(NumParseError::InvalidFormat(_))
    );
    assert_matches!(
      FixedPoint::new_from_str("123456789.", 0),
      Err(NumParseError::InvalidFormat(_))
    );
    assert_matches!(
      FixedPoint::new_from_str(".1", 0),
      Err(NumParseError::InvalidFormat(_))
    );

    assert_matches!(
      FixedPoint::new_from_str("", 18),
      Err(NumParseError::InvalidFormat(_))
    );
    assert_matches!(
      FixedPoint::new_from_str(" ", 18),
      Err(NumParseError::InvalidFormat(_))
    );
    assert_matches!(
      FixedPoint::new_from_str(" 123.456", 18),
      Err(NumParseError::InvalidFormat(_))
    );

    assert_matches!(
      FixedPoint::new_from_str(" .456 ", 18),
      Err(NumParseError::InvalidFormat(_))
    );
    assert_matches!(
      FixedPoint::new_from_str(" 456", 18),
      Err(NumParseError::InvalidFormat(_))
    );
    assert_matches!(
      FixedPoint::new_from_str("456 ", 18),
      Err(NumParseError::InvalidFormat(_))
    );
    assert_matches!(
      FixedPoint::new_from_str("45 6", 18),
      Err(NumParseError::InvalidFormat(_))
    );
    assert_matches!(
      FixedPoint::new_from_str("123. 456", 18),
      Err(NumParseError::InvalidFormat(_))
    );
    assert_matches!(
      FixedPoint::new_from_str("123.456.789", 18),
      Err(NumParseError::InvalidFormat(_))
    );
  }

  #[test]
  fn test_invalid_fixed_point() {
    assert_matches!(
      FixedPoint::new_from_str("0.0", 0),
      Err(NumParseError::OutOfMaxScale(0))
    );
    assert_matches!(
      FixedPoint::new_from_str("0.001", 2),
      Err(NumParseError::OutOfMaxScale(2))
    );
    // number of decimal fractional can not exceed 18
    assert_matches!(
      FixedPoint::new_from_str("123456789.12345678901234567891", 18),
      Err(NumParseError::OutOfMaxScale(18))
    );
    assert_matches!(
      FixedPoint::new_from_str("1.0000000000000000001", 18),
      Err(NumParseError::OutOfMaxScale(18))
    );

    assert_matches!(
      FixedPoint::new_from_str("340282366920938463463.374607431768211456", 18),
      Err(NumParseError::OutOfRange(_))
    );
    assert_matches!(
      FixedPoint::new_from_str("340282366920938463464.374607431768211455", 18),
      Err(NumParseError::OutOfRange(_))
    );
  }
  #[test]
  fn to_big_decimal() {
    assert_eq!(
      FixedPoint { value: 0, scale: 0 }.to_big_decimal(),
      BigDecimal::from(0)
    );
    assert_eq!(
      FixedPoint { value: 0, scale: 1 }.to_big_decimal(),
      BigDecimal::from(0)
    );
    assert_eq!(
      FixedPoint {
        value: 0,
        scale: 18
      }
      .to_big_decimal(),
      BigDecimal::from(0)
    );
    assert_eq!(
      FixedPoint { value: 1, scale: 0 }.to_big_decimal(),
      BigDecimal::from(1)
    );
    assert_eq!(
      FixedPoint {
        value: 100,
        scale: 2
      }
      .to_big_decimal(),
      BigDecimal::from(1)
    );
    assert_eq!(
      FixedPoint { value: 1, scale: 1 }.to_big_decimal(),
      BigDecimal::from_str("0.1").unwrap()
    );
    assert_eq!(
      FixedPoint {
        value: 1,
        scale: 18
      }
      .to_big_decimal(),
      BigDecimal::from_str("0.000000000000000001").unwrap()
    );
    assert_eq!(
      FixedPoint {
        value: 1234567890123456789,
        scale: 18
      }
      .to_big_decimal(),
      BigDecimal::from_str("1.234567890123456789").unwrap()
    );
  }

  #[test]
  #[should_panic(expected = "num overflow")]
  fn add() {
    let _ = FixedPoint::MAX + FixedPoint { value: 1, scale: 0 };
  }

  #[test]
  #[should_panic(expected = "num overflow")]
  fn add_assign() {
    let mut l = FixedPoint::MAX;
    l += FixedPoint { value: 1, scale: 0 }
  }

  #[test]
  #[should_panic(expected = "num underflow")]
  fn sub() {
    let _ = FixedPoint::MIN - FixedPoint { value: 1, scale: 0 };
  }

  #[test]
  #[should_panic(expected = "num underflow")]
  fn sub_assign() {
    let mut l = FixedPoint::MIN;
    l -= FixedPoint { value: 1, scale: 0 };
  }
}
