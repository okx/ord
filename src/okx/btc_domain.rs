use super::{entry::CollectionType, *};

#[derive(Debug, Clone, PartialEq)]
enum TopDomain {
  BTC,
  Unisat,
  Sats,
  X,
}

impl TopDomain {
  pub const ALL: [Self; 4] = [Self::BTC, Self::Unisat, Self::Sats, Self::X];
}

impl Display for TopDomain {
  fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
    match self {
      TopDomain::BTC => write!(f, "btc"),
      TopDomain::Unisat => write!(f, "unisat"),
      TopDomain::Sats => write!(f, "sats"),
      TopDomain::X => write!(f, "x"),
    }
  }
}

impl Into<CollectionType> for TopDomain {
  fn into(self) -> CollectionType {
    match self {
      TopDomain::BTC => CollectionType::BtcName,
      TopDomain::Unisat => CollectionType::UnisatName,
      TopDomain::Sats => CollectionType::SatsName,
      TopDomain::X => CollectionType::XName,
    }
  }
}

impl TryFrom<&str> for TopDomain {
  type Error = ();

  fn try_from(value: &str) -> Result<Self, Self::Error> {
    match value {
      "btc" => Ok(Self::BTC),
      "unisat" => Ok(Self::Unisat),
      "sats" => Ok(Self::Sats),
      "x" => Ok(Self::X),
      _ => Err(()),
    }
  }
}

#[derive(Debug, Clone)]
pub struct BtcDomain {
  name: String,
  domain: TopDomain,
}

impl Display for BtcDomain {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    write!(f, "{}.{}", self.name, self.domain)
  }
}

pub trait BTCDomainExtractor {
  fn extract_btc_domain(&self) -> Option<BtcDomain>;
}

impl BtcDomain {
  /// check the name is valid or not
  /// https://docs.btcname.id/docs/overview/chapter-4-thinking-about-.btc-domain-name/calibration-rules
  fn is_name_valid(name: &str) -> bool {
    let pattern = r"[\.\n ]";
    let re = Regex::new(pattern).unwrap();
    if re.captures(name).is_some() {
      return false;
    }
    // check if it's json format
    if name.contains("{") {
      let value: Result<serde_json::Value, _> = serde_json::from_str(name);
      return value.is_err();
    }
    true
  }
}

impl BTCDomainExtractor for Inscription {
  fn extract_btc_domain(&self) -> Option<BtcDomain> {
    let Some(content) = self.body() else {
      return None;
    };

    let domains = TopDomain::ALL
      .iter()
      .map(ToString::to_string)
      .collect::<Vec<_>>()
      .join("|");
    let pattern = format!(r"^(?<name>.+)\.(?<domain>{domains})$");

    let re = Regex::new(&pattern).unwrap();

    let content = std::str::from_utf8(content).ok()?;

    if let Some(capture) = re.captures(&content.to_lowercase()) {
      let name = &capture["name"];
      let domain = &capture["domain"];
      if BtcDomain::is_name_valid(name) {
        return Some(BtcDomain {
          name: name.to_string(),
          domain: domain.try_into().unwrap(),
        });
      }
    }
    None
  }
}

impl BtcDomain {
  pub(crate) fn execute(
    &self,
    context: &mut TableContext,
    sequence_number: u32,
    inscription_id: InscriptionId,
  ) -> Result<()> {
    let full_domain = self.to_string();
    if context
      .load_btc_domain_to_sequence_number(&full_domain)?
      .is_some()
    {
      log::debug!(
        "BtcDomain::execute: btc domain {} already exists, inscription_id = {}",
        self,
        inscription_id
      );
      return Ok(());
    }

    context.insert_btc_domain_to_sequence_number(&full_domain, sequence_number)?;

    context
      .insert_sequence_number_to_collection_type(sequence_number, self.domain.clone().into())?;
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn validate_regex() {
    let invalid_domains = [
      "abc.bitmap",
      "btc.com.btc",
      "hi.jack.btc",
      " jack.btc",
      "jack.btc ",
      "hi jack.btc",
      " jack.btc ",
      "jack.btc\n",
      "\njack.btc",
      "hi\njack.btc",
      "\njack.btc\n",
      "abc.aaa",
      r#"{ "p":"sns", "op":"reg",    "name":"jack.btc"}"#,
    ];
    for domain in invalid_domains {
      let inscription = Inscription {
        body: Some(domain.as_bytes().into()),
        ..default()
      };
      let btc_name = inscription.extract_btc_domain();
      assert!(btc_name.is_none());
    }

    let valid_domains = [
      "01.btc",
      "123456.btc",
      "Jack.btc",
      "JACK.BTC",
      "jack.BtC",
      "比特币.btc",
      "😀.btc",
      "\\jack.btc",
      "\tjack.btc",
    ];
    for domain in valid_domains {
      let inscription = Inscription {
        body: Some(domain.as_bytes().into()),
        ..default()
      };
      let btc_name = inscription.extract_btc_domain();
      assert_eq!(btc_name.unwrap().domain, TopDomain::BTC);
    }

    // test "unisat", "sats", "x"
    assert_eq!(
      Inscription {
        body: Some("abcdef.unisat".as_bytes().into()),
        ..default()
      }
      .extract_btc_domain()
      .unwrap()
      .domain,
      TopDomain::Unisat
    );

    assert_eq!(
      Inscription {
        body: Some("abcdef.sats".as_bytes().into()),
        ..default()
      }
      .extract_btc_domain()
      .unwrap()
      .domain,
      TopDomain::Sats
    );

    assert_eq!(
      Inscription {
        body: Some("abcdef.x".as_bytes().into()),
        ..default()
      }
      .extract_btc_domain()
      .unwrap()
      .domain,
      TopDomain::X
    );

    for d in TopDomain::ALL {
      let inscription = Inscription {
        body: Some(format!("abc.{d}").as_bytes().into()),
        ..default()
      };

      let btc_domain = inscription.extract_btc_domain().unwrap();
      assert_eq!(btc_domain.name, "abc");
      assert_eq!(btc_domain.domain, d);
    }
  }
}
