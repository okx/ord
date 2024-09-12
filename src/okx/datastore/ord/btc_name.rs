use {super::*, anyhow::anyhow, regex::Regex};

const BTC_DOMAIN_KEY: &str = r"BTC_NAME";

pub struct BtcName {
  pub name: String,
  pub domain: String,
}

const DEFAULT_DOMAIN_LIST: [&str; 4] = ["btc", "unisat", "sats", "x"];
impl BtcName {
  pub fn parse(bytes: &[u8], domain_list: &[String]) -> Result<Self> {
    let domains = if domain_list.is_empty() {
      DEFAULT_DOMAIN_LIST.join("|")
    } else {
      domain_list.join("|")
    };
    let pattern = format!(r"^(?<name>.+)\.(?<domain>{domains})$");
    let content = std::str::from_utf8(bytes)?;
    let re = Regex::new(&pattern).unwrap();
    if let Some(capture) = re.captures(&content.to_lowercase()) {
      let name = &capture["name"];
      let domain = &capture["domain"];
      if Self::is_name_valid(name) {
        return Ok(Self {
          name: name.to_string(),
          domain: domain.to_string(),
        });
      }
    }
    Err(anyhow!("No match found."))
  }

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

  pub fn to_collection_key(&self) -> String {
    format!("{}_{}_{}", BTC_DOMAIN_KEY, self.name, self.domain)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn validate_regex() {
    let domain_list = vec![];
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
      r#"{ "p":"sns", "op":"reg",    "name":"jack.btc"}"#,
    ];
    for domain in invalid_domains {
      let btc_name = BtcName::parse(domain.as_bytes(), &domain_list);
      assert!(btc_name.is_err());
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
      let btc_name = BtcName::parse(domain.as_bytes(), &domain_list);
      assert!(btc_name.is_ok());
    }

    for d in DEFAULT_DOMAIN_LIST {
      let s = format!("abc.{d}");
      let btc_name = BtcName::parse(s.as_bytes(), &domain_list).unwrap();
      assert!(DEFAULT_DOMAIN_LIST.contains(&btc_name.domain.as_str()));
      assert_eq!(btc_name.name, "abc");
    }
    // new domain list
    let domain_list = vec!["aaa".to_string(), "bbb".to_string()];
    let btc_name = BtcName::parse("abc.aaa".as_bytes(), &domain_list).unwrap();
    assert_eq!(btc_name.name, "abc");
    assert_eq!(btc_name.domain, "aaa");
  }
}
