use super::*;
use serde::ser::SerializeTuple;

/// Represents an endpoint, which can be either a left or right endpoint.
#[derive(Debug, Clone, PartialEq)]
pub enum Endpoint<T> {
  Left(T),
  Right(T),
}

impl<T> Serialize for Endpoint<T>
where
  T: Serialize,
{
  /// Inserts a marker (0 or 1) between the value and the endpoint type during serialization.
  fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let mut seq = serializer.serialize_tuple(2)?;
    match self {
      Endpoint::Left(value) => {
        seq.serialize_element(value)?;
        seq.serialize_element(&0u8)?;
      }
      Endpoint::Right(value) => {
        seq.serialize_element(value)?;
        seq.serialize_element(&1u8)?;
      }
    }
    seq.end()
  }
}

impl<'de, T> Deserialize<'de> for Endpoint<T>
where
  T: Deserialize<'de>,
{
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    struct EndpointVisitor<T> {
      marker: std::marker::PhantomData<T>,
    }

    impl<'de, T> serde::de::Visitor<'de> for EndpointVisitor<T>
    where
      T: Deserialize<'de>,
    {
      type Value = Endpoint<T>;

      fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("a tuple representing an Endpoint")
      }

      fn visit_seq<V>(self, mut seq: V) -> Result<Endpoint<T>, V::Error>
      where
        V: serde::de::SeqAccess<'de>,
      {
        let value: T = seq
          .next_element()?
          .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
        let variant: u8 = seq
          .next_element()?
          .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;

        match variant {
          0 => Ok(Endpoint::Left(value)),
          1 => Ok(Endpoint::Right(value)),
          _ => Err(serde::de::Error::unknown_variant(
            &variant.to_string(),
            &["0", "1"],
          )),
        }
      }
    }

    deserializer.deserialize_tuple(
      2,
      EndpointVisitor {
        marker: std::marker::PhantomData,
      },
    )
  }
}

/// A composite key containing two values.
/// Used to quickly find related records through the first value.
#[derive(Debug, Clone, PartialEq)]
pub struct CompositeKey<T, U> {
  pub primary: T,
  pub secondary: U,
}

impl<T: Clone, U> CompositeKey<T, U> {
  pub fn primary_left_endpoint(&self) -> Endpoint<T> {
    Endpoint::Left(self.primary.clone())
  }

  pub fn primary_right_endpoint(&self) -> Endpoint<T> {
    Endpoint::Right(self.primary.clone())
  }
}

impl<T, U> Serialize for CompositeKey<T, U>
where
  T: Serialize,
  U: Serialize,
{
  /// Inserts a marker value `0` between `primary` and `secondary` during serialization.
  fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    let mut seq = serializer.serialize_tuple(3)?;
    seq.serialize_element(&self.primary)?;
    seq.serialize_element(&0u8)?;
    seq.serialize_element(&self.secondary)?;
    seq.end()
  }
}

impl<'de, T, U> Deserialize<'de> for CompositeKey<T, U>
where
  T: Deserialize<'de>,
  U: Deserialize<'de>,
{
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: Deserializer<'de>,
  {
    struct CompositeKeyVisitor<T, U> {
      marker: std::marker::PhantomData<(T, U)>,
    }

    impl<'de, T, U> serde::de::Visitor<'de> for CompositeKeyVisitor<T, U>
    where
      T: Deserialize<'de>,
      U: Deserialize<'de>,
    {
      type Value = CompositeKey<T, U>;

      fn expecting(&self, formatter: &mut Formatter) -> fmt::Result {
        formatter.write_str("a tuple representing a CompositeKey")
      }

      fn visit_seq<V>(self, mut seq: V) -> Result<CompositeKey<T, U>, V::Error>
      where
        V: serde::de::SeqAccess<'de>,
      {
        let primary: T = seq
          .next_element()?
          .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
        let _: u8 = seq
          .next_element()?
          .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
        let secondary: U = seq
          .next_element()?
          .ok_or_else(|| serde::de::Error::invalid_length(2, &self))?;
        Ok(CompositeKey { primary, secondary })
      }
    }

    deserializer.deserialize_tuple(
      3,
      CompositeKeyVisitor {
        marker: std::marker::PhantomData,
      },
    )
  }
}

pub type AddressTickerKey = CompositeKey<UtxoAddress, brc20::BRC20LowerCaseTicker>;
pub type AddressEndpoint = Endpoint<UtxoAddress>;

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_address_ticker_key() {
    let address = Address::from_str("bc1qhvd6suvqzjcu9pxjhrwhtrlj85ny3n2mqql5w4").unwrap();

    let address_ticker_key = AddressTickerKey {
      primary: UtxoAddress::from_script(
        address.assume_checked().script_pubkey().as_script(),
        &Chain::Mainnet,
      ),
      secondary: brc20::BRC20Ticker::from_str("value")
        .unwrap()
        .to_lowercase(),
    };

    let serialized = bincode::serialize(&address_ticker_key).unwrap();
    assert_eq!(
      serialized,
      vec![
        0, 0, 0, 0, 42, 0, 0, 0, 0, 0, 0, 0, 98, 99, 49, 113, 104, 118, 100, 54, 115, 117, 118,
        113, 122, 106, 99, 117, 57, 112, 120, 106, 104, 114, 119, 104, 116, 114, 108, 106, 56, 53,
        110, 121, 51, 110, 50, 109, 113, 113, 108, 53, 119, 52, 0, 5, 0, 0, 0, 0, 0, 0, 0, 118, 97,
        108, 117, 101
      ]
    );
    assert_eq!(
      bincode::deserialize::<AddressTickerKey>(serialized.as_slice()).unwrap(),
      address_ticker_key
    );
  }

  #[test]
  fn test_left_address_endpoint() {
    let address_endpoint = AddressEndpoint::Left(UtxoAddress::from_script(
      Address::from_str("bc1qhvd6suvqzjcu9pxjhrwhtrlj85ny3n2mqql5w4")
        .unwrap()
        .assume_checked()
        .script_pubkey()
        .as_script(),
      &Chain::Mainnet,
    ));

    let serialized = bincode::serialize(&address_endpoint).unwrap();
    assert_eq!(
      serialized,
      vec![
        0, 0, 0, 0, 42, 0, 0, 0, 0, 0, 0, 0, 98, 99, 49, 113, 104, 118, 100, 54, 115, 117, 118,
        113, 122, 106, 99, 117, 57, 112, 120, 106, 104, 114, 119, 104, 116, 114, 108, 106, 56, 53,
        110, 121, 51, 110, 50, 109, 113, 113, 108, 53, 119, 52, 0
      ]
    );

    assert_eq!(
      bincode::deserialize::<AddressEndpoint>(serialized.as_slice()).unwrap(),
      address_endpoint
    );
  }

  #[test]
  fn test_right_address_endpoint() {
    let address_endpoint = AddressEndpoint::Right(UtxoAddress::from_script(
      Address::from_str("bc1qhvd6suvqzjcu9pxjhrwhtrlj85ny3n2mqql5w4")
        .unwrap()
        .assume_checked()
        .script_pubkey()
        .as_script(),
      &Chain::Mainnet,
    ));

    let serialized = bincode::serialize(&address_endpoint).unwrap();
    assert_eq!(
      serialized,
      vec![
        0, 0, 0, 0, 42, 0, 0, 0, 0, 0, 0, 0, 98, 99, 49, 113, 104, 118, 100, 54, 115, 117, 118,
        113, 122, 106, 99, 117, 57, 112, 120, 106, 104, 114, 119, 104, 116, 114, 108, 106, 56, 53,
        110, 121, 51, 110, 50, 109, 113, 113, 108, 53, 119, 52, 1
      ]
    );

    assert_eq!(
      bincode::deserialize::<AddressEndpoint>(serialized.as_slice()).unwrap(),
      address_endpoint
    );
  }
}
