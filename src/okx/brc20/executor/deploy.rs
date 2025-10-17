use bitcoin::hashes::sha256;

use crate::okx::brc20::entry::BRC20Predeploy;

use super::*;
use std::u128;

const MINIMUM_PREDEPLOY_AGE: u32 = 3;

impl BRC20ExecutionMessage {
  pub(super) fn execute_deploy(
    &self,
    context: &mut TableContext,
    height: u32,
    blocktime: u32,
  ) -> Result<BRC20Receipt, ExecutionError> {
    let BRC20Operation::Deploy { deploy, parent } = &self.operation else {
      unreachable!()
    };

    // get the deployer address.
    let deployer = self.receiver.clone().unwrap();
    let ticker = BRC20Ticker::from_str(&deploy.tick).map_err(BRC20Error::TickerParse)?;

    // check if the ticker is not already deployed.
    if context.load_brc20_ticker_info(&ticker)?.is_some() {
      return Err(ExecutionError::ExecutionFailed(
        BRC20Error::DuplicateDeployment(ticker.to_string()),
      ));
    }

    // validate the predeploy if the ticker length is 6
    if ticker.len() == PREDEPLOYED_TICKER_LENGTH {
      let Some(parent) = parent else {
        return Err(ExecutionError::ExecutionFailed(
          BRC20Error::PredeployNotFound(ticker.to_string()),
        ));
      };
      // check if the ticker is predeployed properly
      let Some(predeploy) = context.load_brc20_predeploy(&parent)? else {
        return Err(ExecutionError::ExecutionFailed(
          BRC20Error::PredeployNotFound(ticker.to_string()),
        ));
      };

      validate_predeploy_hash(&predeploy, &deploy, &ticker, height)?;
    }

    // validate and parse the decimals.
    let decimals = if let Some(dec) = &deploy.decimals {
      let uncheck = FixedPoint::new_from_str(dec, 0).map_err(BRC20Error::NumericError)?;
      let (value, scale) = uncheck.to_u128_and_scale();
      if scale != 0 || value > u128::from(FixedPoint::MAX_SCALE) {
        return Err(ExecutionError::ExecutionFailed(
          BRC20Error::DecimalsExceedLimit(uncheck),
        ));
      } else {
        u8::try_from(value).unwrap()
      }
    } else {
      FixedPoint::MAX_SCALE
    };

    // validate and parse the max supply.
    let mut max =
      FixedPoint::new_from_str(&deploy.max_supply, decimals).map_err(BRC20Error::NumericError)?;
    if max > *MAXIMUM_SUPPLY {
      return Err(ExecutionError::ExecutionFailed(BRC20Error::InvalidSupply(
        max,
      )));
    }

    let self_minted = deploy.self_mint.unwrap_or_default();
    if max.is_zero() {
      if self_minted {
        max = FixedPoint::new_unchecked(
          u128::from(u64::MAX) * 10_u128.pow(decimals as u32),
          decimals,
        );
      } else {
        return Err(ExecutionError::ExecutionFailed(BRC20Error::InvalidSupply(
          max,
        )));
      }
    }

    // limit the mint amount.
    let limit = if let Some(lim) = &deploy.mint_limit {
      let limit = FixedPoint::new_from_str(lim, decimals).map_err(BRC20Error::NumericError)?;
      if limit.is_zero() || limit > *MAXIMUM_SUPPLY {
        return Err(ExecutionError::ExecutionFailed(
          BRC20Error::InvalidMaxMintLimit(limit),
        ));
      }
      limit
    } else {
      max
    };

    let total_supply = max.to_u128_and_scale().0;
    let max_mint_limit = limit.to_u128_and_scale().0;

    let ticker_info = BRC20TickerInfo {
      ticker: ticker.clone(),
      sequence_number: self.sequence_number,
      inscription_number: self.inscription_number,
      inscription_id: self.inscription_id,
      total_supply,
      burned: 0,
      minted: 0,
      max_mint_limit,
      decimals,
      deployer: deployer.clone(),
      self_minted,
      deployed_block_height: height,
      deployed_timestamp: blocktime,
      latest_minted_block_height: height,
    };

    // insert the ticker info to the table.
    context.update_brc20_ticker_info(&ticker, ticker_info)?;

    Ok(BRC20Receipt {
      inscription_id: self.inscription_id,
      sequence_number: self.sequence_number,
      inscription_number: self.inscription_number,
      old_satpoint: self.old_satpoint,
      new_satpoint: self.new_satpoint,
      sender: self.sender.clone(),
      receiver: deployer,
      op_type: BRC20OpType::Deploy,
      result: Ok(BRC20Event::Deploy(DeployEvent {
        ticker,
        total_supply,
        decimals,
        self_minted,
        max_mint_limit,
      })),
    })
  }
}

fn validate_predeploy_hash(
  predeploy: &BRC20Predeploy,
  deploy: &Deploy,
  ticker: &BRC20Ticker,
  height: u32,
) -> Result<(), ExecutionError> {
  if predeploy.block_height.saturating_add(MINIMUM_PREDEPLOY_AGE) > height {
    return Err(ExecutionError::ExecutionFailed(
      BRC20Error::PredeployTooYoung(ticker.to_string(), predeploy.block_height),
    ));
  }

  let Some(salt) = &deploy.salt else {
    return Err(ExecutionError::ExecutionFailed(BRC20Error::SaltNotFound(
      ticker.to_string(),
    )));
  };

  let Ok(salt_bytes) = hex::decode(salt) else {
    return Err(ExecutionError::ExecutionFailed(BRC20Error::SaltInvalidHex(
      salt.clone(),
    )));
  };

  let salted_ticker = [
    ticker.as_bytes(),
    &salt_bytes,
    &predeploy.predeployer.to_script_bytes(),
  ]
  .concat();

  if sha256::Hash::from_slice(&predeploy.hash)
    != Ok(sha256::Hash::hash(
      sha256::Hash::hash(&salted_ticker)
        .to_byte_array()
        .as_slice(),
    ))
  {
    return Err(ExecutionError::ExecutionFailed(
      BRC20Error::PredeployHashInvalid(ticker.to_string()),
    ));
  }

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_validate_predeploy_hash_ok() {
    let predeploy = BRC20Predeploy {
      hash: sha256::Hash::from_str(
        "4245293d3aa93d79f79554f0bee7565d7da6f19f4025b26c9d0440dba9eade10",
      )
      .unwrap()
      .as_byte_array()
      .as_slice()
      .try_into()
      .unwrap(),
      predeployer: UtxoAddress::from_str(
        "bc1qhqexvv8f4rqgwyk8yl60xrgn75ams53v4a5pm9",
        Network::Bitcoin,
      )
      .unwrap(),
      block_height: 100,
    };

    let deploy = Deploy {
      tick: "EXAMPL".to_string(),
      max_supply: "1000000".to_string(),
      decimals: Some("2".to_string()),
      self_mint: Some(false),
      mint_limit: Some("500000".to_string()),
      salt: Some(hex::encode("okxsalt".as_bytes())),
    };

    let ticker = BRC20Ticker::from_str("EXAMPL").unwrap();

    let result = validate_predeploy_hash(&predeploy, &deploy, &ticker, 104);
    assert!(result.is_ok());
  }

  #[test]
  fn test_validate_predeploy_hash_too_soon() {
    let predeploy = BRC20Predeploy {
      hash: sha256::Hash::from_str(
        "4245293d3aa93d79f79554f0bee7565d7da6f19f4025b26c9d0440dba9eade10",
      )
      .unwrap()
      .as_byte_array()
      .as_slice()
      .try_into()
      .unwrap(),
      predeployer: UtxoAddress::from_str(
        "bc1qhqexvv8f4rqgwyk8yl60xrgn75ams53v4a5pm9",
        Network::Bitcoin,
      )
      .unwrap(),
      block_height: 100,
    };

    let deploy = Deploy {
      tick: "EXAMPL".to_string(),
      max_supply: "1000000".to_string(),
      decimals: Some("2".to_string()),
      self_mint: Some(false),
      mint_limit: Some("500000".to_string()),
      salt: Some(hex::encode("okxsalt".as_bytes())),
    };

    let ticker = BRC20Ticker::from_str("EXAMPL").unwrap();

    let result = validate_predeploy_hash(&predeploy, &deploy, &ticker, 102);
    assert!(result.is_err());
  }

  #[test]
  fn test_validate_predeploy_hash_err() {
    let predeploy = BRC20Predeploy {
      hash: sha256::Hash::from_str(
        "3a6eb0794f5f5e8e2c3f4b5a6d7e8f90123456789abcdef0123456789abcdef0",
      )
      .unwrap()
      .as_byte_array()
      .as_slice()
      .try_into()
      .unwrap(),
      predeployer: UtxoAddress::from_str(
        "bc1qxy2kgdygjrsqtzq2n0yrf2493p83kkfjhx0wlh",
        Network::Bitcoin,
      )
      .unwrap(),
      block_height: 100,
    };

    let deploy = Deploy {
      tick: "EXAMPL".to_string(),
      max_supply: "1000000".to_string(),
      decimals: Some("2".to_string()),
      self_mint: Some(false),
      mint_limit: Some("500000".to_string()),
      salt: Some("abcdef1234567890".to_string()),
    };

    let ticker = BRC20Ticker::from_str("EXAMPL").unwrap();

    let result = validate_predeploy_hash(&predeploy, &deploy, &ticker, 104);
    assert!(result.is_err());
  }
}
