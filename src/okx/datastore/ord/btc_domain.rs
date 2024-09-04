use {super::*, anyhow::anyhow, regex::Regex};

const BTC_DOMAIN_KEY: &str = r"BTC_DOMAIN";

pub struct BtcDomain {
  pub name: String,
  pub domain: String,
}

const DEFAULT_DOMAIN_LIST: [&'static str; 4] = ["btc", "unisat", "sats", "x"];
impl BtcDomain {
  pub fn parse(bytes: &[u8], domain_list: &[String]) -> Result<Self> {
    let domains = if domain_list.is_empty() {
      DEFAULT_DOMAIN_LIST.join("|")
    } else {
      domain_list.join("|")
    };
    let pattern = format!(r"^(?<name>.+)\.(?<domain>{domains})$");
    let content = std::str::from_utf8(bytes)?;
    let re = Regex::new(&pattern).unwrap();
    if let Some(capture) = re.captures(content) {
      let name = &capture["name"];
      let domain = &capture["domain"];
      Ok(Self {
        name: name.to_string(),
        domain: domain.to_string(),
      })
    } else {
      Err(anyhow!("No match found."))
    }
  }

  pub fn to_collection_key(&self) -> String {
    format!("{}_{}_{}", BTC_DOMAIN_KEY, self.name, self.domain)
  }

  /// need image display if the domain name of "*.btc" is 6-digit
  pub fn btc_block_height(&self) -> Option<u32> {
    if self.name.len() == 6 && self.domain == "btc" {
      if let Ok(block_height) = self.name.parse::<u32>() {
        Some(block_height)
      } else {
        None
      }
    } else {
      None
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use bech32::ToBase32;

  #[test]
  fn validate_regex() {
    let domain_list = vec![];
    let district = BtcDomain::parse("0.bitmap".as_bytes(), &domain_list);
    assert!(district.is_err());

    let district = BtcDomain::parse("01.btc".as_bytes(), &domain_list).unwrap();
    assert_eq!(district.domain, "btc");
    assert_eq!(district.name, "01");
    assert_eq!(district.btc_block_height(), None);
    let district = BtcDomain::parse("btc.com/01?.btc.btc".as_bytes(), &domain_list).unwrap();
    assert_eq!(district.domain, "btc");
    assert_eq!(district.name, "btc.com/01?.btc");

    let district = BtcDomain::parse("123456.btc".as_bytes(), &domain_list).unwrap();
    assert_eq!(district.btc_block_height(), Some(123456));
    let district = BtcDomain::parse("100000.btc".as_bytes(), &domain_list).unwrap();
    assert_eq!(district.btc_block_height(), Some(100000));
    let district = BtcDomain::parse("000001.btc".as_bytes(), &domain_list).unwrap();
    assert_eq!(district.btc_block_height(), Some(1));

    let district = BtcDomain::parse("1234567.btc".as_bytes(), &domain_list).unwrap();
    assert_eq!(district.btc_block_height(), None);

    let district = BtcDomain::parse("abc.btc".as_bytes(), &domain_list).unwrap();
    assert_eq!(district.domain, "btc");
    assert_eq!(district.name, "abc");

    for d in DEFAULT_DOMAIN_LIST {
      let s = format!("abc.{d}");
      let district = BtcDomain::parse(s.as_bytes(), &domain_list).unwrap();
      assert!(DEFAULT_DOMAIN_LIST.contains(&district.domain.as_str()));
      assert_eq!(district.name, "abc");
    }
    // new domain list
    let domain_list = vec!["aaa".to_string(), "bbb".to_string()];
    let district = BtcDomain::parse("abc.aaa".as_bytes(), &domain_list).unwrap();
    assert_eq!(district.name, "abc");
    assert_eq!(district.domain, "aaa");

    let district = BtcDomain::parse("abc.btc".as_bytes(), &domain_list);
    assert!(district.is_err());
  }
}
